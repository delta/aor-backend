use self::util::{DefenderTypeResponse, MineTypeResponse};

use super::attack::util::get_game_id_from_redis;
use super::auth::session::AuthUser;
use super::inventory::util::get_user_artifacts;
use super::user::util::fetch_user;
use super::PgPool;
use super::RedisPool;
use crate::api::error;
use crate::api::util::HistoryboardQuery;
use crate::models::*;
use actix_web::error::{ErrorBadRequest, ErrorNotFound};
use actix_web::web::{self, Data, Json};
use actix_web::{Responder, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod shortest_path;
pub mod util;
mod validate;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("")
            .route(web::put().to(set_base_details))
            .route(web::get().to(get_user_base_details)),
    )
    .service(web::resource("/top").route(web::get().to(get_top_defenses)))
    .service(web::resource("/transfer").route(web::post().to(post_transfer_artifacts)))
    .service(web::resource("/batch_transfer").route(web::post().to(post_batch_transfer_artifacts)))
    .service(web::resource("/save").route(web::put().to(confirm_base_details)))
    .service(web::resource("/game/{id}").route(web::get().to(get_game_base_details)))
    .service(web::resource("/history").route(web::get().to(defense_history)))
    .service(web::resource("/{defender_id}").route(web::get().to(get_other_base_details)))
    .app_data(Data::new(web::JsonConfig::default().limit(1024 * 1024)));
}

#[derive(Deserialize)]
pub struct MapSpacesEntry {
    pub x_coordinate: i32,
    pub y_coordinate: i32,
    pub block_type_id: i32,
    pub artifacts: i32,
}

#[derive(Deserialize)]
pub struct TransferArtifactEntry {
    pub artifacts_differ: i32,
    pub map_space_id: i32,
}

#[derive(Serialize)]
pub struct TransferArtifactResponse {
    pub building_map_space_id: i32,
    pub artifacts_in_building: i32,
    pub bank_map_space_id: i32,
    pub artifacts_in_bank: i32,
}

#[derive(Deserialize)]
pub struct BatchTransferArtifacts {
    pub transfers: Vec<TransferArtifactEntry>,
}

async fn post_transfer_artifacts(
    transfer: Json<TransferArtifactEntry>,
    pg_pool: Data<PgPool>,
    redis_pool: Data<RedisPool>,
    user: AuthUser,
) -> Result<impl Responder> {
    let user_id = user.0;

    let mut redis_conn = redis_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;

    if let Ok(Some(_)) = get_game_id_from_redis(user_id, &mut redis_conn, false) {
        return Err(ErrorBadRequest(
            "You are under attack. Cannot transfer artifacts",
        ));
    }

    let transfer = transfer.into_inner();

    let mut conn = pg_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;
    let bank_block_type_id = web::block(move || util::get_block_id_of_bank(&mut conn, &user_id))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pg_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;
    let current_layout_id =
        web::block(move || util::check_valid_map_id(&mut conn, &user_id, &transfer.map_space_id))
            .await?
            .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pg_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;
    let is_valid_map_space_building =
        web::block(move || util::check_valid_map_space_building(&mut conn, &transfer.map_space_id))
            .await?
            .map_err(|err| error::handle_error(err.into()))?;

    if !is_valid_map_space_building {
        return Err(ErrorBadRequest(
            "Map Space ID does not correspond to a valid building",
        ));
    }

    let mut conn = pg_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;
    let bank_map_space_id = web::block(move || {
        util::get_bank_map_space_id(&mut conn, &current_layout_id, &bank_block_type_id)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    if bank_map_space_id == transfer.map_space_id {
        return Err(ErrorBadRequest("Cannot transfer to the same building"));
    }

    let mut conn = pg_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;
    let bank_artifact_count = web::block(move || {
        util::get_building_artifact_count(&mut conn, &current_layout_id, &bank_map_space_id)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    if transfer.artifacts_differ > bank_artifact_count {
        return Err(ErrorBadRequest("Not enough artifacts in the bank"));
    }

    let mut conn = pg_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;
    let mut building_artifact_count = web::block(move || {
        util::get_building_artifact_count(&mut conn, &current_layout_id, &transfer.map_space_id)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    if building_artifact_count == -1 {
        let mut conn = pg_pool
            .get()
            .map_err(|err| error::handle_error(err.into()))?;
        web::block(move || util::create_artifact_record(&mut conn, &transfer.map_space_id, &0))
            .await?
            .map_err(|err| error::handle_error(err.into()))?;
        building_artifact_count = 0;
    }

    if transfer.artifacts_differ + building_artifact_count < 0 {
        return Err(ErrorBadRequest("Not enough artifacts in the building"));
    }

    let mut conn = pg_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;
    let building_capacity =
        web::block(move || util::get_building_capacity(&mut conn, &transfer.map_space_id))
            .await?
            .map_err(|err| error::handle_error(err.into()))?;

    if building_capacity < transfer.artifacts_differ + building_artifact_count {
        return Err(ErrorBadRequest("Building capacity not sufficient"));
    }

    let new_building_artifact_count = building_artifact_count + transfer.artifacts_differ;
    let new_bank_artifact_count = bank_artifact_count - transfer.artifacts_differ;

    //Transfer Artifacts
    let mut conn = pg_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;
    web::block(move || {
        util::transfer_artifacts_building(
            &mut conn,
            &transfer.map_space_id,
            &bank_map_space_id,
            &new_building_artifact_count,
            &new_bank_artifact_count,
        )
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    Ok(web::Json(TransferArtifactResponse {
        building_map_space_id: transfer.map_space_id,
        artifacts_in_building: new_building_artifact_count,
        bank_map_space_id,
        artifacts_in_bank: new_bank_artifact_count,
    }))
}

async fn post_batch_transfer_artifacts(
    batch_transfer: Json<BatchTransferArtifacts>,
    pg_pool: Data<PgPool>,
    redis_pool: Data<RedisPool>,
    user: AuthUser,
) -> Result<impl Responder> {
    let user_id = user.0;
    let mut redis_conn = redis_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;

    if let Ok(Some(_)) = get_game_id_from_redis(user_id, &mut redis_conn, false) {
        return Err(ErrorBadRequest(
            "You are under attack. Cannot transfer artifacts",
        ));
    }

    let transfers = batch_transfer.into_inner().transfers;
    let total_artifact_differ: i32 = transfers.iter().map(|transfer| transfer.artifacts_differ).sum();    
    let mut responses = Vec::new();
    let mut accum_val: i32 = 0;

    for transfer in transfers {
        let mut conn = pg_pool
            .get()
            .map_err(|err| error::handle_error(err.into()))?;
        let bank_block_type_id =
            web::block(move || util::get_block_id_of_bank(&mut conn, &user_id))
                .await?
                .map_err(|err| error::handle_error(err.into()))?;

        let mut conn = pg_pool
            .get()
            .map_err(|err| error::handle_error(err.into()))?;
        let current_layout_id = web::block(move || {
            util::check_valid_map_id(&mut conn, &user_id, &transfer.map_space_id)
        })
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

        let mut conn = pg_pool
            .get()
            .map_err(|err| error::handle_error(err.into()))?;
        let is_valid_map_space_building = web::block(move || {
            util::check_valid_map_space_building(&mut conn, &transfer.map_space_id)
        })
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

        if !is_valid_map_space_building {
            return Err(ErrorBadRequest(
                "Map Space ID does not correspond to a valid building",
            ));
        }

        let mut conn = pg_pool
            .get()
            .map_err(|err| error::handle_error(err.into()))?;
        let bank_map_space_id = web::block(move || {
            util::get_bank_map_space_id(&mut conn, &current_layout_id, &bank_block_type_id)
        })
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

        if bank_map_space_id == transfer.map_space_id {
            return Err(ErrorBadRequest("Cannot transfer to the same building"));
        }

        let mut conn = pg_pool
            .get()
            .map_err(|err| error::handle_error(err.into()))?;
        let bank_artifact_count = web::block(move || {
            util::get_building_artifact_count(&mut conn, &current_layout_id, &bank_map_space_id)
        })
        .await?
        .map_err(|err| error::handle_error(err.into()))?;
        
        if total_artifact_differ > bank_artifact_count+accum_val {
            return Err(ErrorBadRequest("Not enough artifacts in the bank"));
        }

        let mut conn = pg_pool
            .get()
            .map_err(|err| error::handle_error(err.into()))?;
        let mut building_artifact_count = web::block(move || {
            util::get_building_artifact_count(&mut conn, &current_layout_id, &transfer.map_space_id)
        })
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

        if building_artifact_count == -1 {
            let mut conn = pg_pool
                .get()
                .map_err(|err| error::handle_error(err.into()))?;
            web::block(move || util::create_artifact_record(&mut conn, &transfer.map_space_id, &0))
                .await?
                .map_err(|err| error::handle_error(err.into()))?;
            building_artifact_count = 0;
        }

        if transfer.artifacts_differ + building_artifact_count < 0 {
            return Err(ErrorBadRequest("Not enough artifacts in the building"));
        }

        let mut conn = pg_pool
            .get()
            .map_err(|err| error::handle_error(err.into()))?;
        let building_capacity =
            web::block(move || util::get_building_capacity(&mut conn, &transfer.map_space_id))
                .await?
                .map_err(|err| error::handle_error(err.into()))?;

        if building_capacity < transfer.artifacts_differ + building_artifact_count {
            return Err(ErrorBadRequest("Building capacity not sufficient"));
        }

        let new_building_artifact_count = building_artifact_count + transfer.artifacts_differ;
        let new_bank_artifact_count = bank_artifact_count - transfer.artifacts_differ;

        let mut conn = pg_pool
            .get()
            .map_err(|err| error::handle_error(err.into()))?;
        web::block(move || {
            util::transfer_artifacts_building(
                &mut conn,
                &transfer.map_space_id,
                &bank_map_space_id,
                &new_building_artifact_count,
                &new_bank_artifact_count,
            )
        })
        .await?
        .map_err(|err| error::handle_error(err.into()))?;
        accum_val += transfer.artifacts_differ;
        responses.push(TransferArtifactResponse {
            building_map_space_id: transfer.map_space_id,
            artifacts_in_building: new_building_artifact_count,
            bank_map_space_id,
            artifacts_in_bank: new_bank_artifact_count,
        });
    }

    Ok(web::Json(responses))
}

async fn get_user_base_details(pool: Data<PgPool>, user: AuthUser) -> Result<impl Responder> {
    let defender_id = user.0;
    let response = web::block(move || {
        let mut conn = pool.get()?;
        let user = fetch_user(&mut conn, defender_id)?;
        let map = util::fetch_map_layout(&mut conn, &defender_id)?;
        util::get_details_from_map_layout(&mut conn, map, user)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    Ok(Json(response))
}

async fn get_other_base_details(
    defender_id: web::Path<i32>,
    pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let defender_id = defender_id.into_inner();
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let defender_exists = web::block(move || util::defender_exists(defender_id, &mut conn))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;
    if !defender_exists {
        return Err(ErrorNotFound("Player not found"));
    }

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let map = web::block(move || util::fetch_map_layout(&mut conn, &defender_id))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

    if !map.is_valid {
        return Err(ErrorBadRequest("Invalid Base"));
    }

    let response = web::block(move || {
        let mut conn = pool.get()?;
        util::get_map_details_for_attack(&mut conn, map, defender_id)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    Ok(Json(response))
}

async fn get_game_base_details(
    game_id: web::Path<i32>,
    pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let game_id = game_id.into_inner();
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let map = web::block(move || util::fetch_map_layout_from_game(&mut conn, game_id))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

    if map.is_none() {
        return Err(ErrorNotFound("Game not found"));
    }

    let response = web::block(move || {
        let mut conn = pool.get()?;
        util::get_details_from_map_layout(&mut conn, map.unwrap(), None)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    Ok(Json(response))
}

async fn set_base_details(
    map_spaces: Json<Vec<MapSpacesEntry>>,
    pool: Data<PgPool>,
    user: AuthUser,
) -> Result<impl Responder> {
    let defender_id = user.0;
    let map_spaces = map_spaces.into_inner();
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let (map, blocks, buildings) = web::block(move || {
        Ok((
            util::fetch_map_layout(&mut conn, &defender_id)?,
            util::fetch_blocks(&mut conn, &defender_id)?,
            util::fetch_buildings(&mut conn)?,
        )) as anyhow::Result<(MapLayout, HashMap<i32, BlockType>, Vec<BuildingType>)>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    validate::is_valid_update_layout(&map_spaces, &blocks, &buildings)?;

    web::block(move || {
        let mut conn = pool.get()?;
        // util::set_map_invalid(&mut conn, map.id)?;
        util::put_base_details(&map_spaces, &map, &mut conn)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    Ok("Updated successfully")
}

async fn confirm_base_details(
    map_spaces: Json<Vec<MapSpacesEntry>>,
    redis_pool: Data<RedisPool>,
    pool: Data<PgPool>,
    user: AuthUser,
) -> Result<impl Responder> {
    let defender_id = user.0;

    let mut redis_conn = redis_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;

    if let Ok(Some(_)) = get_game_id_from_redis(defender_id, &mut redis_conn, false) {
        return Err(ErrorBadRequest("You are under attack. Cannot save base"));
    }

    let map_spaces = map_spaces.into_inner();
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let (map, blocks, mut level_constraints, buildings, defenders, mines, user_artifacts) =
        web::block(move || {
            let map = util::fetch_map_layout(&mut conn, &defender_id)?;
            Ok((
                map.clone(),
                util::fetch_blocks(&mut conn, &defender_id)?,
                util::get_level_constraints(&mut conn, map.level_id, &defender_id)?,
                util::fetch_buildings(&mut conn)?,
                util::fetch_defender_types(&mut conn, &defender_id)?,
                util::fetch_mine_types(&mut conn, &defender_id)?,
                get_user_artifacts(defender_id, &mut conn)?,
            ))
                as anyhow::Result<(
                    MapLayout,
                    HashMap<i32, BlockType>,
                    HashMap<i32, i32>,
                    Vec<BuildingType>,
                    Vec<DefenderTypeResponse>,
                    Vec<MineTypeResponse>,
                    i32,
                )>
        })
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

    validate::is_valid_save_layout(
        &map_spaces,
        &mut level_constraints,
        &blocks,
        &buildings,
        &defenders,
        &mines,
        &user_artifacts,
    )?;

    web::block(move || {
        let mut conn = pool.get()?;
        util::put_base_details(&map_spaces, &map, &mut conn)
        // util::calculate_shortest_paths(&mut conn, map.id)?;
        // util::set_map_valid(&mut conn, map.id)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    Ok("Saved successfully")
}

async fn defense_history(
    user: AuthUser,
    query: web::Query<HistoryboardQuery>,
    pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let user_id = user.0;
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    if page <= 0 || limit <= 0 {
        return Err(ErrorBadRequest("Invalid query params"));
    }
    let response = web::block(move || {
        let mut conn = pool.get()?;
        util::fetch_defense_historyboard(user_id, page, limit, &mut conn)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;
    Ok(web::Json(response))
}

async fn get_top_defenses(pool: web::Data<PgPool>, user: AuthUser) -> Result<impl Responder> {
    let user_id = user.0;
    let response = web::block(move || {
        let mut conn = pool.get()?;
        util::fetch_top_defenses(user_id, &mut conn)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;
    Ok(web::Json(response))
}

use self::util::{remove_game, NewAttack};
use super::auth::session::AuthUser;
use super::{error, PgPool};
use crate::api;
use crate::models::{AttackerType, LevelsFixture};
use actix_web::error::ErrorBadRequest;
use actix_web::{web, HttpResponse, Responder, Result};
use std::collections::{HashMap, HashSet};

mod rating;
pub mod util;
mod validate;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("").route(web::post().to(create_attack)))
        .service(web::resource("/{attacker_id}/history").route(web::get().to(attack_history)))
        .service(web::resource("/top").route(web::get().to(get_top_attacks)))
        .service(web::resource("/attacker_types").route(web::get().to(attacker_types)));
}

async fn create_attack(
    new_attack: web::Json<NewAttack>,
    pool: web::Data<PgPool>,
    user: AuthUser,
) -> Result<impl Responder> {
    let attacker_id = user.0;
    let attackers = new_attack.attackers.clone();

    if !util::is_attack_allowed_now() {
        return Err(ErrorBadRequest("Attack not allowed"));
    }

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let defender_id = new_attack.defender_id;
    let (level, map) = web::block(move || {
        let level = api::util::get_current_levels_fixture(&mut conn)?;
        let map = util::get_map_id(&defender_id, &level.id, &mut conn)?;
        Ok((level, map)) as anyhow::Result<(LevelsFixture, Option<i32>)>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    let map_id = if let Some(map) = map {
        map
    } else {
        return Err(ErrorBadRequest("Invalid base"));
    };

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let (valid_road_paths, valid_emp_ids, is_attack_allowed, attacker_types) =
        web::block(move || {
            let is_attack_allowed = util::is_attack_allowed(attacker_id, defender_id, &mut conn)?;
            let valid_emp_ids: HashSet<i32> = util::get_valid_emp_ids(&mut conn)?;
            let valid_road_paths = util::get_valid_road_paths(map_id, &mut conn)?;
            let attacker_types = util::get_attacker_types(&mut conn)?;
            Ok((
                valid_road_paths,
                valid_emp_ids,
                is_attack_allowed,
                attacker_types,
            ))
                as anyhow::Result<(
                    HashSet<(i32, i32)>,
                    HashSet<i32>,
                    bool,
                    HashMap<i32, AttackerType>,
                )>
        })
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

    if !is_attack_allowed {
        return Err(ErrorBadRequest("Attack not allowed"));
    }

    if !validate::is_attack_valid(
        &new_attack,
        valid_road_paths,
        valid_emp_ids,
        &level.no_of_bombs,
        &level.no_of_attackers,
        &attacker_types,
    ) {
        return Err(ErrorBadRequest("Invalid attack path"));
    }

    let file_content = web::block(move || {
        let mut conn = pool.get()?;
        let game_id = util::add_game(attacker_id, &new_attack, map_id, &mut conn)?;
        let sim_result = util::run_simulation(game_id, attackers, &mut conn);
        match sim_result {
            Ok(file_content) => Ok(file_content),
            Err(_) => {
                remove_game(game_id, &mut conn)?;
                Err(anyhow::anyhow!(
                    "Failed to run simulation for game {}",
                    game_id
                ))
            }
        }
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    Ok(HttpResponse::Ok().body(file_content))
}

async fn attack_history(
    attacker_id: web::Path<i32>,
    pool: web::Data<PgPool>,
    user: AuthUser,
) -> Result<impl Responder> {
    let user_id = user.0;
    let attacker_id = attacker_id.into_inner();
    let response = web::block(move || {
        let mut conn = pool.get()?;
        util::fetch_attack_history(attacker_id, user_id, &mut conn)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;
    Ok(web::Json(response))
}

async fn get_top_attacks(pool: web::Data<PgPool>, user: AuthUser) -> Result<impl Responder> {
    let user_id = user.0;
    let response = web::block(move || {
        let mut conn = pool.get()?;
        util::fetch_top_attacks(user_id, &mut conn)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;
    Ok(web::Json(response))
}

async fn attacker_types(pool: web::Data<PgPool>) -> Result<impl Responder> {
    let response = web::block(move || {
        let mut conn = pool.get()?;
        util::fetch_attacker_types(&mut conn)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;
    Ok(web::Json(response))
}

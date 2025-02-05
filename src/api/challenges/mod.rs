use std::{
    collections::{HashMap, HashSet},
    env, fs,
    io::Read,
};

use actix_web::{
    error::ErrorBadRequest,
    web::{self, Data, Json, Path, Payload, Query},
    HttpRequest, HttpResponse, Responder, Result,
};
use awc::http::{header::map, Error};
use serde::{Deserialize, Serialize};
use util::{get_challenge_maps, is_challenge_possible};

use crate::{
    api::{
        attack::{
            socket::BuildingResponse,
            util::{get_attacker_types, get_bomb_types, GameLog, ResultResponse},
        },
        defense::{
            shortest_path::{self, run_shortest_paths_challenges},
            util::{
                fetch_attacker_types, fetch_emp_types, AdminSaveData, BuildingTypeResponse,
                DefenderTypeResponse, MapSpacesResponseWithArifacts, MineTypeResponse,
                SimulationBaseResponse,
            },
        },
        user::util::fetch_user,
    },
    constants::MOD_USER_BASE_PATH,
    models::{AttackerType, User},
    validator::util::{
        BombType, BuildingDetails, Coords, DefenderDetails, MineDetails, SourceDestXY,
    },
};

use super::{auth::session::AuthUser, error, PgPool, RedisPool};

pub mod util;

#[derive(Deserialize, Serialize)]
pub struct ChallengeInitBody {
    challenge_id: i32,
    user_id: i32,
    map_id: i32,
}

pub struct ChallengeSocketQuery {
    pub challenge_id: i32,
    pub user_id: i32,
    pub map_id: i32,
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/{id}").route(web::get().to(challenge_maps)))
        .service(web::resource("/init").route(web::post().to(init_challenge)))
        .app_data(Data::new(web::JsonConfig::default().limit(1024 * 1024)));
}

async fn challenge_maps(challenge_id: Path<i32>, pool: Data<PgPool>) -> Result<impl Responder> {
    let challenge_id = challenge_id.into_inner();
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let challenge_maps = web::block(move || get_challenge_maps(&mut conn, challenge_id))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;
    Ok(Json(challenge_maps))
}

async fn init_challenge(
    body: Json<ChallengeInitBody>,
    redis_pool: Data<RedisPool>,
    pg_pool: Data<PgPool>,
    user: AuthUser,
) -> Result<impl Responder> {
    let attacker_id = user.0;
    let body = body.into_inner();

    log::info!(
        "User {} is trying to start challenge {}",
        attacker_id,
        body.challenge_id
    );
    let mut conn = pg_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;

    let is_challenge_possible = web::block(move || {
        is_challenge_possible(&mut conn, attacker_id, body.map_id, body.challenge_id)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    if !is_challenge_possible {
        return Err(ErrorBadRequest(
            "You have already played this challenge map before",
        ));
    }

    //challenge base data
    let json_path = env::current_dir()?.join(MOD_USER_BASE_PATH);
    log::info!("Json path: {}", json_path.display());
    let mut json_data_str = String::new();
    if json_path.exists() {
        let mut file = fs::File::open(json_path.clone())?;
        file.read_to_string(&mut json_data_str)?;
    }

    let json_data: HashMap<i32, HashMap<i32, AdminSaveData>> = if json_data_str.is_empty() {
        HashMap::new()
    } else {
        serde_json::from_str(&json_data_str).unwrap_or_else(|_| HashMap::new())
    };

    let default_user_rec = HashMap::new();
    let map_data = json_data.get(&body.user_id).unwrap_or(&default_user_rec);
    let map_data = map_data
        .get(&body.map_id)
        .unwrap_or(&AdminSaveData {
            map_id: body.map_id,
            building: Vec::new(),
            defenders: Vec::new(),
            mine_type: Vec::new(),
            road: Vec::new(),
        })
        .clone();

    Ok(Json(map_data))
}

async fn challenge_socket_handler(
    query_params: Query<ChallengeSocketQuery>,
    pool: Data<PgPool>,
    user: AuthUser,
    body: Payload,
    req: HttpRequest,
) -> Result<HttpResponse> {
    //challenge base data
    let attacker_id = user.0;
    let mod_user_id = query_params.user_id;

    let json_path = env::current_dir()?.join(MOD_USER_BASE_PATH);
    log::info!("Json path: {}", json_path.display());
    let mut json_data_str = String::new();
    if json_path.exists() {
        let mut file = fs::File::open(json_path.clone())?;
        file.read_to_string(&mut json_data_str)?;
    }

    let json_data: HashMap<i32, HashMap<i32, AdminSaveData>> = if json_data_str.is_empty() {
        HashMap::new()
    } else {
        serde_json::from_str(&json_data_str).unwrap_or_else(|_| HashMap::new())
    };

    let default_user_rec = HashMap::new();
    let map_data = json_data
        .get(&query_params.user_id)
        .unwrap_or(&default_user_rec);
    let map_data = map_data
        .get(&query_params.map_id)
        .unwrap_or(&AdminSaveData {
            map_id: query_params.map_id,
            building: Vec::new(),
            defenders: Vec::new(),
            mine_type: Vec::new(),
            road: Vec::new(),
        })
        .clone();

    let shortest_paths =
        run_shortest_paths_challenges(&map_data.road).expect("Error getting shortest paths");

    let defenders: Vec<DefenderDetails> = map_data
        .defenders
        .iter()
        .map(|defender| DefenderDetails {
            block_id: defender.block_id,
            mapSpaceId: defender.map_space_id,
            name: defender.name.clone(),
            radius: defender.radius,
            speed: defender.speed,
            damage: defender.damage,
            defender_pos: Coords {
                x: defender.pos_x,
                y: defender.pos_y,
            },
            is_alive: true,
            damage_dealt: false,
            target_id: None,
            path_in_current_frame: Vec::new(),
            level: defender.level,
        })
        .collect();

    //let hut_defenders = None;

    let mines: Vec<MineDetails> = map_data
        .mine_type
        .iter()
        .map(|mine_save| MineDetails {
            damage: mine_save.damage,
            id: mine_save.id,
            position: Coords {
                x: mine_save.pos_x,
                y: mine_save.pos_y,
            },
            radius: mine_save.radius,
        })
        .collect();

    let buildings: Vec<BuildingDetails> = map_data
        .building
        .iter()
        .map(|building| BuildingDetails {
            artifacts_obtained: 0,
            block_id: building.block_id,
            map_space_id: building.map_space_id,
            current_hp: building.hp,
            total_hp: building.hp,
            tile: Coords {
                x: building.pos_x,
                y: building.pos_y,
            },
            width: building.width_in_tiles,
            name: building.name.clone(),
            range: building.range,
            frequency: building.frequency,
        })
        .collect();

    let roads: HashSet<(i32, i32)> = map_data
        .road
        .iter()
        .map(|road_save| (road_save.pos_x, road_save.pos_y))
        .collect();

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let bomb_types =
        web::block(move || Ok(get_bomb_types(&mut conn)?) as anyhow::Result<Vec<BombType>>)
            .await?
            .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let attacker_type = web::block(move || {
        Ok(get_attacker_types(&mut conn)?) as anyhow::Result<HashMap<i32, AttackerType>>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let defender_user_details =
        web::block(move || Ok(fetch_user(&mut conn, mod_user_id)?) as anyhow::Result<Option<User>>)
            .await?
            .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let attacker_user_details =
        web::block(move || Ok(fetch_user(&mut conn, attacker_id)?) as anyhow::Result<Option<User>>)
            .await?
            .map_err(|err| error::handle_error(err.into()))?;

    let map_spaces_response_with_artifacts_sim: Vec<MapSpacesResponseWithArifacts> = map_data
        .building
        .iter()
        .map(|building| MapSpacesResponseWithArifacts {
            artifacts: Some(building.artifacts),
            id: building.map_space_id,
            x_coordinate: building.pos_x,
            y_coordinate: building.pos_y,
            block_type_id: building.block_id,
        })
        .collect();

    let building_type_response_sim: Vec<BuildingTypeResponse> = map_data
        .building
        .iter()
        .map(|building| BuildingTypeResponse {
            id: building.id,
            block_id: building.block_id,
            name: building.name.clone(),
            width: building.width_in_tiles,
            height: building.length_in_tiles,
            level: building.level,
            capacity: building.capacity,
            cost: building.cost,
            hp: building.hp,
            range: building.range,
            frequency: building.frequency,
        })
        .collect();

    let defender_type_response_sim: Vec<DefenderTypeResponse> = map_data
        .defenders
        .iter()
        .map(|defender| DefenderTypeResponse {
            id: defender.id,
            radius: defender.radius,
            speed: defender.speed,
            damage: defender.damage,
            block_id: defender.block_id,
            level: defender.level,
            cost: defender.cost,
            name: defender.name.clone(),
        })
        .collect();

    let mine_type_response_sim: Vec<MineTypeResponse> = map_data
        .mine_type
        .iter()
        .map(|mine| MineTypeResponse {
            id: mine.id,
            radius: mine.radius,
            damage: mine.damage,
            block_id: mine.block_id,
            level: mine.level,
            cost: mine.cost,
            name: mine.name.clone(),
        })
        .collect();

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let attacker_types_sim = web::block(move || fetch_attacker_types(&mut conn, &attacker_id))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let bomb_types_sim = web::block(move || fetch_emp_types(&mut conn, &attacker_id))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

    let defender_base_details = SimulationBaseResponse {
        m: map_data.map_id,
        ms: map_spaces_response_with_artifacts_sim,
        b: building_type_response_sim,
        d: defender_type_response_sim,
        mt: mine_type_response_sim,
        at: attacker_types_sim,
        bt: bomb_types_sim,
    };

    if attacker_user_details.is_none() {
        return Err(ErrorBadRequest("Attacker Does not exist"));
    }

    let mut damaged_buildings: Vec<BuildingResponse> = Vec::new();

    let game_log = GameLog {
        g: -1,
        a: attacker_user_details.unwrap(),
        d: defender_user_details.unwrap(),
        b: defender_base_details,
        e: Vec::new(),
        r: ResultResponse {
            d: 0,
            a: 0,
            b: 0,
            au: 0,
            na: 0,
            nd: 0,
            oa: 0,
            od: 0,
        },
    };

    log::info!(
        "Challenge:{} is ready for player:{}",
        query_params.challenge_id,
        attacker_id
    );

    let (response, session, mut msg_stream) = actix_ws::handle(&req, body)?;

    Ok(response)
}

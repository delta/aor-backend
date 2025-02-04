use std::{collections::HashMap, env, fs, io::Read};

use actix_web::{
    error::ErrorBadRequest,
    web::{self, Data, Json, Path},
    Responder, Result,
};
use util::{get_challenge_maps, is_challenge_possible};

use crate::{
    api::{
        attack::util::{add_game, encode_attack_token},
        defense::util::AdminSaveData,
    },
    constants::MOD_USER_BASE_PATH,
};

use super::{auth::session::AuthUser, error, PgPool, RedisPool};

pub mod util;

pub struct ChallengeInitBody {
    challenge_id: i32,
    user_id: i32,
    map_id: i32,
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/{id}").route(web::get().to(challenge_maps)))
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

use self::util::{upgrade_attacker, upgrade_building, upgrade_defender, upgrade_emp, upgrade_mine};
use super::{
    attack::util::get_game_id_from_redis, auth::session::AuthUser, error, PgPool, RedisPool,
};
use actix_web::{
    error::ErrorBadRequest,
    web::{self, Json},
    Responder, Result,
};
use serde::{Deserialize, Serialize};
pub mod util;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/get").route(web::get().to(get_inventory)))
        .service(web::resource("/upgrade").route(web::post().to(upgrade)));
}

async fn get_inventory(user: AuthUser, pool: web::Data<PgPool>) -> Result<impl Responder> {
    let user_id = user.0;
    let response = web::block(move || {
        let mut conn = pool.get()?;
        util::get_inventory(user_id, &mut conn)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    Ok(web::Json(response))
}

#[derive(Deserialize, Serialize)]
struct UpgradeStruct {
    pub item_type: String,
    pub item_id: i32,
    pub map_space_id: Option<i32>,
}

async fn upgrade(
    pool: web::Data<PgPool>,
    redis_pool: web::Data<RedisPool>,
    user: AuthUser,
    req: Json<UpgradeStruct>,
) -> Result<impl Responder> {
    let user_id = user.0;
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let item_type = &req.item_type;
    let item_id = req.item_id;
    let map_space_id = req.map_space_id.unwrap_or(-1);

    let mut redis_conn = redis_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;

    if let Ok(Some(_)) = get_game_id_from_redis(user_id, &mut redis_conn, false) {
        return Err(ErrorBadRequest("You are under attack. Cannot upgrade now"));
    }

    let mut map_space_id_if_valid = 0;

    match item_type.as_str() {
        "attacker" => upgrade_attacker(user_id, &mut conn, item_id)
            .map_err(|err| ErrorBadRequest(err.to_string()))?,
        "building" => {
            map_space_id_if_valid = upgrade_building(user_id, &mut conn, item_id, map_space_id)
                .map_err(|err| ErrorBadRequest(err.to_string()))?;
        }
        "defender" => upgrade_defender(user_id, &mut conn, item_id)
            .map_err(|err| ErrorBadRequest(err.to_string()))?,
        "emp" => upgrade_emp(user_id, &mut conn, item_id)
            .map_err(|err| ErrorBadRequest(err.to_string()))?,
        "mine" => upgrade_mine(user_id, &mut conn, item_id)
            .map_err(|err| ErrorBadRequest(err.to_string()))?,
        _ => return Err(ErrorBadRequest("Invalid item type")),
    }
    Ok(Json(map_space_id_if_valid))
}

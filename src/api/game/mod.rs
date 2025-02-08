use super::{auth::session::AuthUser, error, PgPool};
use actix_web::{error::ErrorBadRequest, web, Responder, Result};
use util::LeaderboardQuery;

pub mod util;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/leaderboard").route(web::get().to(list_leaderboard)))
        .service(web::resource("/{game_id}/replay").route(web::get().to(get_replay)))
        .service(web::resource("/{game_id}/stats").route(web::get().to(get_game_details)));
}

async fn list_leaderboard(
    query: web::Query<LeaderboardQuery>,
    pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    if page <= 0 || limit <= 0 {
        return Err(ErrorBadRequest("Invalid query params"));
    }
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;

    let response = util::get_leaderboard(page, limit, &mut conn)
        .map_err(|err| error::handle_error(err.into()))?;
    Ok(web::Json(response))
}
async fn get_replay(
    game_id: web::Path<i32>,
    pool: web::Data<PgPool>,
    user: AuthUser,
) -> Result<impl Responder> {
    let user_id = user.0;
    let game_id = game_id.into_inner();

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let is_replay_allowed = util::fetch_is_replay_allowed(game_id, user_id, &mut conn)
        .map_err(|err| error::handle_error(err.into()))?;

    if !is_replay_allowed {
        return Err(ErrorBadRequest("Requested replay is not available"));
    }

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let response =
        util::fetch_replay(game_id, &mut conn).map_err(|err| error::handle_error(err.into()))?;
    Ok(web::Json(response))
}

async fn get_game_details(
    game_id: web::Path<i32>,
    pool: web::Data<PgPool>,
    user: AuthUser,
) -> Result<impl Responder> {
    let user_id = user.0;
    let game_id = game_id.into_inner();

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let response = util::fetch_game_details(game_id, user_id, &mut conn)
        .map_err(|err| error::handle_error(err.into()))?;

    Ok(web::Json(response))
}

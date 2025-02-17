use super::auth::session::AuthUser;
use super::{PgPool, RedisPool};
use crate::api::error;
use crate::models::UpdateUser;
use actix_web::error::{ErrorBadRequest, ErrorConflict, ErrorNotFound};
use actix_web::web::{self, Data, Json, Path};
use actix_web::{Responder, Result};
use serde::Deserialize;

pub mod util;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/update").route(web::patch().to(update_user)))
        .service(web::resource("/profile/{player_id}").route(web::get().to(view_user_profile)))
        .service(web::resource("/register").route(web::post().to(register)))
        .service(web::resource("/{id}/stats").route(web::get().to(get_user_stats)))
        .service(web::resource("/update-avatar").route(web::patch().to(update_avatar_id)));
}

#[derive(Clone, Deserialize)]
pub struct InputUser {
    name: String,
    username: String,
}

// #[derive(Debug, Serialize)]
// struct ErrorResponse {
//     message: String,
// }
// #[derive(Debug, Serialize)]
// struct SuccessResponse {
//     message: String,
// }
async fn register(
    pg_pool: Data<PgPool>,
    redis_pool: Data<RedisPool>,
    input_user: Json<InputUser>,
) -> Result<impl Responder> {
    let mut conn = pg_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;
    let user = input_user.clone();
    if user.username.len() < 6 {
        return Err(ErrorBadRequest(
            "Username should contain atleast 6 characters",
        ));
    }
    let duplicates = web::block(move || util::get_duplicate_users(&mut conn, &user))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;
    for duplicate in duplicates {
        if duplicate.username == input_user.username {
            return Err(ErrorConflict("Username already exists"));
        }
    }
    web::block(move || {
        let mut conn = pg_pool.get()?;
        let redis_conn = redis_pool.get()?;
        util::add_user(&mut conn, redis_conn, &input_user)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;
    Ok("Successfully Registered")
}

async fn update_user(
    user_details: Json<UpdateUser>,
    pool: Data<PgPool>,
    user: AuthUser,
) -> Result<impl Responder> {
    let user_id = user.0;
    let username = user_details.username.clone();
    if username.is_some()
        && (username.as_ref().unwrap().len() < 5 || username.as_ref().unwrap().len() > 15)
    {
        return Err(ErrorBadRequest(
            "Username should contain atleast 5 characters and atmost 15 characters",
        ));
    }
    if let Some(username) = username {
        let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
        let duplicate = web::block(move || util::get_duplicate_username(&mut conn, &username))
            .await?
            .map_err(|err| error::handle_error(err.into()))?;
        if duplicate.is_some() && duplicate.as_ref().unwrap().id != user_id {
            return Err(ErrorConflict("Username already exists"));
        }
        if duplicate.is_some() && duplicate.unwrap().id == user_id {
            return Ok("No change in Username");
        }
    }
    web::block(move || {
        let mut conn = pool.get()?;
        util::update_user(&mut conn, user_id, &user_details)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    Ok("User updated successfully")
}

async fn get_user_stats(user_id: Path<i32>, pool: Data<PgPool>) -> Result<impl Responder> {
    let user_id = user_id.into_inner();
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let user = web::block(move || util::fetch_user(&mut conn, user_id))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;
    if let Some(user) = user {
        let response = web::block(move || {
            let mut conn = pool.get()?;
            let attack_game = util::fetch_attack_game(&mut conn, user_id)?;
            let defense_game = util::fetch_defense_game(&mut conn, user_id)?;
            let users = util::fetch_all_user(&mut conn)?;
            util::make_response(&user, &attack_game, &defense_game, &users)
        })
        .await?
        .map_err(|err| error::handle_error(err.into()))?;
        Ok(Json(response))
    } else {
        Err(ErrorNotFound("User not found"))
    }
}

async fn view_user_profile(player_id: Path<i32>, pool: Data<PgPool>) -> Result<impl Responder> {
    let user_id = player_id.into_inner();
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let user = web::block(move || util::fetch_user(&mut conn, user_id))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;
    if let Some(user) = user {
        let response = web::block(move || {
            let mut conn = pool.get()?;
            let users = util::fetch_all_user(&mut conn)?;
            util::make_profile_response(&user, &users)
        })
        .await?
        .map_err(|err| error::handle_error(err.into()))?;
        Ok(Json(response))
    } else {
        Err(ErrorNotFound("User not found"))
    }
}

async fn update_avatar_id(pool: Data<PgPool>, user: AuthUser, user_details: Json<UpdateUser>) -> Result<impl Responder> {
    let user_id = user.0;
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    web::block(move || util::update_user(&mut conn, user_id, &user_details))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;
    Ok("Avatar updated successfully")
}
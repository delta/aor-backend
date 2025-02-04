use actix_web::{
    web::{self, Data, Json, Path},
    Responder, Result,
};

use super::PgPool;

pub mod util;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/{id}").route(web::get().to(get_challenge_maps)))
        .app_data(Data::new(web::JsonConfig::default().limit(1024 * 1024)));
}

async fn get_challenge_maps(challenge_id: Path<i32>, pool: Data<PgPool>) -> Result<impl Responder> {
    Ok(Json(()))
}

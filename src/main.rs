use crate::api::{attack, auth, defense, game, inventory, user};
use actix_cors::Cors;
use actix_session::{
    config::PersistentSession, storage::RedisActorSessionStore, SessionMiddleware,
};
use actix_web::cookie::time::Duration;
use actix_web::web::Data;
use actix_web::{cookie::Key, middleware, web, App, HttpResponse, HttpServer};
use api::challenges;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use flexi_logger::{Cleanup, Criterion, Duplicate, FileSpec, Naming};

mod api;
mod constants;
mod error;
mod models;
mod schema;
mod util;
mod validator;

#[macro_use]
extern crate diesel;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    flexi_logger::Logger::try_with_str("info, actix_web=info")
        .unwrap()
        .log_to_file(FileSpec::default().directory("./logs"))
        .append()
        .duplicate_to_stderr(Duplicate::All)
        .rotate(
            Criterion::Size(50 * 1024 * 1024),
            Naming::Timestamps,
            Cleanup::Never,
        )
        .start()
        .unwrap();

    let pg_pool = util::get_pg_conn_pool();
    let redis_pool = util::get_redis_conn_pool();
    let cookie_key = std::env::var("COOKIE_KEY").expect("COOKIE_KEY must be set");
    let key = Key::derive_from(cookie_key.as_bytes());
    let frontend_url = std::env::var("FRONTEND_URL").expect("FRONTEND_URL must be set");
    let redis_url = std::env::var("REDIS_URL").unwrap();

    let conn = &mut pg_pool.get().expect("Could not get connection from pool");
    conn.run_pending_migrations(MIGRATIONS).unwrap();
    let max_age: i64 = std::env::var("MAX_AGE_IN_MINUTES")
        .expect("max age must be set!")
        .parse()
        .expect("max age must be an integer!");
    HttpServer::new(move || {
        App::new()
            .wrap(
                SessionMiddleware::builder(RedisActorSessionStore::new(&redis_url), key.clone())
                    .cookie_name("session".to_string())
                    .session_lifecycle(
                        PersistentSession::default().session_ttl(Duration::minutes(max_age)),
                    )
                    .build(),
            )
            .wrap(
                Cors::default()
                    .allowed_origin(&frontend_url)
                    .allow_any_header()
                    .allow_any_method()
                    .expose_any_header()
                    .supports_credentials()
                    .max_age(3600),
            )
            .wrap(middleware::Logger::new(
                "%t %{r}a %r %s %b %{Referer}i %{User-Agent}i %T",
            ))
            .app_data(Data::new(pg_pool.clone()))
            .app_data(Data::new(redis_pool.clone()))
            .route("/", web::get().to(HttpResponse::Ok))
            .service(web::scope("/attack").configure(attack::routes))
            .service(
                web::scope("/user")
                    .configure(user::routes)
                    .configure(auth::routes),
            )
            .service(web::scope("/base").configure(defense::routes))
            .service(web::scope("/game").configure(game::routes))
            .service(web::scope("/inventory").configure(inventory::routes))
            .service(web::scope("/challenge").configure(challenges::routes))
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}

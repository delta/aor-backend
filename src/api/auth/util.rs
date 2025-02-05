use crate::api::defense::util::{add_user_default_base};
use crate::api::error;
use crate::error::DieselError;
use crate::models::*;
use crate::schema::user::{self};
use crate::util::function;
use anyhow::Result;
use chrono::{Duration, Local};
use diesel::prelude::*;
use jsonwebtoken::{encode, EncodingKey, Header};
use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use rand::{distributions::Alphanumeric, Rng};
use std::env;

use super::TokenClaims;

pub fn client() -> BasicClient {
    let google_client_id = ClientId::new(
        env::var("GOOGLE_OAUTH_CLIENT_ID").expect("Google oauth client id must be set!"),
    );
    let google_client_secret = ClientSecret::new(
        env::var("GOOGLE_OAUTH_CLIENT_SECRET").expect("Google oauth client secret must be set!"),
    );
    let auth_url = env::var("GOOGLE_OAUTH_AUTH_URL").expect("Google oauth auth URL must be set!");
    let token_url =
        env::var("GOOGLE_OAUTH_TOKEN_URL").expect("Google oauth token URL must be set!");

    let auth_url = AuthUrl::new(auth_url).expect("Invalid authorization endpoint URL");
    let token_url = TokenUrl::new(token_url).expect("Invalid token endpoint URL");

    // Set up the client for the Google OAuth2 process.
    let redirect_url =
        env::var("GOOGLE_OAUTH_REDIRECT_URL").expect("Google oauth redirect URL must be set!");

    BasicClient::new(
        google_client_id,
        Some(google_client_secret),
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_url).expect("Invalid redirect URL"))
}

pub fn generate_jwt_token(id: i32) -> Result<(String, String, String)> {
    let jwt_secret = env::var("COOKIE_KEY").expect("COOKIE_KEY must be set!");
    let now = Local::now();
    let iat = now.timestamp() as usize;
    let jwt_max_age: i64 = env::var("MAX_AGE_IN_MINUTES")
        .expect("JWT max age must be set!")
        .parse()
        .expect("JWT max age must be an integer!");
    let token_expiring_time = now + Duration::minutes(jwt_max_age);
    let exp = (token_expiring_time).timestamp() as usize;
    let device: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(4)
        .map(char::from)
        .collect();
    let claims: TokenClaims = TokenClaims {
        id,
        device: device.clone() + &exp.to_string(),
        exp,
        iat,
    };

    let token_result = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    );
    let token = match token_result {
        Ok(token) => token,
        Err(e) => return Err(e.into()),
    };

    Ok((token, exp.to_string(), device))
}

pub fn get_oauth_user(pg_conn: &mut PgConnection, email: &str, name: &str) -> Result<User> {
    // Already logged in before
    if let Some(user) = user::table
        .filter(user::email.eq(&email))
        .first::<User>(pg_conn)
        .optional()
        .map_err(|err| DieselError {
            table: "user",
            function: function!(),
            error: err,
        })?
    {
        Ok(user)
    } else if let Ok(user) =
        add_user_default_base(pg_conn, name, email).map_err(|err| error::handle_error(err.into()))
    {
        Ok(user)
    } else {
        Err(anyhow::anyhow!(
            "Can't add user to database! Try again later."
        ))
    }
}

use anyhow::{Ok, Result};
use chrono::Utc;
use diesel::prelude::*;
use diesel::PgConnection;
use serde::Serialize;

use crate::error::DieselError;
use crate::util::function;
use crate::{models::Challenge, schema::challenges};

#[derive(Serialize)]
pub struct ChallengeTypeResponse {
    pub id: i32,
    pub name: String,
}

pub fn get_challenge_type(conn: &mut PgConnection) -> Result<Option<ChallengeTypeResponse>> {
    let now = Utc::now().naive_utc();

    let current_challenge = challenges::table
        .filter(challenges::start.le(now))
        .filter(challenges::end.ge(now))
        .first::<Challenge>(conn)
        .optional()
        .map_err(|err| DieselError {
            table: "challenges",
            function: function!(),
            error: err,
        })?;
    let res_challenge_response = if let Some(current_challenge) = current_challenge {
        Some(ChallengeTypeResponse {
            id: current_challenge.id,
            name: current_challenge.name,
        })
    } else {
        None
    };

    Ok(res_challenge_response)
}

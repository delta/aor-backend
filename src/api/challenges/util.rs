use anyhow::{Ok, Result};
use chrono::Utc;
use diesel::prelude::*;
use diesel::PgConnection;
use serde::Serialize;

use crate::api::defense::util::AdminSaveData;
use crate::error::DieselError;
use crate::models::ChallengeMap;
use crate::models::ChallengeResponse;
use crate::schema::challenge_maps;
use crate::schema::challenges_responses;
use crate::util::function;
use crate::{models::Challenge, schema::challenges};

#[derive(Serialize)]
pub struct ChallengeTypeResponse {
    pub id: i32,
    pub name: String,
}

#[derive(Serialize)]
pub struct ChallengeMapsResponse {
    pub id: i32,
    pub user_id: i32,
    pub map_id: i32,
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

pub fn get_challenge_maps(
    conn: &mut PgConnection,
    challenge_id: i32,
) -> Result<Vec<ChallengeMapsResponse>> {
    let challenge_maps_resp: Vec<ChallengeMapsResponse> = challenge_maps::table
        .inner_join(challenges::table)
        .filter(challenges::id.eq(challenge_id))
        .load::<(ChallengeMap, Challenge)>(conn)
        .map_err(|err| DieselError {
            table: "challenge_maps",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(challenge_map, _)| ChallengeMapsResponse {
            id: challenge_map.id,
            user_id: challenge_map.user_id,
            map_id: challenge_map.map_id,
        })
        .collect();

    Ok(challenge_maps_resp)
}

pub fn is_challenge_possible(
    conn: &mut PgConnection,
    user_id: i32,
    map_id: i32,
    challenge_id: i32,
) -> Result<bool> {
    let challenge_response = challenges_responses::table
        .filter(
            challenges_responses::challenge_id.eq(challenge_id).and(
                challenges_responses::attacker_id
                    .eq(user_id)
                    .and(challenges_responses::map_id.eq(map_id)),
            ),
        )
        .first::<ChallengeResponse>(conn)
        .optional()?;
    let is_possible = challenge_response.is_none();

    Ok(is_possible)
}

use anyhow::{Ok, Result};
use chrono::Utc;
use diesel::dsl::{sql, sum};
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use diesel::PgConnection;
use serde::Serialize;

use crate::api::attack::util::GameLog;
use crate::constants::MAX_CHALLENGE_ATTEMPTS;
use crate::error::DieselError;
use crate::models::ChallengeMap;
use crate::models::ChallengeResponse;
use crate::models::NewChallengeResponse;
use crate::schema::challenge_maps;
use crate::schema::challenges_responses;
use crate::schema::user;
use crate::util::function;
use crate::validator::util::ChallengeType;
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
    pub completed: bool,
    pub attempts: i32,
}

#[derive(Serialize)]
pub struct ChallengeLeaderBoardResponse {
    id: i32,
    name: String,
    score: i64,
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
    attacker_id: i32,
) -> Result<Vec<ChallengeMapsResponse>> {
    let user_challenge_response = challenges_responses::table
        .filter(challenges_responses::attacker_id.eq(attacker_id))
        .filter(challenges_responses::challenge_id.eq(challenge_id))
        .load::<ChallengeResponse>(conn)
        .map_err(|err| DieselError {
            table: "challenges_responses",
            function: function!(),
            error: err,
        })?;

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
        .map(|(challenge_map, _)| {
            let completed =
                is_challenge_possible(conn, attacker_id, challenge_map.map_id, challenge_id);

            let completed = match completed {
                core::result::Result::Ok(completed) => {
                    log::info!("is possible: {completed}");
                    !completed
                }
                Err(_) => {
                    log::info!("Errorrrr");
                    false
                }
            };

            ChallengeMapsResponse {
                id: challenge_map.id,
                user_id: challenge_map.user_id,
                map_id: challenge_map.map_id,
                attempts: user_challenge_response
                    .iter()
                    .find(|&x| x.map_id == challenge_map.map_id)
                    .map_or(0, |x| x.attempts),
                completed,
            }
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

    let is_possible = if let Some(challenge_response) = challenge_response {
        challenge_response.attempts < MAX_CHALLENGE_ATTEMPTS
    } else {
        true
    };

    Ok(is_possible)
}

pub fn terminate_challenge(
    conn: &mut PgConnection,
    game_log: &mut GameLog,
    map_id: i32,
    challenge_id: i32,
) -> Result<()> {
    let attacker_id = game_log.a.id;
    let score = game_log.r.sc;

    let new_challenge_resp = NewChallengeResponse {
        attacker_id: &attacker_id,
        challenge_id: &challenge_id,
        map_id: &map_id,
        score: &score,
        attempts: &1,
    };

    let inserted_response: ChallengeResponse = diesel::insert_into(challenges_responses::table)
        .values(&new_challenge_resp)
        .on_conflict((
            challenges_responses::attacker_id,
            challenges_responses::challenge_id,
            challenges_responses::map_id,
        ))
        .do_update()
        .set((
            challenges_responses::attempts.eq(challenges_responses::attempts + 1),
            challenges_responses::score
                .eq(sql("GREATEST(challenges_responses.score, EXCLUDED.score)")),
        ))
        .get_result(conn)
        .map_err(|err| DieselError {
            table: "challenge_responses",
            function: function!(),
            error: err,
        })?;

    Ok(())
}

pub fn get_challenge_type_enum(
    conn: &mut PgConnection,
    challenge_id: i32,
) -> Result<Option<ChallengeType>> {
    let resp: Challenge = challenges::table
        .filter(challenges::id.eq(challenge_id))
        .first::<Challenge>(conn)
        .map_err(|err| DieselError {
            table: "challenges",
            function: function!(),
            error: err,
        })?;

    let challege_type = if resp.name == "Maze" {
        Some(ChallengeType::Maze)
    } else if resp.name == "FallGuys" {
        Some(ChallengeType::FallGuys)
    } else {
        None
    };

    Ok(challege_type)
}

pub fn get_leaderboard(
    conn: &mut PgConnection,
    challenge_id: i32,
) -> Result<Vec<(ChallengeLeaderBoardResponse)>> {
    let resp = challenges_responses::table
        .inner_join(user::table.on(challenges_responses::attacker_id.eq(user::id)))
        .filter(challenges_responses::challenge_id.eq(challenge_id))
        .group_by(user::id)
        .select((
            user::id,
            user::name,
            sum(challenges_responses::score).nullable(), // Ensuring compatibility
        ))
        .order(sum(challenges_responses::score).nullable().desc()) // Sorting remains correct
        .load::<(i32, String, Option<i64>)>(conn) // Handling nullable sum return type
        .map_err(|err| DieselError {
            table: "challenges_responses",
            function: function!(),
            error: err,
        })?;

    // Convert `Option<i64>` to `i64`, replacing `None` with `0`
    let leaderboard = resp
        .into_iter()
        .map(|(id, name, score_)| ChallengeLeaderBoardResponse {
            id,
            name,
            score: score_.unwrap_or(0),
        })
        .collect();

    Ok(leaderboard)
}

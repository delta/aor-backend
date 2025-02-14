use super::InputUser;
use crate::api::RedisConn;
use crate::constants::INITIAL_RATING;
use crate::error::DieselError;
use crate::models::NewUser;
use crate::models::{Game, UpdateUser, User};
use crate::util::function;
use anyhow::Result;
use diesel::prelude::*;
use redis::Commands;
use serde::Serialize;

#[derive(Serialize)]
pub struct StatsResponse {
    pub highest_attack_score: i32,
    pub highest_defense_score: i32,
    pub trophies: i32,
    pub position_in_leaderboard: i32,
    pub no_of_emps_used: i32,
    pub total_damage_defense: i32,
    pub total_damage_attack: i32,
    pub no_of_attackers_suicided: i32,
    pub no_of_attacks: i32,
    pub no_of_defenses: i32,
}

#[derive(Serialize)]
pub struct UserProfileResponse {
    user_id: i32,
    name: String,
    username: String,
    trophies: i32,
    artifacts: i32,
    attacks_won: i32,
    defenses_won: i32,
    avatar_id: i32,
    leaderboard_position: i32,
}

pub fn fetch_user(conn: &mut PgConnection, player_id: i32) -> Result<Option<User>> {
    use crate::schema::user;
    Ok(user::table
        .filter(user::id.eq(player_id))
        .first::<User>(conn)
        .optional()
        .map_err(|err| DieselError {
            table: "user",
            function: function!(),
            error: err,
        })?)
}

pub fn fetch_all_user(conn: &mut PgConnection) -> Result<Vec<User>> {
    use crate::schema::user;
    Ok(user::table
        .order_by(user::trophies.desc())
        .load::<User>(conn)
        .map_err(|err| DieselError {
            table: "user",
            function: function!(),
            error: err,
        })?)
}

pub fn add_user(
    pg_conn: &mut PgConnection,
    mut redis_conn: RedisConn,
    user: &InputUser,
) -> anyhow::Result<()> {
    use crate::schema::user;
    let new_user = NewUser {
        name: &user.name,
        email: "",
        username: &user.username,
        is_pragyan: &false,
        attacks_won: &0,
        defenses_won: &0,
        trophies: &INITIAL_RATING,
        avatar_id: &0,
        artifacts: &0,
    };
    let user: User = diesel::insert_into(user::table)
        .values(&new_user)
        .get_result(pg_conn)
        .map_err(|err| DieselError {
            table: "user",
            function: function!(),
            error: err,
        })?;
    // Set last reset password time as 0 for new user
    redis_conn.set::<_, _, ()>(user.id, 0)?;
    Ok(())
}

pub fn update_user(conn: &mut PgConnection, user_id: i32, update_user: &UpdateUser) -> Result<()> {
    use crate::schema::user;
    diesel::update(user::table.find(user_id))
        .set(update_user)
        .execute(conn)
        .map_err(|err| DieselError {
            table: "user",
            function: function!(),
            error: err,
        })?;
    Ok(())
}

pub fn get_duplicate_users(conn: &mut PgConnection, user: &InputUser) -> Result<Vec<User>> {
    use crate::schema::user;
    let duplicates = user::table
        .filter(user::username.eq(&user.username))
        .load::<User>(conn)
        .map_err(|err| DieselError {
            table: "user",
            function: function!(),
            error: err,
        })?;
    Ok(duplicates)
}

pub fn get_duplicate_username(conn: &mut PgConnection, username: &str) -> Result<Option<User>> {
    use crate::schema::user;
    Ok(user::table
        .filter(user::username.eq(username))
        .first::<User>(conn)
        .optional()
        .map_err(|err| DieselError {
            table: "user",
            function: function!(),
            error: err,
        })?)
}

pub fn fetch_attack_game(conn: &mut PgConnection, player_id: i32) -> Result<Vec<Game>> {
    use crate::schema::game;
    Ok(game::table
        .filter(game::attack_id.eq(player_id))
        .order_by(game::attack_score.desc())
        .load::<Game>(conn)
        .map_err(|err| DieselError {
            table: "game",
            function: function!(),
            error: err,
        })?)
}

pub fn fetch_defense_game(conn: &mut PgConnection, player_id: i32) -> Result<Vec<Game>> {
    use crate::schema::game;
    Ok(game::table
        .filter(game::defend_id.eq(player_id))
        .order_by(game::defend_score.desc())
        .load::<Game>(conn)
        .map_err(|err| DieselError {
            table: "game",
            function: function!(),
            error: err,
        })?)
}

pub fn make_profile_response(user: &User, users: &[User]) -> Result<UserProfileResponse> {
    let mut profile = UserProfileResponse {
        user_id: user.id,
        name: user.name.clone(),
        username: user.username.clone(),
        trophies: user.trophies,
        artifacts: user.artifacts,
        attacks_won: user.attacks_won,
        defenses_won: user.defenses_won,
        avatar_id: user.avatar_id,
        leaderboard_position: 0,
    };
    if !users.is_empty() {
        for (i, u) in users.iter().enumerate() {
            if user.id == u.id {
                profile.leaderboard_position = i as i32;
                break;
            }
        }
        profile.leaderboard_position += 1;
    }
    Ok(profile)
}

pub fn make_response(
    user: &User,
    attack_game: &[Game],
    defense_game: &[Game],
    users: &[User],
) -> Result<StatsResponse> {
    let mut stats = StatsResponse {
        highest_attack_score: 0,
        highest_defense_score: 0,
        trophies: user.trophies,
        position_in_leaderboard: 0,
        no_of_emps_used: 0,
        total_damage_defense: 0,
        total_damage_attack: 0,
        no_of_attackers_suicided: 0,
        no_of_attacks: attack_game.len() as i32,
        no_of_defenses: defense_game.len() as i32,
    };

    if !attack_game.is_empty() {
        stats.highest_attack_score = attack_game[0].attack_score;
        for attack in attack_game {
            stats.total_damage_attack += attack.damage_done;
            stats.no_of_emps_used += attack.emps_used;
            // if !attack.is_attacker_alive {
            //     stats.no_of_attackers_suicided += 1;
            // }
        }
    }
    if !defense_game.is_empty() {
        stats.highest_defense_score = defense_game[0].defend_score;
        for defend in defense_game {
            stats.total_damage_defense += defend.damage_done;
        }
    }
    if !users.is_empty() {
        for (i, u) in users.iter().enumerate() {
            if user.id == u.id {
                stats.position_in_leaderboard = i as i32;
                break;
            }
        }
        stats.position_in_leaderboard += 1;
    }
    Ok(stats)
}

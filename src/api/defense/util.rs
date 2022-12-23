/// CRUD functions
use super::MapSpacesEntry;
use crate::api;
use crate::api::util::GameHistoryEntry;
use crate::constants::{DEFENSE_END_TIME, DEFENSE_START_TIME};
use crate::models::*;
use crate::util::function;
use crate::{api::util::GameHistoryResponse, error::DieselError};
use anyhow::Result;
use chrono::{Local, NaiveTime};
use diesel::dsl::exists;
use diesel::{prelude::*, select};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize)]
pub struct DefenseResponse {
    pub map_spaces: Vec<MapSpaces>,
    pub blocks: Vec<BlockType>,
    pub levels_fixture: LevelsFixture,
    pub level_constraints: Vec<LevelConstraints>,
    pub attack_type: Vec<AttackType>,
}

#[derive(Deserialize, Serialize)]
pub struct DefenceHistoryResponse {
    pub games: Vec<Game>,
}

pub fn is_defense_allowed_now() -> bool {
    let start_time = NaiveTime::parse_from_str(DEFENSE_START_TIME, "%H:%M:%S").unwrap();
    let end_time = NaiveTime::parse_from_str(DEFENSE_END_TIME, "%H:%M:%S").unwrap();
    let current_time = Local::now().naive_local().time();
    current_time >= start_time || current_time <= end_time
}

pub fn defender_exists(defender: i32, conn: &PgConnection) -> Result<bool> {
    use crate::schema::user;

    Ok(select(exists(user::table.filter(user::id.eq(defender))))
        .get_result(conn)
        .map_err(|err| DieselError {
            table: "user",
            function: function!(),
            error: err,
        })?)
}

pub fn fetch_map_layout(conn: &PgConnection, player: &i32) -> Result<MapLayout> {
    use crate::schema::map_layout;

    let level_id = &api::util::get_current_levels_fixture(conn)?.id;
    let layout = map_layout::table
        .filter(map_layout::player.eq(player))
        .filter(map_layout::level_id.eq(level_id))
        .first::<MapLayout>(conn)
        .optional()
        .map_err(|err| DieselError {
            table: "map_layout",
            function: function!(),
            error: err,
        })?;

    if let Some(layout) = layout {
        Ok(layout)
    } else {
        let new_map_layout = NewMapLayout { player, level_id };
        Ok(diesel::insert_into(map_layout::table)
            .values(&new_map_layout)
            .get_result(conn)
            .map_err(|err| DieselError {
                table: "map_layout",
                function: function!(),
                error: err,
            })?)
    }
}

pub fn fetch_map_layout_from_game(conn: &PgConnection, game_id: i32) -> Result<Option<MapLayout>> {
    use crate::schema::{game, map_layout};

    let map_layout_id = game::table
        .select(game::map_layout_id)
        .find(game_id)
        .first::<i32>(conn)
        .optional()
        .map_err(|err| DieselError {
            table: "game",
            function: function!(),
            error: err,
        })?;

    if let Some(map_layout_id) = map_layout_id {
        let map_layout = map_layout::table
            .find(map_layout_id)
            .first::<MapLayout>(conn)
            .map_err(|err| DieselError {
                table: "map_layout",
                function: function!(),
                error: err,
            })?;
        Ok(Some(map_layout))
    } else {
        Ok(None)
    }
}

pub fn get_details_from_map_layout(conn: &PgConnection, map: MapLayout) -> Result<DefenseResponse> {
    use crate::schema::{attack_type, level_constraints, levels_fixture, map_spaces};

    let map_spaces = map_spaces::table
        .filter(map_spaces::map_id.eq(map.id))
        .load::<MapSpaces>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?;
    let blocks: Vec<BlockType> = fetch_blocks(conn)?;
    let levels_fixture = levels_fixture::table
        .find(map.level_id)
        .first::<LevelsFixture>(conn)
        .map_err(|err| DieselError {
            table: "levels_fixture",
            function: function!(),
            error: err,
        })?;
    let level_constraints = level_constraints::table
        .filter(level_constraints::level_id.eq(map.level_id))
        .load::<LevelConstraints>(conn)
        .map_err(|err| DieselError {
            table: "level_constraints",
            function: function!(),
            error: err,
        })?;
    let attack_type = attack_type::table
        .load::<AttackType>(conn)
        .map_err(|err| DieselError {
            table: "attack_type",
            function: function!(),
            error: err,
        })?;
    Ok(DefenseResponse {
        map_spaces,
        blocks,
        levels_fixture,
        level_constraints,
        attack_type,
    })
}

pub fn fetch_blocks(conn: &PgConnection) -> Result<Vec<BlockType>> {
    use crate::schema::block_type::dsl::*;

    Ok(block_type
        .load::<BlockType>(conn)
        .map_err(|err| DieselError {
            table: "block_type",
            function: function!(),
            error: err,
        })?)
}

pub fn put_base_details(
    maps: &[MapSpacesEntry],
    map: &MapLayout,
    conn: &PgConnection,
) -> Result<()> {
    use crate::schema::map_spaces::dsl::*;

    diesel::delete(map_spaces)
        .filter(map_id.eq(map.id))
        .execute(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?;
    let m: Vec<NewMapSpaces> = maps
        .iter()
        .map(|e| NewMapSpaces {
            map_id: map.id,
            blk_type: e.blk_type,
            x_coordinate: e.x_coordinate,
            y_coordinate: e.y_coordinate,
            rotation: e.rotation,
        })
        .collect();
    diesel::insert_into(map_spaces)
        .values(m)
        .on_conflict_do_nothing()
        .execute(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?;

    Ok(())
}

pub fn get_level_constraints(conn: &PgConnection, map_level_id: i32) -> Result<HashMap<i32, i32>> {
    use crate::schema::level_constraints::dsl::*;

    Ok(level_constraints
        .filter(level_id.eq(map_level_id))
        .load::<LevelConstraints>(conn)
        .map_err(|err| DieselError {
            table: "level_constraints",
            function: function!(),
            error: err,
        })?
        .iter()
        .map(|constraint| (constraint.block_id, constraint.no_of_buildings))
        .collect())
}

pub fn set_map_valid(conn: &PgConnection, map_id: i32) -> Result<()> {
    use crate::schema::map_layout::dsl::*;

    diesel::update(map_layout.find(map_id))
        .set(is_valid.eq(true))
        .execute(conn)
        .map_err(|err| DieselError {
            table: "map_layout",
            function: function!(),
            error: err,
        })?;

    Ok(())
}

pub fn set_map_invalid(conn: &PgConnection, map_id: i32) -> Result<()> {
    use crate::schema::map_layout::dsl::*;

    diesel::update(map_layout.find(map_id))
        .set(is_valid.eq(false))
        .execute(conn)
        .map_err(|err| DieselError {
            table: "map_layout",
            function: function!(),
            error: err,
        })?;

    Ok(())
}

pub fn fetch_defense_history(
    defender_id: i32,
    user_id: i32,
    conn: &PgConnection,
) -> Result<GameHistoryResponse> {
    use crate::schema::{game, levels_fixture, map_layout};

    let joined_table = game::table.inner_join(map_layout::table.inner_join(levels_fixture::table));
    let games_result: Result<Vec<GameHistoryEntry>> = joined_table
        .filter(game::defend_id.eq(defender_id))
        .load::<(Game, (MapLayout, LevelsFixture))>(conn)?
        .into_iter()
        .map(|(game, (_, levels_fixture))| {
            let is_replay_available = api::util::can_show_replay(user_id, &game, &levels_fixture);
            let player_name = api::util::get_username(game.attack_id, conn)?;
            Ok(GameHistoryEntry {
                game,
                player_name,
                is_replay_available,
            })
        })
        .collect();
    let games = games_result?;
    Ok(GameHistoryResponse { games })
}

pub fn fetch_top_defenses(user_id: i32, conn: &PgConnection) -> Result<GameHistoryResponse> {
    use crate::schema::{game, levels_fixture, map_layout};

    let joined_table = game::table.inner_join(map_layout::table.inner_join(levels_fixture::table));
    let games_result: Result<Vec<GameHistoryEntry>> = joined_table
        .order_by(game::defend_score.desc())
        .limit(10)
        .load::<(Game, (MapLayout, LevelsFixture))>(conn)?
        .into_iter()
        .map(|(game, (_, levels_fixture))| {
            let is_replay_available = api::util::can_show_replay(user_id, &game, &levels_fixture);
            let player_name = api::util::get_username(game.defend_id, conn)?;
            Ok(GameHistoryEntry {
                game,
                player_name,
                is_replay_available,
            })
        })
        .collect();
    let games = games_result?;
    Ok(GameHistoryResponse { games })
}

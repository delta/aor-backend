/// CRUD functions
use super::MapSpacesEntry;
use crate::api::auth::LoginResponse;
use crate::api::error::AuthError;
use crate::api::game::util::UserDetail;
use crate::api::user::util::fetch_user;
use crate::api::util::GameHistoryEntry;
use crate::api::util::{HistoryboardEntry, HistoryboardResponse};
use crate::api::{self};
use crate::constants::{BANK_BUILDING_NAME, INITIAL_ARTIFACTS, INITIAL_RATING, ROAD_ID};
use crate::models::*;
use crate::schema::{available_emps, map_layout, map_spaces, prop};
use crate::util::function;
use crate::{api::util::GameHistoryResponse, error::DieselError};
use anyhow::{Ok, Result};
use diesel::dsl::exists;
use diesel::{prelude::*, select};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Clone)]
pub struct MapSpacesResponseWithArifacts {
    pub id: i32,
    pub x_coordinate: i32,
    pub y_coordinate: i32,
    pub block_type_id: i32,
    pub artifacts: Option<i32>,
}

#[derive(Serialize)]
pub struct Artifact {
    pub id: i32,
    pub count: i32,
}

#[derive(Serialize)]
pub struct MineTypeResponseWithoutBlockId {
    pub id: i32,
    pub radius: i32,
    pub damage: i32,
    pub level: i32,
    pub cost: i32,
    pub name: String,
}

#[derive(Serialize, Clone)]
pub struct MineTypeResponse {
    pub id: i32,
    pub radius: i32,
    pub damage: i32,
    pub block_id: i32,
    pub level: i32,
    pub cost: i32,
    pub name: String,
}

#[derive(Serialize, Clone)]
pub struct DefenderTypeResponse {
    pub id: i32,
    pub radius: i32,
    pub speed: i32,
    pub damage: i32,
    pub block_id: i32,
    pub level: i32,
    pub cost: i32,
    pub name: String,
    pub max_health: i32,
}

#[derive(Serialize, Clone)]
pub struct BuildingTypeResponse {
    pub id: i32,
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub level: i32,
    pub cost: i32,
    pub capacity: i32,
    pub block_id: i32,
    pub hp: i32,
    pub range: i32,
    pub frequency: i32,
}

#[derive(Serialize)]
pub struct DefenseBaseResponse {
    pub user: Option<LoginResponse>,
    pub map_spaces: Vec<MapSpacesResponseWithArifacts>,
    pub blocks: Vec<BuildingTypeResponse>,
    pub defender_types: Vec<DefenderTypeResponse>,
    pub mine_types: Vec<MineTypeResponse>,
}

#[derive(Serialize)]
pub struct AttackBaseResponse {
    pub map_spaces: Vec<MapSpaces>,
    pub blocks: Vec<BuildingTypeResponse>,
    pub defender_types: Vec<DefenderTypeResponse>,
    pub mine_types: Vec<MineTypeResponseWithoutBlockId>,
}

#[derive(Serialize, Clone)]
pub struct SimulationBaseResponse {
    pub m: i32,                                 //map_id
    pub ms: Vec<MapSpacesResponseWithArifacts>, //map_spaces
    pub b: Vec<BuildingTypeResponse>,           //blocks
    pub d: Vec<DefenderTypeResponse>,           //defender_types
    pub mt: Vec<MineTypeResponse>,              //mine_types
    pub at: Vec<AttackerType>,                  //attacker_types
    pub bt: Vec<EmpType>,                       //bomb_types
}

#[derive(Serialize)]
pub struct DefenseResponse {
    pub map_spaces: Vec<MapSpaces>,
    pub blocks: Vec<BuildingTypeResponse>,
    pub levels_fixture: LevelsFixture,
    pub level_constraints: Vec<LevelConstraints>,
    pub bomb_types: Vec<EmpType>,
    pub defender_types: Vec<DefenderTypeResponse>,
    pub mine_types: Vec<MineTypeResponse>,
    pub attacker_types: Vec<AttackerType>,
    pub user: Option<LoginResponse>,
    pub is_map_valid: bool,
}

#[derive(Deserialize, Serialize)]
pub struct DefenceHistoryResponse {
    pub games: Vec<Game>,
}

pub fn check_valid_map_id(
    conn: &mut PgConnection,
    player: &i32,
    map_space_id: &i32,
) -> Result<i32> {
    use crate::schema::{map_layout, map_spaces};

    let map_layout_id = map_layout::table
        .filter(map_layout::player.eq(player))
        .inner_join(map_spaces::table.on(map_layout::id.eq(map_spaces::map_id)))
        .filter(map_spaces::id.eq(map_space_id))
        .select(map_layout::id)
        .first::<i32>(conn)
        .map_err(|err| DieselError {
            table: "map_layout",
            function: function!(),
            error: err,
        })?;
    Ok(map_layout_id)
}

pub fn defender_exists(defender: i32, conn: &mut PgConnection) -> Result<bool> {
    use crate::schema::user;

    Ok(select(exists(user::table.filter(user::id.eq(defender))))
        .get_result(conn)
        .map_err(|err| DieselError {
            table: "user",
            function: function!(),
            error: err,
        })?)
}

pub fn transfer_artifacts_building(
    conn: &mut PgConnection,
    building_map_space_id: &i32,
    bank_map_space_id: &i32,
    new_building_artifact_count: &i32,
    new_bank_artifact_count: &i32,
) -> Result<()> {
    use crate::schema::artifact;

    diesel::update(artifact::table.filter(artifact::map_space_id.eq(bank_map_space_id)))
        .set(artifact::count.eq(new_bank_artifact_count))
        .execute(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })
        .unwrap();

    if *new_building_artifact_count == 0 {
        diesel::delete(
            artifact::dsl::artifact.filter(artifact::dsl::map_space_id.eq(building_map_space_id)),
        )
        .execute(conn)
        .map_err(|err| DieselError {
            table: "shortest_path",
            function: function!(),
            error: err,
        })?;
        Ok(())
    } else {
        diesel::update(artifact::table.filter(artifact::map_space_id.eq(building_map_space_id)))
            .set(artifact::count.eq(new_building_artifact_count))
            .execute(conn)
            .map_err(|err| DieselError {
                table: "map_spaces",
                function: function!(),
                error: err,
            })
            .unwrap();
        Ok(())
    }
}

pub fn create_artifact_record(
    conn: &mut PgConnection,
    map_space_id: &i32,
    artifact_count: &i32,
) -> Result<()> {
    use crate::schema::artifact;

    let new_artifact = NewArtifact {
        map_space_id: *map_space_id,
        count: *artifact_count,
    };

    diesel::insert_into(artifact::table)
        .values(&new_artifact)
        .execute(conn)
        .map_err(|err| DieselError {
            table: "artifact",
            function: function!(),
            error: err,
        })?;

    Ok(())
}

pub fn get_block_id_of_bank(conn: &mut PgConnection, player: &i32) -> Result<i32> {
    use crate::schema::{block_type, building_type};
    let bank_block_type_id = map_spaces::table
        .inner_join(map_layout::table.on(map_layout::id.eq(map_spaces::map_id)))
        .filter(map_layout::player.eq(player))
        .inner_join(block_type::table.on(block_type::id.eq(map_spaces::block_type_id)))
        .filter(block_type::category.eq(BlockCategory::Building))
        .inner_join(building_type::table.on(building_type::id.eq(block_type::building_type)))
        .filter(building_type::name.eq(BANK_BUILDING_NAME))
        .select(block_type::id)
        .first::<i32>(conn)
        .map_err(|err| DieselError {
            table: "block_type",
            function: function!(),
            error: err,
        })?;

    Ok(bank_block_type_id)
}

pub fn get_bank_map_space_id(
    conn: &mut PgConnection,
    filtered_layout_id: &i32,
    bank_block_type_id: &i32,
) -> Result<i32> {
    use crate::schema::map_spaces;
    let fetched_bank_map_space_id = map_spaces::table
        .filter(map_spaces::map_id.eq(filtered_layout_id))
        .filter(map_spaces::block_type_id.eq(bank_block_type_id))
        .select(map_spaces::id)
        .first::<i32>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?;
    Ok(fetched_bank_map_space_id)
}

pub fn check_valid_map_space_building(
    conn: &mut PgConnection,
    given_map_space_id: &i32,
) -> Result<bool> {
    use crate::schema::{block_type, building_type, map_spaces};

    let category_type = map_spaces::table
        .filter(map_spaces::id.eq(given_map_space_id))
        .inner_join(block_type::table.on(map_spaces::block_type_id.eq(block_type::id)))
        .select(block_type::category)
        .first::<BlockCategory>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?;

    if category_type != BlockCategory::Building {
        return Ok(false);
    }

    let builiding_type_id = map_spaces::table
        .filter(map_spaces::id.eq(given_map_space_id))
        .inner_join(block_type::table.on(map_spaces::block_type_id.eq(block_type::id)))
        .inner_join(building_type::table.on(block_type::building_type.eq(building_type::id)))
        .select(building_type::id)
        .first::<i32>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?;

    if builiding_type_id == ROAD_ID {
        return Ok(false);
    }
    Ok(true)
}

pub fn get_building_artifact_count(
    conn: &mut PgConnection,
    filtered_layout_id: &i32,
    given_map_space_id: &i32,
) -> Result<i32> {
    use crate::schema::{artifact, map_spaces};
    let building_artifact_count = map_spaces::table
        .inner_join(artifact::table)
        .filter(map_spaces::map_id.eq(filtered_layout_id)) //Eg:1
        .filter(map_spaces::id.eq(given_map_space_id))
        .select(artifact::count)
        .first::<i32>(conn)
        .unwrap_or(-1);
    Ok(building_artifact_count)
}

pub fn get_building_capacity(conn: &mut PgConnection, given_map_space_id: &i32) -> Result<i32> {
    use crate::schema::{block_type, building_type, map_spaces};
    let building_capacity = map_spaces::table
        .filter(map_spaces::id.eq(given_map_space_id))
        .inner_join(block_type::table.on(map_spaces::block_type_id.eq(block_type::id)))
        .inner_join(building_type::table.on(block_type::building_type.eq(building_type::id)))
        .select(building_type::capacity)
        .first::<i32>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?;
    Ok(building_capacity)
}

pub fn fetch_map_layout(conn: &mut PgConnection, player: &i32) -> Result<MapLayout> {
    use crate::schema::map_layout;

    let layout = map_layout::table
        .filter(map_layout::player.eq(player))
        .first::<MapLayout>(conn)
        .map_err(|err| DieselError {
            table: "map_layout",
            function: function!(),
            error: err,
        })?;

    Ok(layout)
}

pub fn fetch_map_layout_from_game(
    conn: &mut PgConnection,
    game_id: i32,
) -> Result<Option<MapLayout>> {
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

pub fn get_details_from_map_layout(
    conn: &mut PgConnection,
    map: MapLayout,
    user: Option<User>,
) -> Result<DefenseBaseResponse> {
    use crate::schema::{artifact, map_spaces};

    let map_spaces: Vec<MapSpacesResponseWithArifacts> = map_spaces::table
        .left_join(artifact::table)
        .filter(map_spaces::map_id.eq(map.id))
        .select((map_spaces::all_columns, artifact::count.nullable()))
        .load::<(MapSpaces, Option<i32>)>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(map_space, count)| MapSpacesResponseWithArifacts {
            id: map_space.id,
            x_coordinate: map_space.x_coordinate,
            y_coordinate: map_space.y_coordinate,
            block_type_id: map_space.block_type_id,
            artifacts: count,
        })
        .collect();

    let blocks = fetch_building_blocks(conn, &map.player)?;

    let mine_types = fetch_mine_types(conn, &map.player)?;
    let defender_types = fetch_defender_types(conn, &map.player)?;
    let user_response = if let Some(user) = user {
        Some(LoginResponse {
            user_id: user.id,
            username: user.username,
            name: user.name,
            avatar_id: user.avatar_id,
            attacks_won: user.attacks_won,
            defenses_won: user.defenses_won,
            trophies: user.trophies,
            artifacts: user.artifacts,
            email: user.email,
            token: None,
        })
    } else {
        None
    };

    Ok(DefenseBaseResponse {
        map_spaces,
        blocks,
        mine_types,
        defender_types,
        user: user_response,
    })
}

pub fn get_map_details_for_attack(
    conn: &mut PgConnection,
    map: MapLayout,
    attacker_id: i32,
) -> Result<DefenseResponse> {
    use crate::schema::{block_type, emp_type, level_constraints, levels_fixture, map_spaces};

    let map_spaces = map_spaces::table
        .inner_join(block_type::table)
        .filter(map_spaces::map_id.eq(map.id))
        .load::<(MapSpaces, BlockType)>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(mut map_space, block_type)| {
            if block_type.category == BlockCategory::Mine {
                map_space.block_type_id = ROAD_ID;
                map_space
            } else {
                map_space
            }
            // if block_type.building_type == ROAD_ID {
            //     if block_type.category == BlockCategory::Mine {
            //         map_space.block_type_id = ROAD_ID;
            //     }
            //     map_space
            // } else {
            //     map_space
            // }
        })
        .collect();
    let blocks = fetch_building_blocks(conn, &map.player)?;
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

    let bomb_types = emp_type::table
        .inner_join(available_emps::table.on(available_emps::emp_type_id.eq(emp_type::id)))
        .filter(available_emps::user_id.eq(&attacker_id))
        .load::<(EmpType, AvailableEmps)>(conn)
        .map_err(|err| DieselError {
            table: "emp_type",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(emp_type, _)| emp_type)
        .collect();

    let mine_types = fetch_mine_types(conn, &map.player)?;
    let defender_types = fetch_defender_types(conn, &map.player)?;
    let attacker_types = fetch_attacker_types(conn, &attacker_id)?;

    Ok(DefenseResponse {
        map_spaces,
        blocks,
        levels_fixture,
        level_constraints,
        bomb_types,
        mine_types,
        defender_types,
        attacker_types,
        user: None,
        is_map_valid: map.is_valid,
    })
}

pub fn get_map_details_for_simulation(
    conn: &mut PgConnection,
    map: MapLayout,
) -> Result<SimulationBaseResponse> {
    use crate::schema::{artifact, emp_type, map_spaces};

    let map_spaces: Vec<MapSpacesResponseWithArifacts> = map_spaces::table
        .left_join(artifact::table)
        .filter(map_spaces::map_id.eq(map.id))
        .select((map_spaces::all_columns, artifact::count.nullable()))
        .load::<(MapSpaces, Option<i32>)>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(map_space, count)| MapSpacesResponseWithArifacts {
            id: map_space.id,
            x_coordinate: map_space.x_coordinate,
            y_coordinate: map_space.y_coordinate,
            block_type_id: map_space.block_type_id,
            artifacts: count,
        })
        .collect();

    let blocks = fetch_building_blocks(conn, &map.player)?;

    let bomb_types = emp_type::table
        .inner_join(available_emps::table.on(available_emps::emp_type_id.eq(emp_type::id)))
        .filter(available_emps::user_id.eq(&map.player))
        .load::<(EmpType, AvailableEmps)>(conn)
        .map_err(|err| DieselError {
            table: "emp_type",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(emp_type, _)| emp_type)
        .collect();

    let mine_types = fetch_mine_types(conn, &map.player)?;
    let defender_types = fetch_defender_types(conn, &map.player)?;
    let attacker_types = fetch_attacker_types(conn, &map.player)?;

    Ok(SimulationBaseResponse {
        m: map.id,
        ms: map_spaces,
        b: blocks,
        bt: bomb_types,
        mt: mine_types,
        d: defender_types,
        at: attacker_types,
    })
}

pub fn fetch_buildings(conn: &mut PgConnection) -> Result<Vec<BuildingType>> {
    use crate::schema::building_type::dsl::*;

    Ok(building_type
        .load::<BuildingType>(conn)
        .map_err(|err| DieselError {
            table: "building_type",
            function: function!(),
            error: err,
        })?)
}

pub fn put_base_details(
    maps: &[MapSpacesEntry],
    map: &MapLayout,
    conn: &mut PgConnection,
) -> Result<()> {
    use crate::schema::artifact;
    use crate::schema::map_spaces::dsl::*;

    diesel::delete(artifact::table)
        .filter(artifact::map_space_id.eq_any(map_spaces.filter(map_id.eq(map.id)).select(id)))
        .execute(conn)
        .map_err(|err| DieselError {
            table: "artifact",
            function: function!(),
            error: err,
        })?;

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
            x_coordinate: e.x_coordinate,
            y_coordinate: e.y_coordinate,
            block_type_id: e.block_type_id,
        })
        .collect();

    let result: Vec<MapSpaces> = diesel::insert_into(map_spaces)
        .values(m)
        .on_conflict_do_nothing()
        .get_results(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?;

    let mut map_space_map: HashMap<(i32, i32), i32> = HashMap::new();
    for map_space in result {
        map_space_map.insert(
            (map_space.x_coordinate, map_space.y_coordinate),
            map_space.id,
        );
    }

    let artifact_entries: Vec<NewArtifact> = maps
        .iter()
        .filter_map(|e| {
            if e.artifacts > 0 {
                Some(NewArtifact {
                    map_space_id: map_space_map[&(e.x_coordinate, e.y_coordinate)],
                    count: e.artifacts,
                })
            } else {
                None
            }
        })
        .collect();

    diesel::insert_into(artifact::table)
        .values(artifact_entries)
        .on_conflict_do_nothing()
        .execute(conn)
        .map_err(|err| DieselError {
            table: "artifact",
            function: function!(),
            error: err,
        })?;

    Ok(())
}

pub fn get_level_constraints(
    conn: &mut PgConnection,
    map_level_id: i32,
    user_id: &i32,
) -> Result<HashMap<i32, i32>> {
    use crate::schema::{block_type, level_constraints, map_layout, map_spaces};

    Ok(map_spaces::table
        .inner_join(map_layout::table.on(map_layout::id.eq(map_spaces::map_id)))
        .inner_join(
            block_type::table
                .on(map_spaces::block_type_id.eq(block_type::id))
                .inner_join(
                    level_constraints::table.on(level_constraints::block_id.eq(block_type::id)),
                ),
        )
        .filter(map_layout::player.eq(user_id))
        .filter(level_constraints::level_id.eq(map_level_id))
        .load::<(MapSpaces, MapLayout, (BlockType, LevelConstraints))>(conn)
        .map_err(|err| DieselError {
            table: "available_blocks",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(_, _, (_, constraint))| (constraint.block_id, constraint.no_of_blocks))
        .collect())
}

pub fn fetch_defense_historyboard(
    user_id: i32,
    page: i64,
    limit: i64,
    conn: &mut PgConnection,
) -> Result<HistoryboardResponse> {
    use crate::schema::{game, levels_fixture, map_layout, user};

    let joined_table = game::table
        .filter(game::defend_id.eq(user_id))
        .inner_join(map_layout::table.inner_join(levels_fixture::table))
        .inner_join(user::table.on(game::attack_id.eq(user::id)));

    let total_entries: i64 = joined_table
        .count()
        .get_result(conn)
        .map_err(|err| DieselError {
            table: "game",
            function: function!(),
            error: err,
        })?;
    let off_set: i64 = (page - 1) * limit;
    let last_page: i64 = (total_entries as f64 / limit as f64).ceil() as i64;

    let games_result: Result<Vec<HistoryboardEntry>> = joined_table
        .offset(off_set)
        .limit(limit)
        .load::<(Game, (MapLayout, LevelsFixture), User)>(conn)
        .map_err(|err| DieselError {
            table: "game",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(game, (_, levels_fixture), user)| {
            let is_replay_available = api::util::can_show_replay(user_id, &game, &levels_fixture);
            Ok(HistoryboardEntry {
                opponent_user_name: user.username.to_string(),
                is_attack: false,
                damage_percent: game.damage_done,
                artifacts_taken: -game.artifacts_collected,
                trophies_taken: game.defend_score,
                match_id: game.id,
                replay_availability: is_replay_available,
                avatar_id: user.avatar_id,
            })
        })
        .collect();
    let games = games_result?;
    Ok(HistoryboardResponse { games, last_page })
}

pub fn fetch_top_defenses(user_id: i32, conn: &mut PgConnection) -> Result<GameHistoryResponse> {
    use crate::schema::{game, levels_fixture, map_layout};

    let joined_table = game::table.inner_join(map_layout::table.inner_join(levels_fixture::table));
    let games_result: Result<Vec<GameHistoryEntry>> = joined_table
        .order_by(game::defend_score.desc())
        .limit(10)
        .load::<(Game, (MapLayout, LevelsFixture))>(conn)
        .map_err(|err| DieselError {
            table: "game",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(game, (_, levels_fixture))| {
            let is_replay_available = api::util::can_show_replay(user_id, &game, &levels_fixture);
            let attacker = fetch_user(conn, game.attack_id)?.ok_or(AuthError::UserNotFound)?;
            let defender = fetch_user(conn, game.defend_id)?.ok_or(AuthError::UserNotFound)?;
            Ok(GameHistoryEntry {
                game,
                attacker: UserDetail {
                    user_id: attacker.id,
                    username: attacker.username,
                    trophies: attacker.trophies,
                    avatar_id: attacker.avatar_id,
                },
                defender: UserDetail {
                    user_id: defender.id,
                    username: defender.username,
                    trophies: defender.trophies,
                    avatar_id: defender.avatar_id,
                },
                is_replay_available,
            })
        })
        .collect();
    let games = games_result?;
    Ok(GameHistoryResponse { games })
}

pub fn fetch_mine_types(conn: &mut PgConnection, user_id: &i32) -> Result<Vec<MineTypeResponse>> {
    use crate::schema::block_type;
    use crate::schema::mine_type;
    use crate::schema::{map_layout, map_spaces};

    let mines: Result<Vec<MineTypeResponse>> = map_spaces::table
        .inner_join(map_layout::table.on(map_layout::id.eq(map_spaces::map_id)))
        .filter(map_layout::player.eq(user_id))
        .inner_join(block_type::table.on(block_type::id.eq(map_spaces::block_type_id)))
        .filter(block_type::category.eq(BlockCategory::Mine))
        .inner_join(mine_type::table.on(mine_type::id.eq(block_type::mine_type.assume_not_null())))
        .inner_join(prop::table.on(mine_type::prop_id.eq(prop::id)))
        .load::<(MapSpaces, MapLayout, BlockType, MineType, Prop)>(conn)
        .map_err(|err| DieselError {
            table: "mine_type",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(_, _, block_type, mine_type, prop)| {
            Ok(MineTypeResponse {
                id: mine_type.id,
                radius: prop.range,
                damage: mine_type.damage,
                block_id: block_type.id,
                cost: mine_type.cost,
                level: mine_type.level,
                name: mine_type.name,
            })
        })
        .collect();
    mines
}

pub fn fetch_defender_types(
    conn: &mut PgConnection,
    user_id: &i32,
) -> Result<Vec<DefenderTypeResponse>> {
    use crate::schema::{block_type, defender_type};

    let defenders: Result<Vec<DefenderTypeResponse>> = map_spaces::table
        .inner_join(map_layout::table.on(map_layout::id.eq(map_spaces::map_id)))
        .filter(map_layout::player.eq(user_id))
        .inner_join(block_type::table.on(map_spaces::block_type_id.eq(block_type::id)))
        .filter(block_type::category.eq(BlockCategory::Defender))
        .inner_join(defender_type::table.on(block_type::defender_type.assume_not_null().eq(defender_type::id)))
        .inner_join(prop::table.on(prop::id.eq(defender_type::prop_id)))
        .load::<(MapSpaces, MapLayout, BlockType, DefenderType, Prop)>(conn)
        .map_err(|err| DieselError {
            table: "defender_type",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(_, _, block_type, defender_type, prop)| {
            Ok(DefenderTypeResponse {
                id: defender_type.id,
                radius: prop.range,
                speed: defender_type.speed,
                damage: defender_type.damage,
                block_id: block_type.id,
                level: defender_type.level,
                cost: defender_type.cost,
                name: defender_type.name,
                max_health: defender_type.max_health,
            })
        })
        .collect();
    defenders
}

pub fn fetch_building_blocks(
    conn: &mut PgConnection,
    user_id: &i32,
) -> Result<Vec<BuildingTypeResponse>> {
    use crate::schema::{block_type, building_type, map_layout, map_spaces};

    let buildings: Vec<BuildingTypeResponse> = map_spaces::table
        .inner_join(map_layout::table.on(map_layout::id.eq(map_spaces::map_id)))
        .filter(map_layout::player.eq(user_id))
        .inner_join(block_type::table)
        .filter(block_type::category.eq(BlockCategory::Building))
        .inner_join(building_type::table.on(block_type::building_type.eq(building_type::id)))
        .inner_join(prop::table.on(building_type::prop_id.eq(prop::id)))
        .load::<(MapSpaces, MapLayout, BlockType, BuildingType, Prop)>(conn)
        .map_err(|err| DieselError {
            table: "block_type",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(
            |(_, _, block_type, building_type, prop)| BuildingTypeResponse {
                id: building_type.id,
                name: building_type.name,
                width: building_type.width,
                height: building_type.height,
                level: building_type.level,
                cost: building_type.cost,
                capacity: building_type.capacity,
                block_id: block_type.id,
                hp: building_type.hp,
                range: prop.range,
                frequency: prop.frequency,
            },
        )
        .collect();
    Ok(buildings)
}

pub fn fetch_blocks(conn: &mut PgConnection, user_id: &i32) -> Result<HashMap<i32, BlockType>> {
    use crate::schema::{block_type, map_layout, map_spaces};

    Ok(map_spaces::table
        .inner_join(map_layout::table.on(map_layout::id.eq(map_spaces::map_id)))
        .filter(map_layout::player.eq(user_id))
        .inner_join(block_type::table.on(map_spaces::block_type_id.eq(block_type::id)))
        .load::<(MapSpaces, MapLayout, BlockType)>(conn)
        .map_err(|err| DieselError {
            table: "available_blocks",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(_, _, block_type)| {
            (
                block_type.id,
                BlockType {
                    id: block_type.id,
                    mine_type: block_type.mine_type,
                    defender_type: block_type.defender_type,
                    category: block_type.category,
                    building_type: block_type.building_type,
                },
            )
        })
        .collect::<HashMap<i32, BlockType>>())
}

pub fn fetch_attacker_types(conn: &mut PgConnection, user_id: &i32) -> Result<Vec<AttackerType>> {
    use crate::schema::{attacker_type, available_attackers};
    let results: Vec<AttackerType> = attacker_type::table
        .inner_join(available_attackers::table)
        .filter(available_attackers::user_id.eq(user_id))
        .load::<(AttackerType, AvailableAttackers)>(conn)
        .map_err(|err| DieselError {
            table: "attacker_type",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(attacker_type, _)| attacker_type)
        .collect();

    Ok(results)
}

pub fn add_user_default_base(
    conn: &mut PgConnection,
    user_name: &str,
    user_email: &str,
) -> Result<User> {
    conn.transaction(|conn| {
        use crate::schema::{
            artifact, available_attackers, available_emps, map_layout, map_spaces, user,
        };

        let username = user_email.split('@').next().unwrap();
        let new_user = NewUser {
            name: user_name,
            email: user_email,
            username,
            is_pragyan: &false,
            attacks_won: &0,
            defenses_won: &0,
            trophies: &INITIAL_RATING,
            avatar_id: &rand::thread_rng().gen_range(0..=7),
            artifacts: &INITIAL_ARTIFACTS,
        };

        let user: User = diesel::insert_into(user::table)
            .values(&new_user)
            .get_result(conn)
            .map_err(|err| DieselError {
                table: "user",
                function: function!(),
                error: err,
            })?;

        let bot_user_id = user::table
            .filter(user::is_pragyan.eq(true))
            .select(user::id)
            .first::<i32>(conn)
            .map_err(|err| DieselError {
                table: "user",
                function: function!(),
                error: err,
            })?;

        let level_id: &i32 = &api::util::get_current_levels_fixture(conn)?.id;

        let new_map_layout = NewMapLayout {
            player: &user.id,
            level_id,
            is_valid: &true,
        };

        let map_layout: MapLayout = diesel::insert_into(map_layout::table)
            .values(&new_map_layout)
            .get_result(conn)
            .map_err(|err| DieselError {
                table: "map_layout",
                function: function!(),
                error: err,
            })?;

        let joined_table = user::table
            .filter(user::id.eq(bot_user_id))
            .inner_join(map_layout::table.inner_join(map_spaces::table.left_join(artifact::table)));

        let mut artifact_map: HashMap<(i32, i32), i32> = HashMap::new();

        let new_map_spaces: Vec<NewMapSpaces> = joined_table
            .select((map_spaces::all_columns, artifact::count.nullable()))
            .load::<(MapSpaces, Option<i32>)>(conn)
            .map_err(|err| DieselError {
                table: "user",
                function: function!(),
                error: err,
            })?
            .into_iter()
            .map(|(map_space, count)| {
                if let Some(cnt) = count {
                    artifact_map.insert((map_space.x_coordinate, map_space.y_coordinate), cnt);
                }
                NewMapSpaces {
                    block_type_id: map_space.block_type_id,
                    map_id: map_layout.id,
                    x_coordinate: map_space.x_coordinate,
                    y_coordinate: map_space.y_coordinate,
                }
            })
            .collect();

        diesel::insert_into(map_spaces::table)
            .values(new_map_spaces)
            .on_conflict_do_nothing()
            .execute(conn)
            .map_err(|err| DieselError {
                table: "map_spaces",
                function: function!(),
                error: err,
            })?;

        let result: Vec<MapSpaces> = map_spaces::table
            .filter(map_spaces::map_id.eq(map_layout.id))
            .load::<MapSpaces>(conn)
            .map_err(|err| DieselError {
                table: "map_spaces",
                function: function!(),
                error: err,
            })?;

        let artifact_entries: Vec<NewArtifact> = result
            .iter()
            .filter_map(|m| {
                if artifact_map.contains_key(&(m.x_coordinate, m.y_coordinate)) {
                    Some(NewArtifact {
                        map_space_id: m.id,
                        count: artifact_map[&(m.x_coordinate, m.y_coordinate)],
                    })
                } else {
                    None
                }
            })
            .collect();

        diesel::insert_into(artifact::table)
            .values(artifact_entries)
            .on_conflict_do_nothing()
            .execute(conn)
            .map_err(|err| DieselError {
                table: "artifact",
                function: function!(),
                error: err,
            })?;

        //
        let new_available_attackers: Vec<NewAvailableAttackers> = available_attackers::table
            .filter(available_attackers::user_id.eq(bot_user_id))
            .load::<AvailableAttackers>(conn)
            .map_err(|err| DieselError {
                table: "available_attackers",
                function: function!(),
                error: err,
            })?
            .into_iter()
            .map(|available_attackers| NewAvailableAttackers {
                user_id: user.id,
                attacker_type_id: available_attackers.id,
            })
            .collect();

        diesel::insert_into(available_attackers::table)
            .values(new_available_attackers)
            .on_conflict_do_nothing()
            .execute(conn)
            .map_err(|err| DieselError {
                table: "available_attackers",
                function: function!(),
                error: err,
            })?;

        let new_available_emps: Vec<NewAvailableEmps> = available_emps::table
            .filter(available_emps::user_id.eq(bot_user_id))
            .load::<AvailableEmps>(conn)
            .map_err(|err| DieselError {
                table: "available_emps",
                function: function!(),
                error: err,
            })?
            .into_iter()
            .map(|available_emp| NewAvailableEmps {
                user_id: user.id,
                emp_type_id: available_emp.id,
            })
            .collect();

        diesel::insert_into(available_emps::table)
            .values(new_available_emps)
            .on_conflict_do_nothing()
            .execute(conn)
            .map_err(|err| DieselError {
                table: "available_emps",
                function: function!(),
                error: err,
            })?;
        Ok(user)
    })
}

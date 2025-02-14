use crate::api::attack::rating::new_rating;
use crate::api::auth::TokenClaims;
use crate::api::defense::util::{
    fetch_map_layout, get_map_details_for_attack, get_map_details_for_simulation,
    AttackBaseResponse, DefenseResponse, SimulationBaseResponse,
};
use crate::api::error::AuthError;
use crate::api::game::util::UserDetail;
use crate::api::inventory::util::{get_bank_map_space_id, get_block_id_of_bank, get_user_map_id};
use crate::api::user::util::fetch_user;
use crate::api::util::{
    GameHistoryEntry, GameHistoryResponse, HistoryboardEntry, HistoryboardResponse,
};
use crate::api::{self, RedisConn};
use crate::constants::*;
use crate::error::DieselError;
use crate::models::{
    Artifact, AttackerType, BlockCategory, BlockType, BuildingType, DefenderType, EmpType, Game,
    LevelsFixture, MapLayout, MapSpaces, MineType, NewAttackerPath, NewGame, Prop, User,
};
use crate::schema::{block_type, building_type, defender_type, map_layout, map_spaces, prop, user};
use crate::util::function;
use crate::validator::util::Coords;
use crate::validator::util::{BombType, BuildingDetails, DefenderDetails, MineDetails};
use ::serde::{Deserialize, Serialize};
use anyhow::Result;
use chrono;
use diesel::prelude::*;
use diesel::PgConnection;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::seq::IteratorRandom;
use redis::Commands;
use std::collections::{HashMap, HashSet};
use std::env;

use super::socket::BuildingResponse;

#[derive(Debug, Serialize)]
pub struct DefensePosition {
    pub y_coord: i32,
    pub x_coord: i32,
    pub block_category: BlockCategory,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NewAttack {
    pub defender_id: i32,
    pub no_of_attackers: i32,
    pub attackers: Vec<NewAttacker>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NewAttacker {
    pub attacker_type: i32,
    pub attacker_path: Vec<NewAttackerPath>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttackToken {
    pub game_id: i32,
    pub attacker_id: i32,
    pub defender_id: i32,
    pub iat: usize,
    pub exp: usize,
}
// #[derive(Serialize, Clone, Debug)]
// pub enum Direction {
//     Up,
//     Down,
//     Left,
//     Right,
// }

#[derive(Serialize, Clone, Debug)]
pub struct EventResponse {
    // pub attacker_initial_position: Option<Coords>,
    pub attacker_id: Option<i32>,
    pub bomb_id: Option<i32>,
    pub coords: Coords,
    // pub direction: Direction,
    pub is_bomb: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct ResultResponse {
    pub d: i32,  //damage_done
    pub a: i32,  //artifacts_collected
    pub b: i32,  //bombs_used
    pub au: i32, //attackers_used
    pub na: i32, //new_attacker_trophies
    pub nd: i32, //new_defender_trophies
    pub oa: i32, //old_attacker_trophies
    pub od: i32, //old_defender_trophies
}

#[derive(Serialize, Clone)]
pub struct GameLog {
    pub g: i32,                    //game_id
    pub a: User,                   //attacker
    pub d: User,                   //defender
    pub b: SimulationBaseResponse, //base
    pub e: Vec<EventResponse>,     //events
    pub r: ResultResponse,         //result
}

pub fn get_map_id(defender_id: &i32, conn: &mut PgConnection) -> Result<Option<i32>> {
    use crate::schema::map_layout;
    let map_id = map_layout::table
        .filter(map_layout::player.eq(defender_id))
        .filter(map_layout::is_valid.eq(true))
        .select(map_layout::id)
        .first::<i32>(conn)
        .optional()
        .map_err(|err| DieselError {
            table: "map_layout",
            function: function!(),
            error: err,
        })?;
    Ok(map_id)
}

pub fn get_valid_road_paths(map_id: i32, conn: &mut PgConnection) -> Result<HashSet<(i32, i32)>> {
    use crate::schema::{block_type, map_spaces};
    let valid_road_paths: HashSet<(i32, i32)> = map_spaces::table
        .inner_join(block_type::table)
        .filter(map_spaces::map_id.eq(map_id))
        .filter(block_type::building_type.eq(ROAD_ID))
        .select((map_spaces::x_coordinate, map_spaces::y_coordinate))
        .load::<(i32, i32)>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?
        .iter()
        .cloned()
        .collect();
    Ok(valid_road_paths)
}

pub fn add_game(
    attacker_id: i32,
    defender_id: i32,
    map_layout_id: i32,
    conn: &mut PgConnection,
) -> Result<i32> {
    use crate::schema::game;

    // insert in game table

    let new_game = NewGame {
        attack_id: &attacker_id,
        defend_id: &defender_id,
        map_layout_id: &map_layout_id,
        attack_score: &0,
        defend_score: &0,
        artifacts_collected: &0,
        damage_done: &0,
        emps_used: &0,
        is_game_over: &false,
        date: &chrono::Local::now().date_naive(),
    };

    let inserted_game: Game = diesel::insert_into(game::table)
        .values(&new_game)
        .get_result(conn)
        .map_err(|err| DieselError {
            table: "game",
            function: function!(),
            error: err,
        })?;

    Ok(inserted_game.id)
}

pub fn fetch_attack_history(
    user_id: i32,
    page: i64,
    limit: i64,
    conn: &mut PgConnection,
) -> Result<HistoryboardResponse> {
    use crate::schema::{game, levels_fixture, map_layout};
    let joined_table = game::table
        .filter(game::attack_id.eq(user_id))
        .inner_join(map_layout::table.inner_join(levels_fixture::table))
        .inner_join(user::table.on(game::defend_id.eq(user::id)));

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
        .load::<(Game, (MapLayout, LevelsFixture), User)>(conn)?
        .into_iter()
        .map(|(game, (_, levels_fixture), user)| {
            let is_replay_available = api::util::can_show_replay(user_id, &game, &levels_fixture);
            Ok(HistoryboardEntry {
                opponent_user_name: user.username.to_string(),
                is_attack: true,
                damage_percent: game.damage_done,
                artifacts_taken: game.artifacts_collected,
                trophies_taken: game.attack_score,
                match_id: game.id,
                replay_availability: is_replay_available,
                avatar_id: user.avatar_id,
            })
        })
        .collect();
    let games = games_result?;
    Ok(HistoryboardResponse { games, last_page })
}

pub fn fetch_top_attacks(user_id: i32, conn: &mut PgConnection) -> Result<GameHistoryResponse> {
    use crate::schema::{game, levels_fixture, map_layout};

    let joined_table = game::table
        .inner_join(map_layout::table.inner_join(levels_fixture::table))
        .inner_join(user::table.on(game::defend_id.eq(user::id)));
    let games_result: Result<Vec<GameHistoryEntry>> = joined_table
        .order_by(game::attack_score.desc())
        .limit(10)
        .load::<(Game, (MapLayout, LevelsFixture), User)>(conn)?
        .into_iter()
        .map(|(game, (_, levels_fixture), defender)| {
            let is_replay_available = api::util::can_show_replay(user_id, &game, &levels_fixture);
            let attacker = fetch_user(conn, game.attack_id)?.ok_or(AuthError::UserNotFound)?;
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

pub fn get_attacker_types(conn: &mut PgConnection) -> Result<HashMap<i32, AttackerType>> {
    use crate::schema::attacker_type::dsl::*;
    Ok(attacker_type
        .load::<AttackerType>(conn)
        .map_err(|err| DieselError {
            table: "attacker_type",
            function: function!(),
            error: err,
        })?
        .iter()
        .map(|attacker| {
            (
                attacker.id,
                AttackerType {
                    id: attacker.id,
                    name: attacker.name.clone(),
                    max_health: attacker.max_health,
                    speed: attacker.speed,
                    amt_of_emps: attacker.amt_of_emps,
                    level: attacker.level,
                    cost: attacker.cost,
                    prop_id: attacker.prop_id,
                },
            )
        })
        .collect::<HashMap<i32, AttackerType>>())
}

#[derive(Serialize)]
pub struct ShortestPathResponse {
    pub source: Coords,
    pub dest: Coords,
    pub next_hop: Coords,
}

#[derive(Serialize)]
pub struct AttackResponse {
    pub user: Option<User>,
    pub base: AttackBaseResponse,
    pub max_bombs: i32,
    pub attacker_types: Vec<AttackerType>,
    pub bomb_types: Vec<EmpType>,
    pub shortest_paths: Option<Vec<ShortestPathResponse>>,
    pub obtainable_artifacts: i32,
    pub attack_token: String,
    pub game_id: i32,
}

pub fn get_random_opponent_id(
    attacker_id: i32,
    conn: &mut PgConnection,
    mut redis_conn: RedisConn,
) -> Result<Option<i32>> {
    let sorted_users: Vec<(i32, i32)> = user::table
        .filter(user::is_pragyan.eq(false))
        .order_by(user::trophies.asc())
        .select((user::id, user::trophies))
        .load::<(i32, i32)>(conn)?;

    if let Some(attacker_index) = sorted_users.iter().position(|(id, _)| *id == attacker_id) {
        let less_or_equal_trophies = sorted_users
            .iter()
            .take(attacker_index)
            .filter(|(id, _)| *id != attacker_id)
            .rev()
            .take(10)
            .cloned()
            .collect::<Vec<_>>();
        let more_or_equal_trophies = sorted_users
            .iter()
            .skip(attacker_index + 1)
            .filter(|(id, _)| *id != attacker_id)
            .take(10)
            .cloned()
            .collect::<Vec<_>>();

        // While the opponent id is not present in redis, keep finding a new opponent
        let mut attempts: i32 = MATCH_MAKING_ATTEMPTS;
        let mut random_opponent = if let Ok(opponent) =
            get_random_opponent(&less_or_equal_trophies, &more_or_equal_trophies)
        {
            opponent
        } else {
            return Err(anyhow::anyhow!("Failed to find an opponent"));
        };

        loop {
            if let Ok(Some(_)) = get_game_id_from_redis(random_opponent, &mut redis_conn, false) {
                random_opponent =
                    match get_random_opponent(&less_or_equal_trophies, &more_or_equal_trophies) {
                        Ok(opponent) => opponent,
                        Err(_) => return Err(anyhow::anyhow!("Failed to find an opponent")),
                    };
            } else if let Ok(check) = can_attack_happen(conn, random_opponent, false) {
                if !check {
                    random_opponent =
                        match get_random_opponent(&less_or_equal_trophies, &more_or_equal_trophies)
                        {
                            Ok(opponent) => opponent,
                            Err(_) => {
                                return Err(anyhow::anyhow!("Failed to find another opponent"))
                            }
                        };
                } else {
                    return Ok(Some(random_opponent));
                }
            } else {
                return Err(anyhow::anyhow!("Cannot check if attack can happen now"));
            }

            attempts += 1;
            if attempts > 10 {
                return Err(anyhow::anyhow!(
                    "Failed to find an opponent despite many attempts"
                ));
            }
        }
    } else {
        Err(anyhow::anyhow!("Attacker id not found"))
    }
}

pub fn get_random_opponent(
    less_or_equal_trophies: &[(i32, i32)],
    more_or_equal_trophies: &[(i32, i32)],
) -> Result<i32> {
    if let Some(random_opponent) = less_or_equal_trophies
        .iter()
        .chain(more_or_equal_trophies.iter())
        .map(|&(id, _)| id)
        .choose(&mut rand::thread_rng())
    {
        Ok(random_opponent)
    } else {
        Err(anyhow::anyhow!("Failed to find an opponent"))
    }
}

pub fn get_opponent_base_details_for_attack(
    defender_id: i32,
    conn: &mut PgConnection,
    attacker_id: i32,
) -> Result<(i32, DefenseResponse)> {
    let map = fetch_map_layout(conn, &defender_id)?;
    let map_id = map.id;

    let response = get_map_details_for_attack(conn, map, attacker_id)?;

    Ok((map_id, response))
}

pub fn get_opponent_base_details_for_simulation(
    defender_id: i32,
    conn: &mut PgConnection,
) -> Result<SimulationBaseResponse> {
    let map = fetch_map_layout(conn, &defender_id)?;

    let response = get_map_details_for_simulation(conn, map)?;

    Ok(response)
}

pub fn add_game_id_to_redis(
    attacker_id: i32,
    defender_id: i32,
    game_id: i32,
    mut redis_conn: RedisConn,
) -> Result<()> {
    redis_conn
        .set_ex::<_, _, ()>(
            format!("Attacker:{}", attacker_id),
            game_id,
            GAME_AGE_IN_MINUTES * 60,
        )
        .map_err(|err| anyhow::anyhow!("Failed to set attacker key: {}", err))?;

    redis_conn
        .set_ex::<_, _, ()>(
            format!("Defender:{}", defender_id),
            game_id,
            GAME_AGE_IN_MINUTES * 60,
        )
        .map_err(|err| anyhow::anyhow!("Failed to set defender key: {}", err))?;

    Ok(())
}

pub fn get_game_id_from_redis(
    user_id: i32,
    redis_conn: &mut RedisConn,
    is_attacker: bool,
) -> Result<Option<i32>> {
    if is_attacker {
        let game_id: Option<i32> = redis_conn
            .get(format!("Attacker:{}", user_id))
            .map_err(|err| anyhow::anyhow!("Failed to get key: {}", err))?;
        Ok(game_id)
    } else {
        let game_id: Option<i32> = redis_conn
            .get(format!("Defender:{}", user_id))
            .map_err(|err| anyhow::anyhow!("Failed to get key: {}", err))?;
        Ok(game_id)
    }
}

pub fn delete_game_id_from_redis(
    attacker_id: i32,
    defender_id: i32,
    redis_conn: &mut RedisConn,
) -> Result<()> {
    redis_conn
        .del::<_, ()>(format!("Attacker:{}", attacker_id))
        .map_err(|err| anyhow::anyhow!("Failed to delete attacker key: {}", err))?;
    redis_conn
        .del::<_, ()>(format!("Defender:{}", defender_id))
        .map_err(|err| anyhow::anyhow!("Failed to delete defender key: {}", err))?;

    Ok(())
}

pub fn encode_attack_token(attacker_id: i32, defender_id: i32, game_id: i32) -> Result<String> {
    let jwt_secret = env::var("COOKIE_KEY").expect("COOKIE_KEY must be set!");
    let now = chrono::Local::now();
    let iat = now.timestamp() as usize;
    let jwt_max_age: i64 = ATTACK_TOKEN_AGE_IN_MINUTES * 60;
    let token_expiring_time = now + chrono::Duration::seconds(jwt_max_age);
    let exp = (token_expiring_time).timestamp() as usize;
    let token: AttackToken = AttackToken {
        game_id,
        attacker_id,
        defender_id,
        exp,
        iat,
    };

    let token_result = encode(
        &Header::default(),
        &token,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    );
    let token = match token_result {
        Ok(token) => token,
        Err(e) => return Err(e.into()),
    };

    Ok(token)
}

pub fn decode_user_token(token: &str) -> Result<i32> {
    let jwt_secret = env::var("COOKIE_KEY").expect("COOKIE_KEY must be set!");
    let token_data = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_str().as_ref()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|err| anyhow::anyhow!("Failed to decode token: {}", err))?;

    let now = chrono::Local::now();
    let iat = now.timestamp() as usize;
    if iat > token_data.claims.exp {
        return Err(anyhow::anyhow!("Attack token expired"));
    }

    Ok(token_data.claims.id)
}

pub fn decode_attack_token(token: &str) -> Result<AttackToken> {
    let jwt_secret = env::var("COOKIE_KEY").expect("COOKIE_KEY must be set!");
    let token_data = decode::<AttackToken>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_str().as_ref()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|err| anyhow::anyhow!("Failed to decode token: {}", err))?;

    Ok(token_data.claims)
}

pub fn get_mines(conn: &mut PgConnection, map_id: i32) -> Result<Vec<MineDetails>> {
    use crate::schema::{block_type, map_spaces, mine_type};

    let joined_table = map_spaces::table
        .filter(map_spaces::map_id.eq(map_id))
        .inner_join(block_type::table.inner_join(mine_type::table))
        .inner_join(prop::table.on(mine_type::prop_id.eq(prop::id)));

    let mines: Vec<MineDetails> = joined_table
        .load::<(MapSpaces, (BlockType, MineType), Prop)>(conn)?
        .into_iter()
        .enumerate()
        .map(|(mine_id, (map_space, (_, mine_type), prop))| MineDetails {
            id: mine_id as i32,
            damage: mine_type.damage,
            radius: prop.range,
            position: Coords {
                x: map_space.x_coordinate,
                y: map_space.y_coordinate,
            },
        })
        .collect();

    Ok(mines)
}

/*
* to get defenders of a map, just map_id is enough
* TODO: Remove this redundnat user_id variable
 */
pub fn get_defenders(
    conn: &mut PgConnection,
    map_id: i32,
    user_id: i32,
) -> Result<Vec<DefenderDetails>> {
    use crate::schema::{block_type, defender_type, map_spaces};
    // let result: Vec<(
    //     MapSpaces,
    //     (BlockType, AvailableBlocks, BuildingType, DefenderType),
    // )> = map_spaces::table
    //     .inner_join(
    //         block_type::table
    //             .inner_join(available_blocks::table)
    //             .inner_join(building_type::table)
    //             .inner_join(defender_type::table),
    //     )
    //     .filter(map_spaces::map_id.eq(map_id))
    //     .filter(available_blocks::user_id.eq(user_id))
    //     .load::<(
    //         MapSpaces,
    //         (BlockType, AvailableBlocks, BuildingType, DefenderType),
    //     )>(conn)
    //     .map_err(|err| DieselError {
    //         table: "map_spaces",
    //         function: function!(),
    //         error: err,
    //     })?;

    // let result = available_blocks::table
    // .inner_join(block_type::table)
    // .filter(block_type::category.eq(BlockCategory::Defender))
    // .inner_join(defender_type::table.on(block_type::category_id.eq(defender_type::id)))
    // .inner_join(prop::table.on(defender_type::prop_id.eq(prop::id)))
    // .inner_join(map_spaces::table.on(block_type::id.eq(map_spaces::block_type_id)))
    // .filter(map_spaces::map_id.eq(map_id))
    // .filter(available_blocks::user_id.eq(user_id))
    // .load::<(AvailableBlocks, BlockType, DefenderType, Prop, MapSpaces)>(conn)
    // .map_err(|err| DieselError {
    //     table: "map_spaces",
    //     function: function!(),
    //     error: err,
    // })?;

    let result = map_spaces::table
        .inner_join(map_layout::table)
        .filter(map_layout::player.eq(user_id))
        .inner_join(block_type::table.on(map_spaces::block_type_id.eq(block_type::id)))
        .inner_join(defender_type::table.on(defender_type::id.eq(block_type::defender_type.assume_not_null())))
        .inner_join(prop::table.on(defender_type::prop_id.eq(prop::id)))
        .filter(map_spaces::map_id.eq(map_id))
        .filter(block_type::category.eq(BlockCategory::Defender))
        .select((
            map_spaces::all_columns,
            block_type::all_columns,
            defender_type::all_columns,
            prop::all_columns,
        ))
        .load::<(MapSpaces, BlockType, DefenderType, Prop)>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?;

    let mut defenders: Vec<DefenderDetails> = Vec::new();

    for (map_space, block_type, defender, prop) in result.iter() {
        let (hut_x, hut_y) = (map_space.x_coordinate, map_space.y_coordinate);
        // let path: Vec<(i32, i32)> = vec![(hut_x, hut_y)];
        defenders.push(DefenderDetails {
            map_space_id: map_space.id,
            name: defender.name.clone(),
            radius: prop.range,
            speed: defender.speed,
            damage: defender.damage,
            defender_pos: Coords { x: hut_x, y: hut_y },
            is_alive: true,
            damage_dealt: false,
            target_id: None,
            path_in_current_frame: Vec::new(),
            block_id: block_type.id,
            level: defender.level,
            max_health: defender.max_health,
        })
    }
    // Sorted to handle multiple defenders attack same attacker at same frame
    // defenders.sort_by(|defender_1, defender_2| (defender_2.damage).cmp(&defender_1.damage));
    Ok(defenders)
}

pub fn get_buildings(conn: &mut PgConnection, map_id: i32) -> Result<Vec<BuildingDetails>> {
    use crate::schema::{block_type, building_type, map_spaces};

    let joined_table = map_spaces::table
        .inner_join(
            block_type::table
                .inner_join(building_type::table)
                .inner_join(prop::table.on(building_type::prop_id.eq(prop::id))),
        )
        .filter(map_spaces::map_id.eq(map_id))
        .filter(building_type::id.ne(ROAD_ID));

    let buildings: Vec<BuildingDetails> = joined_table
        .load::<(MapSpaces, (BlockType, BuildingType, Prop))>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(
            |(map_space, (block_type, building, prop))| BuildingDetails {
                block_id: block_type.id,
                map_space_id: map_space.id,
                current_hp: building.hp,
                total_hp: building.hp,
                artifacts_obtained: 0,
                tile: Coords {
                    x: map_space.x_coordinate,
                    y: map_space.y_coordinate,
                },
                level: building.level,
                width: building.width,
                range: prop.range,
                frequency: prop.frequency,
                name: building.name,
            },
        )
        .collect();
    update_buidling_artifacts(conn, map_id, buildings)
}

pub fn get_hut_defender(
    conn: &mut PgConnection,
    map_id: i32,
) -> Result<HashMap<i32, DefenderDetails>> {
    let joined_table = block_type::table
        .inner_join(defender_type::table)
        .inner_join(prop::table.on(defender_type::prop_id.eq(prop::id)))
        .filter(defender_type::name.eq("Hut_Defender"));
    let hut_defenders = joined_table
        .load::<(BlockType, DefenderType, Prop)>(conn)
        .map_err(|err| DieselError {
            table: "defender_type",
            function: function!(),
            error: err,
        })?
        .into_iter();
    let mut hut_defender_array: Vec<DefenderDetails> = Vec::new();
    for (i, (block_type, defender_type, prop)) in hut_defenders.enumerate() {
        hut_defender_array.push(DefenderDetails {
            map_space_id: (i + 1) as i32,
            name: defender_type.name.clone(),
            radius: prop.range,
            speed: defender_type.speed,
            damage: defender_type.damage,
            defender_pos: Coords { x: 0, y: 0 },
            is_alive: true,
            damage_dealt: false,
            target_id: None,
            path_in_current_frame: Vec::new(),
            block_id: block_type.id,
            level: defender_type.level,
            max_health: defender_type.max_health,
        });
        log::info!("hut_defenders {:?}", i);
    }

    // .map(|(block_type, defender_type, prop)| DefenderDetails {
    //     mapSpaceId: i + 1,
    //     name: defender_type.name.clone(),
    //     radius: prop.range,
    //     speed: defender_type.speed,
    //     damage: defender_type.damage,
    //     defender_pos: Coords { x: 0, y: 0 },
    //     is_alive: true,
    //     damage_dealt: false,
    //     target_id: None,
    //     path_in_current_frame: Vec::new(),
    //     block_id: block_type.id,
    //     level: defender_type.level,
    // })
    // .collect();
    log::info!("hut_defenders array {:?}", hut_defender_array);

    let joined_table = map_spaces::table
        .inner_join(block_type::table)
        .inner_join(building_type::table.on(block_type::building_type.eq(building_type::id)))
        .filter(building_type::name.eq("Defender_Hut"))
        .filter(map_spaces::map_id.eq(map_id))
        .filter(building_type::id.ne(ROAD_ID));

    let huts: Vec<(i32, i32)> = joined_table
        .load::<(MapSpaces, BlockType, BuildingType)>(conn)
        .map_err(|err| DieselError {
            table: "building_type",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(map_spaces, _, building)| (map_spaces.id, building.level))
        .collect();

    log::info!("hut defeners {:?}", hut_defender_array);
    // let mut hut_defenders_res: HashMap<i32, Vec<DefenderDetails>> = HashMap::new();
    // for (i, hut) in huts.iter().enumerate() {
    //     // log::info!("hut mapspaceid{:?}", hut.0);
    //     // if let Some(hut_defender) = hut_defender_array.iter().find(|hd| hd.level == hut.1) {
    //     //     hut_defenders_res.insert(hut.0, hut_defender.clone());
    //     // }
    //     for (i, hut_defender) in hut_defender_array.iter().enumerate() {
    //         if hut_defender.level == hut.1 {
    //             hut_defenders_res
    //                 .entry(hut.0)
    //                 .or_insert_with(Vec::new)
    //                 .push(hut_defender.clone());
    //         }
    //     }
    // }
    let mut hut_defenders_res: HashMap<i32, DefenderDetails> = HashMap::new();
    for (i, hut) in huts.iter().enumerate() {
        // log::info!("hut mapspaceid{:?}", hut.0);
        // if let Some(hut_defender) = hut_defender_array.iter().find(|hd| hd.level == hut.1) {
        //     hut_defenders_res.insert(hut.0, hut_defender.clone());
        // }
        hut_defenders_res.insert(hut.0, hut_defender_array[i].clone());
    }
    log::info!("{:?}", hut_defenders_res);
    Ok(hut_defenders_res)
}

pub fn get_bomb_types(conn: &mut PgConnection) -> Result<Vec<BombType>> {
    use crate::schema::emp_type::dsl::*;
    let bomb_types = emp_type
        .load::<EmpType>(conn)
        .map_err(|err| DieselError {
            table: "emp_type",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|emp| BombType {
            id: emp.id,
            radius: emp.attack_radius,
            damage: emp.attack_damage,
            total_count: 0,
        })
        .collect();
    Ok(bomb_types)
}

pub fn update_buidling_artifacts(
    conn: &mut PgConnection,
    map_id: i32,
    mut buildings: Vec<BuildingDetails>,
) -> Result<Vec<BuildingDetails>> {
    use crate::schema::{artifact, map_spaces};

    let result: Vec<(MapSpaces, Artifact)> = map_spaces::table
        .inner_join(artifact::table)
        .filter(map_spaces::map_id.eq(map_id))
        .load::<(MapSpaces, Artifact)>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?;

    // From the above table, create a hashmap, key being map_space_id and value being the artifact count
    let mut artifact_count: HashMap<i32, i64> = HashMap::new();

    for (map_space, artifact) in result.iter() {
        artifact_count.insert(map_space.id, artifact.count.into());
    }

    // Update the buildings with the artifact count
    for building in buildings.iter_mut() {
        building.artifacts_obtained =
            *artifact_count.get(&building.map_space_id).unwrap_or(&0) as i32;
    }

    Ok(buildings)
}

pub fn terminate_game(
    game_log: &mut GameLog,
    conn: &mut PgConnection,
    damaged_buildings: &[BuildingResponse],
    redis_conn: &mut RedisConn,
) -> Result<()> {
    use crate::schema::{artifact, game};
    let attacker_id = game_log.a.id;
    let defender_id = game_log.d.id;
    let damage_done = game_log.r.d;
    let bombs_used = game_log.r.b;
    let artifacts_collected = game_log.r.a;
    let game_id = game_log.g;
    log::info!(
        "Terminating game for game:{} and attacker:{} and opponent:{}",
        game_id,
        attacker_id,
        defender_id
    );

    let (attack_score, defense_score) = if damage_done < WIN_THRESHOLD {
        (damage_done - 100, 100 - damage_done)
    } else {
        (damage_done, -damage_done)
    };

    let attacker_details = user::table
        .filter(user::id.eq(attacker_id))
        .first::<User>(conn)
        .map_err(|err| DieselError {
            table: "game",
            function: function!(),
            error: err,
        })?;

    let defender_details = user::table
        .filter(user::id.eq(defender_id))
        .first::<User>(conn)
        .map_err(|err| DieselError {
            table: "game",
            function: function!(),
            error: err,
        })?;

    let attack_score = attack_score as f32 / 100_f32;
    let defence_score = defense_score as f32 / 100_f32;

    let new_trophies = new_rating(
        attacker_details.trophies,
        defender_details.trophies,
        attack_score,
        defence_score,
    );

    //Add bonus trophies (just call the function)

    game_log.r.oa = attacker_details.trophies;
    game_log.r.od = defender_details.trophies;
    game_log.r.na = new_trophies.0;
    game_log.r.nd = new_trophies.1;

    diesel::update(game::table.find(game_id))
        .set((
            game::damage_done.eq(damage_done),
            game::is_game_over.eq(true),
            game::emps_used.eq(bombs_used),
            game::attack_score.eq(new_trophies.0 - attacker_details.trophies),
            game::defend_score.eq(new_trophies.1 - defender_details.trophies),
            game::artifacts_collected.eq(artifacts_collected),
        ))
        .execute(conn)
        .map_err(|err| DieselError {
            table: "game",
            function: function!(),
            error: err,
        })?;

    let (attacker_wins, defender_wins) = if damage_done < WIN_THRESHOLD {
        (0, 1)
    } else {
        (1, 0)
    };

    diesel::update(user::table.find(&game_log.a.id))
        .set((
            user::artifacts.eq(user::artifacts + artifacts_collected),
            user::trophies.eq(user::trophies + new_trophies.0 - attacker_details.trophies),
            user::attacks_won.eq(user::attacks_won + attacker_wins),
        ))
        .execute(conn)
        .map_err(|err| DieselError {
            table: "game",
            function: function!(),
            error: err,
        })?;

    if deduct_artifacts_from_building(damaged_buildings.to_vec(), conn).is_err() {
        log::info!(
            "Failed to deduct artifacts from building for game:{} and attacker:{} and opponent:{}",
            game_id,
            attacker_id,
            defender_id
        );
    }
    diesel::update(user::table.find(&game_log.d.id))
        .set((
            user::artifacts.eq(user::artifacts - artifacts_collected),
            user::trophies.eq(user::trophies + new_trophies.1 - defender_details.trophies),
            user::defenses_won.eq(user::defenses_won + defender_wins),
        ))
        .execute(conn)
        .map_err(|err| DieselError {
            table: "game",
            function: function!(),
            error: err,
        })?;

    let attacker_map_id = get_user_map_id(attacker_id, conn)?;
    let attacker_bank_block_type_id = get_block_id_of_bank(conn, &attacker_id)?;
    let attacker_bank_map_space_id =
        get_bank_map_space_id(conn, &attacker_map_id, &attacker_bank_block_type_id)?;

    diesel::update(artifact::table.find(attacker_bank_map_space_id))
        .set(artifact::count.eq(artifact::count + artifacts_collected))
        .execute(conn)
        .map_err(|err| DieselError {
            table: "artifact",
            function: function!(),
            error: err,
        })?;

    // if let Ok(sim_log) = serde_json::to_string(&game_log) {
    //     let new_simulation_log = NewSimulationLog {
    //         game_id: &game_id,
    //         log_text: &sim_log,
    //     };

    //     println!("Inserting into similation log, game id: {}", game_id);
    //     diesel::insert_into(simulation_log::table)
    //         .values(new_simulation_log)
    //         .on_conflict_do_nothing()
    //         .execute(conn)
    //         .map_err(|err| DieselError {
    //             table: "simulation_log",
    //             function: function!(),
    //             error: err,
    //         })?;
    //     println!("Done Inserting into similation log, game id: {}", game_id);
    // }

    if delete_game_id_from_redis(game_log.a.id, game_log.d.id, redis_conn).is_err() {
        log::info!(
            "Can't remove game:{} and attacker:{} and opponent:{} from redis",
            game_id,
            attacker_id,
            defender_id
        );
        return Err(anyhow::anyhow!("Can't remove game from redis"));
    }

    // for event in game_log.events.iter() {
    //     println!("Event: {:?}\n", event);
    // }

    log::info!(
        "Game terminated successfully for game:{} and attacker:{} and opponent:{}",
        game_id,
        attacker_id,
        defender_id
    );

    Ok(())
}

pub fn check_and_remove_incomplete_game(
    attacker_id: &i32,
    defender_id: &i32,
    game_id: &i32,
    conn: &mut PgConnection,
) -> Result<()> {
    use crate::schema::game::dsl::*;

    let pending_games = game
        .filter(
            attack_id
                .eq(attacker_id)
                .and(defend_id.eq(defender_id))
                .and(id.ne(game_id))
                .and(is_game_over.eq(false)),
        )
        .load::<Game>(conn)
        .map_err(|err| DieselError {
            table: "game",
            function: function!(),
            error: err,
        })?;

    let _len = pending_games.len();

    for pending_game in pending_games {
        diesel::delete(game.filter(id.eq(pending_game.id)))
            .execute(conn)
            .map_err(|err| DieselError {
                table: "game",
                function: function!(),
                error: err,
            })?;
    }

    Ok(())
}

pub fn can_attack_happen(conn: &mut PgConnection, user_id: i32, is_attacker: bool) -> Result<bool> {
    use crate::schema::game::dsl::*;

    let current_date = chrono::Local::now().date_naive();

    if is_attacker {
        let count: i64 = game
            .filter(attack_id.eq(user_id))
            .filter(is_game_over.eq(true))
            .filter(date.eq(current_date))
            .count()
            .get_result::<i64>(conn)
            .map_err(|err| DieselError {
                table: "game",
                function: function!(),
                error: err,
            })?;
        Ok(count < TOTAL_ATTACKS_PER_DAY)
    } else {
        let count: i64 = game
            .filter(defend_id.eq(user_id))
            .filter(is_game_over.eq(true))
            .filter(date.eq(current_date))
            .count()
            .get_result::<i64>(conn)
            .map_err(|err| DieselError {
                table: "game",
                function: function!(),
                error: err,
            })?;
        Ok(count < TOTAL_ATTACKS_PER_DAY)
    }
}

pub fn deduct_artifacts_from_building(
    damaged_buildings: Vec<BuildingResponse>,
    conn: &mut PgConnection,
) -> Result<()> {
    use crate::schema::artifact;
    for building in damaged_buildings.iter() {
        if (building.artifacts_if_damaged) > 0 {
            diesel::update(artifact::table.find(building.id))
                .set(artifact::count.eq(artifact::count - building.artifacts_if_damaged))
                .execute(conn)
                .map_err(|err| DieselError {
                    table: "artifact",
                    function: function!(),
                    error: err,
                })?;
        }
    }
    Ok(())
}

pub fn artifacts_obtainable_from_base(map_id: i32, conn: &mut PgConnection) -> Result<i32> {
    use crate::schema::{artifact, map_spaces};

    let mut artifacts = 0;

    for (_, count) in map_spaces::table
        .left_join(artifact::table)
        .filter(map_spaces::map_id.eq(map_id))
        .select((map_spaces::all_columns, artifact::count.nullable()))
        .load::<(MapSpaces, Option<i32>)>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?
        .into_iter()
    {
        if let Some(count) = count {
            artifacts += (count as f32 * PERCENTANGE_ARTIFACTS_OBTAINABLE).floor() as i32;
        }
    }

    Ok(artifacts)
}

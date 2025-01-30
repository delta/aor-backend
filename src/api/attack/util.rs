use self::util::{get_valid_road_paths, AttackResponse, GameLog, ResultResponse};
use super::auth::session::AuthUser;
use super::defense::shortest_path::run_shortest_paths;
use super::defense::util::{
    AttackBaseResponse, DefenseResponse, MineTypeResponseWithoutBlockId, SimulationBaseResponse,
};
use super::user::util::fetch_user;
use super::{error, PgPool, RedisPool};
use crate::api::attack::socket::{BuildingResponse, ResultType, SocketRequest, SocketResponse};
use crate::api::util::HistoryboardQuery;
use crate::constants::{GAME_AGE_IN_MINUTES, MAX_BOMBS_PER_ATTACK};
use crate::models::{AttackerType, User};
use crate::validator::state::State;
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
    Artifact, AttackerType, AvailableBlocks, BlockCategory, BlockType, BuildingType, DefenderType,
    EmpType, Game, LevelsFixture, MapLayout, MapSpaces, MineType, NewAttackerPath, NewGame, Prop,
    User,
};
use crate::schema::{block_type, building_type, defender_type, map_spaces, prop, user};
use crate::schema::{block_type, building_type, defender_type, map_spaces, prop, user};
use crate::util::function;
use crate::validator::util::Coords;
use crate::validator::util::{BombType, BuildingDetails, DefenderDetails, MineDetails};
use crate::validator::util::{Coords, SourceDestXY};
use actix_rt;
use actix_web::error::ErrorBadRequest;
use actix_web::web::{Data, Json};
use actix_web::{web, Error, HttpRequest, HttpResponse, Responder, Result};
use log;
use std::collections::{HashMap, HashSet};
use std::time;

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
>>>>>>> 3114afe (refactor: block type migration update. (#96))
    };

    log::info!(
        "Opponent:{} found for Attacker:{}",
        opponent_id,
        attacker_id
    );

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;

    //Fetch base details and shortest paths data
    let (map_id, opponent_base) = web::block(move || {
        Ok(util::get_opponent_base_details_for_attack(
            opponent_id,
            &mut conn,
            attacker_id,
        )?) as anyhow::Result<(i32, DefenseResponse)>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    log::info!("Base details of Opponent:{} fetched", opponent_id);

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;

    let obtainable_artifacts = web::block(move || {
        Ok(util::artifacts_obtainable_from_base(map_id, &mut conn)?) as anyhow::Result<i32>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    log::info!(
        "Artifacts obtainable from opponent: {} base is {}",
        opponent_id,
        obtainable_artifacts
    );

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;

    let user_details =
        web::block(move || Ok(fetch_user(&mut conn, opponent_id)?) as anyhow::Result<Option<User>>)
            .await?
            .map_err(|err| error::handle_error(err.into()))?;

    log::info!("User details fetched for Opponent:{}", opponent_id);

    //Create game
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let game_id = web::block(move || {
        Ok(util::add_game(attacker_id, opponent_id, map_id, &mut conn)?) as anyhow::Result<i32>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    log::info!(
        "Game:{} created for Attacker:{} and Opponent:{}",
        game_id,
        attacker_id,
        opponent_id
    );

    //Generate attack token to validate the /attack/start
    let attack_token = util::encode_attack_token(attacker_id, opponent_id, game_id)
        .map_err(|err| error::handle_error(err.into()))?;
    let response: AttackResponse = AttackResponse {
        user: user_details,
        max_bombs: MAX_BOMBS_PER_ATTACK,
        base: AttackBaseResponse {
            map_spaces: opponent_base.map_spaces,
            defender_types: opponent_base.defender_types,
            blocks: opponent_base.blocks,
            mine_types: opponent_base
                .mine_types
                .iter()
                .map(|mine_type| MineTypeResponseWithoutBlockId {
                    id: mine_type.id,
                    name: mine_type.name.clone(),
                    damage: mine_type.damage,
                    cost: mine_type.cost,
                    level: mine_type.level,
                    radius: mine_type.radius,
                })
                .collect(),
        },
        shortest_paths: None,
        obtainable_artifacts,
        attack_token,
        attacker_types: opponent_base.attacker_types,
        bomb_types: opponent_base.bomb_types,
        game_id,
    };

    log::info!(
        "Attack response generated for Attacker:{} and Opponent:{}",
        attacker_id,
        opponent_id
    );
    Ok(Json(response))
}

async fn socket_handler(
    pool: web::Data<PgPool>,
    redis_pool: Data<RedisPool>,
    req: HttpRequest,
    body: web::Payload,
) -> Result<HttpResponse, Error> {
    let query_params = req.query_string().split('&').collect::<Vec<&str>>();
    let user_token = query_params[0].split('=').collect::<Vec<&str>>()[1];
    let attack_token = query_params[1].split('=').collect::<Vec<&str>>()[1];

    let attacker_id =
        util::decode_user_token(user_token).map_err(|err| error::handle_error(err.into()))?;
    let attack_token_data =
        util::decode_attack_token(attack_token).map_err(|err| error::handle_error(err.into()))?;
    let game_id = attack_token_data.game_id;

    log::info!(
        "Attacker:{} is trying to start an attack with game:{}",
        attacker_id,
        game_id
    );

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;

    if attacker_id != attack_token_data.attacker_id {
        log::info!(
            "Attacker:{} is not authorised to start an attack with game:{}",
            attacker_id,
            game_id
        );
        return Err(ErrorBadRequest("User not authorised"));
    }

    let defender_id = attack_token_data.defender_id;
    if attacker_id == defender_id {
        log::info!("Attacker:{} is trying to attack himself", attacker_id);
        return Err(ErrorBadRequest("Can't attack yourself"));
    }

    let mut redis_conn = redis_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;

    if let Ok(Some(_)) = util::get_game_id_from_redis(attacker_id, &mut redis_conn, true) {
        log::info!("Attacker:{} has an ongoing game", attacker_id);
        return Err(ErrorBadRequest("Attacker has an ongoing game"));
    }

    if let Ok(Some(_)) = util::get_game_id_from_redis(defender_id, &mut redis_conn, false) {
        log::info!("Defender:{} has an ongoing game", defender_id);
        return Err(ErrorBadRequest("Defender has an ongoing game"));
    }

<<<<<<< HEAD
    if util::check_and_remove_incomplete_game(&attacker_id, &defender_id, &game_id, &mut conn)
        .is_err()
    {
        log::info!(
            "Failed to remove incomplete games for Attacker:{} and Defender:{}",
            attacker_id,
            defender_id
        );
    }
=======
    let joined_table = map_spaces::table
        .filter(map_spaces::map_id.eq(map_id))
        .inner_join(block_type::table.inner_join(mine_type::table))
        .inner_join(prop::table.on(mine_type::prop_id.eq(prop::id)));
>>>>>>> 3114afe (refactor: block type migration update. (#96))

    log::info!(
        "Game:{} is valid for Attacker:{} and Defender:{}",
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

pub fn get_defenders(
    conn: &mut PgConnection,
    map_id: i32,
    user_id: i32,
) -> Result<Vec<DefenderDetails>> {
    use crate::schema::{available_blocks, block_type, defender_type, map_spaces};
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

    let result: Vec<(
        MapSpaces,
        (BlockType, AvailableBlocks, BuildingType, DefenderType, Prop),
    )> = map_spaces::table
        .inner_join(
            block_type::table
                .inner_join(available_blocks::table)
                .inner_join(building_type::table)
                .inner_join(defender_type::table)
                .inner_join(prop::table.on(defender_type::prop_id.eq(prop::id))),
        )
        .filter(map_spaces::map_id.eq(map_id))
        .filter(available_blocks::user_id.eq(user_id))
        .load::<(
            MapSpaces,
            (BlockType, AvailableBlocks, BuildingType, DefenderType, Prop),
        )>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?;

    let mut defenders: Vec<DefenderDetails> = Vec::new();

    for (map_space, (block_type, _, _, defender, prop)) in result.iter() {
        let (hut_x, hut_y) = (map_space.x_coordinate, map_space.y_coordinate);
        // let path: Vec<(i32, i32)> = vec![(hut_x, hut_y)];
        defenders.push(DefenderDetails {
            mapSpaceId: map_space.id,
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

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;

    let defender_base_details = web::block(move || {
        Ok(util::get_opponent_base_details_for_simulation(
            defender_id,
            &mut conn,
        )?) as anyhow::Result<SimulationBaseResponse>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    if attacker_user_details.is_none() || defender_user_details.is_none() {
        return Err(ErrorBadRequest("User details not found"));
    }

    let redis_conn = redis_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;

    if util::add_game_id_to_redis(attacker_id, defender_id, game_id, redis_conn).is_err() {
        println!("Cannot add game:{} to redis", game_id);
        return Err(ErrorBadRequest("Internal Server Error"));
    }

    let mut damaged_buildings: Vec<BuildingResponse> = Vec::new();
    let buildings: Vec<BuildingDetails> = joined_table
        .load::<(MapSpaces, (BlockType, BuildingType, Prop))>(conn)
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
                width: building.width,
                name: building.name,
                range: prop.range,
                frequency: prop.frequency,
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
            mapSpaceId: (i + 1) as i32,
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
        "Socket connection established for Game:{}, Attacker:{} and Defender:{}",
        game_id,
        attacker_id,
        defender_id
    );

    let mut session_clone1 = session.clone();
    let mut session_clone2 = session.clone();

    actix_rt::spawn(async move {
        let mut game_state = State::new(
            attacker_id,
            defender_id,
            defenders,
            hut_defenders,
            mines,
            buildings,
        );
        game_state.set_total_hp_buildings();

        let game_logs = &mut game_log.clone();

        let mut conn = pool
            .get()
            .map_err(|err| error::handle_error(err.into()))
            .unwrap();

        let mut redis_conn = redis_pool
            .clone()
            .get()
            .map_err(|err| error::handle_error(err.into()))
            .unwrap();

        let shortest_path = &shortest_paths.clone();
        let roads = &roads.clone();
        let bomb_types = &bomb_types.clone();
        let attacker_type = &attacker_type.clone();

        log::info!(
            "Game:{} is ready to be played for Attacker:{} and Defender:{}",
            game_id,
            attacker_id,
            defender_id
        );

        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Ping(bytes) => {
                    if session_clone1.pong(&bytes).await.is_err() {
                        return;
                    }
                }
                Message::Text(s) => {
                    if let Ok(socket_request) = serde_json::from_str::<SocketRequest>(&s) {
                        let response_result = game_handler(
                            attacker_type,
                            socket_request,
                            &mut game_state,
                            shortest_path,
                            roads,
                            bomb_types,
                            game_logs,
                        );
                        match response_result {
                            Some(Ok(response)) => {
                                if let Ok(response_json) = serde_json::to_string(&response) {
                                    // println!("Response Json ---- {}", response_json);
                                    if response.result_type == ResultType::GameOver {
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                        if (session_clone1.clone().close(None).await).is_err() {
                                            log::info!("Error closing the socket connection for game:{} and attacker:{} and opponent:{}", game_id, attacker_id, defender_id);
                                        }
                                        if util::terminate_game(
                                            game_logs,
                                            &mut conn,
                                            &damaged_buildings,
                                            &mut redis_conn,
                                        )
                                        .is_err()
                                        {
                                            log::info!("Error terminating the game 1 for game:{} and attacker:{} and opponent:{}", game_id, attacker_id, defender_id);
                                        }
                                    } else if response.result_type == ResultType::MinesExploded {
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    } else if response.result_type == ResultType::DefendersDamaged {
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    } else if response.result_type == ResultType::BulletHit {
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    } else if response.result_type == ResultType::UAV {
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    } else if response.result_type == ResultType::DefendersTriggered
                                    {
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    } else if response.result_type == ResultType::SpawnHutDefender {
                                        // game_state.hut.hut_defenders_count -= 1;
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    } else if response.result_type == ResultType::BuildingsDamaged {
                                        damaged_buildings
                                            .extend(response.damaged_buildings.unwrap());
                                        // if util::deduct_artifacts_from_building(
                                        //     response.damaged_buildings.unwrap(),
                                        //     &mut conn,
                                        // )
                                        // .is_err()
                                        // {
                                        //     log::info!("Failed to deduct artifacts from building for game:{} and attacker:{} and opponent:{}", game_id, attacker_id, defender_id);
                                        // }
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    } else if response.result_type == ResultType::PlacedAttacker {
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    } else if response.result_type == ResultType::Nothing
                                        && session_clone1.text(response_json).await.is_err()
                                    {
                                        return;
                                    }
                                } else {
                                    log::info!("Error serializing JSON for game:{} and attacker:{} and opponent:{}", game_id, attacker_id, defender_id);
                                    if session_clone1.text("Error serializing JSON").await.is_err()
                                    {
                                        return;
                                    }
                                }
                            }
                            Some(Err(err)) => {
                                log::info!("Error: {:?} while handling for game:{} and attacker:{} and opponent:{}", err, game_id, attacker_id, defender_id);
                            }
                            None => {
                                // Handle the case where game_handler returned None (e.g., ActionType::PlaceAttacker)
                                // Add appropriate logic here based on the requirements.
                                log::info!("All fine for now");
                            }
                        }
                    } else {
                        log::info!(
                            "Error parsing JSON for game:{} and attacker:{} and opponent:{}",
                            game_id,
                            attacker_id,
                            defender_id
                        );

                        if session_clone1.text("Error parsing JSON").await.is_err() {
                            return;
                        }
                    }
                }
                Message::Close(_s) => {
                    if util::terminate_game(
                        game_logs,
                        &mut conn,
                        &damaged_buildings,
                        &mut redis_conn,
                    )
                    .is_err()
                    {
                        log::info!("Error terminating the game 2 for game:{} and attacker:{} and opponent:{}", game_id, attacker_id, defender_id);
                    }
                    break;
                }
                _ => {
                    log::info!(
                        "Unknown message type for game:{} and attacker:{} and opponent:{}",
                        game_id,
                        attacker_id,
                        defender_id
                    );
                }
            }
        }
    });

    actix_rt::spawn(async move {
        let timeout_duration = time::Duration::from_secs((GAME_AGE_IN_MINUTES as u64) * 60);
        let last_activity = time::Instant::now();

        log::info!(
            "Timer started for Game:{}, Attacker:{} and Defender:{}",
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

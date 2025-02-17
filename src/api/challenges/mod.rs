use actix_web::{
    error::ErrorBadRequest,
    web::{self, Data, Json, Path, Payload, Query},
    HttpRequest, HttpResponse, Responder, Result,
};
use actix_ws::Message;
use awc::http::header::map;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    io::Read,
    time::{self, SystemTime},
};
use util::{
    get_challenge_maps, get_challenge_type_enum, get_leaderboard, is_challenge_possible,
    terminate_challenge,
};

use crate::{
    api::{
        attack::{
            socket::{BaseItemsDamageResponse, ResultType, SocketRequest, SocketResponse},
            util::{
                get_attacker_types, get_bomb_types, get_hut_defender_types, GameLog, ResultResponse,
            },
        },
        defense::{
            shortest_path::{self, run_shortest_paths_challenges},
            util::{
                fetch_attacker_types, fetch_emp_types, AdminSaveData, BuildingTypeResponse,
                ChallengeData, DefenderTypeResponse, MapSpacesResponseWithArifacts,
                MineTypeResponse, SimulationBaseResponse,
            },
        },
        user::util::fetch_user,
    },
    constants::{GAME_AGE_IN_MINUTES, MAX_BOMBS_PER_ATTACK, MOD_USER_BASE_PATH},
    models::{AttackerType, EmpType},
    validator::{
        game_handler,
        state::State,
        util::{
            Bomb, BombType, BuildingDetails, Challenge, ChallengeType, Coords, DefenderDetails,
            FallGuys, FallGuysMultipliers, MazeChallenge, MineDetails,
        },
    },
};

use super::{auth::session::AuthUser, error, PgPool, RedisPool};

pub mod util;

#[derive(Deserialize, Serialize)]
pub struct ChallengeInitBody {
    challenge_id: i32,
    user_id: i32,
    map_id: i32,
}

#[derive(Deserialize)]
pub struct ChallengeSocketQuery {
    pub challenge_id: i32,
    pub user_id: i32,
    pub map_id: i32,
}

#[derive(Serialize)]
pub struct ChallengeInitResponse {
    pub map_data: AdminSaveData,
    pub attacker_types: Vec<AttackerType>,
    pub bomb_types: Vec<EmpType>,
    pub max_bombs: i32,
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/init").route(web::post().to(init_challenge)))
        .service(web::resource("/start").route(web::get().to(challenge_socket_handler)))
        .service(
            web::resource("/leaderboard/{challenge_id}")
                .route(web::get().to(challenge_leaderboard)),
        )
        .service(web::resource("/{id}").route(web::get().to(challenge_maps)))
        .app_data(Data::new(web::JsonConfig::default().limit(1024 * 1024)));
}

async fn challenge_maps(
    challenge_id: Path<i32>,
    pool: Data<PgPool>,
    user: AuthUser,
) -> Result<impl Responder> {
    let attacker_id = user.0;
    let challenge_id = challenge_id.into_inner();
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let challenge_maps =
        web::block(move || get_challenge_maps(&mut conn, challenge_id, attacker_id))
            .await?
            .map_err(|err| error::handle_error(err.into()))?;
    Ok(Json(challenge_maps))
}

async fn init_challenge(
    body: Json<ChallengeInitBody>,
    redis_pool: Data<RedisPool>,
    pg_pool: Data<PgPool>,
    user: AuthUser,
) -> Result<impl Responder> {
    let attacker_id = user.0;
    let body = body.into_inner();

    log::info!(
        "User {} is trying to start challenge {}",
        attacker_id,
        body.challenge_id
    );
    let mut conn = pg_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;

    let is_challenge_possible = web::block(move || {
        is_challenge_possible(&mut conn, attacker_id, body.map_id, body.challenge_id)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    if !is_challenge_possible {
        return Err(ErrorBadRequest(
            "You have already played this challenge map before",
        ));
    }

    //challenge base data
    let json_path = env::current_dir()?.join(MOD_USER_BASE_PATH);
    log::info!("Json path: {}", json_path.display());
    let mut json_data_str = String::new();
    if json_path.exists() {
        let mut file = fs::File::open(json_path.clone())?;
        file.read_to_string(&mut json_data_str)?;
    }

    let json_data: HashMap<i32, HashMap<i32, AdminSaveData>> = if json_data_str.is_empty() {
        HashMap::new()
    } else {
        serde_json::from_str(&json_data_str).unwrap_or_else(|_| HashMap::new())
    };

    let default_user_rec = HashMap::new();
    let map_data = json_data.get(&body.user_id).unwrap_or(&default_user_rec);
    let map_data = map_data
        .get(&body.map_id)
        .unwrap_or(&AdminSaveData {
            map_id: body.map_id,
            building: Vec::new(),
            defenders: Vec::new(),
            mine_type: Vec::new(),
            road: Vec::new(),
            challenge: ChallengeData {
                start_tile: Coords { x: 0, y: 0 },
                end_tile: Coords { x: 0, y: 0 },
                attacker_health: 0,
                bomb_damage: 0,
                bomb_radius: 0,
            },
        })
        .clone();

    // let mut conn = pg_pool
    //     .get()
    //     .map_err(|err| error::handle_error(err.into()))?;
    // let attacker_types = web::block(move || fetch_attacker_types(&mut conn, &attacker_id))
    //     .await?
    //     .map_err(|err| error::handle_error(err.into()))?;

    // let mut conn = pg_pool
    //     .get()
    //     .map_err(|err| error::handle_error(err.into()))?;
    // let bomb_types = web::block(move || fetch_emp_types(&mut conn, &attacker_id))
    //     .await?
    //     .map_err(|err| error::handle_error(err.into()))?;

    let attacker_health = map_data.challenge.attacker_health;
    let attacker_types = vec![AttackerType {
        amt_of_emps: 10,
        id: 0,
        max_health: attacker_health,
        speed: 1,
        level: 1,
        cost: 0,
        name: "ChallengeAttacker".to_string(),
        prop_id: -1,
    }];

    let bomb_radius = map_data.challenge.bomb_radius;
    let bomb_damage = map_data.challenge.bomb_damage;
    let bomb_types = vec![EmpType {
        att_type: "ChallengeAttacker".to_string(),
        name: "ChallengeBomb".to_string(),
        id: 0,
        attack_damage: bomb_damage,
        attack_radius: bomb_radius,
        cost: 0,
        level: 1,
    }];

    let resp = ChallengeInitResponse {
        attacker_types,
        map_data,
        bomb_types,
        max_bombs: MAX_BOMBS_PER_ATTACK,
    };

    Ok(Json(resp))
}

async fn challenge_socket_handler(
    query_params: Query<ChallengeSocketQuery>,
    pool: Data<PgPool>,
    user: AuthUser,
    body: Payload,
    req: HttpRequest,
) -> Result<HttpResponse> {
    //challenge base data
    let attacker_id = user.0;
    let mod_user_id = query_params.user_id;
    let challenge_id = query_params.challenge_id;

    let json_path = env::current_dir()?.join(MOD_USER_BASE_PATH);
    log::info!("Json path: {}", json_path.display());
    let mut json_data_str = String::new();
    if json_path.exists() {
        let mut file = fs::File::open(json_path.clone())?;
        file.read_to_string(&mut json_data_str)?;
    }

    let json_data: HashMap<i32, HashMap<i32, AdminSaveData>> = if json_data_str.is_empty() {
        HashMap::new()
    } else {
        serde_json::from_str(&json_data_str).unwrap_or_else(|_| HashMap::new())
    };

    let default_user_rec = HashMap::new();
    let map_data = json_data
        .get(&query_params.user_id)
        .unwrap_or(&default_user_rec);
    let map_data = map_data
        .get(&query_params.map_id)
        .unwrap_or(&AdminSaveData {
            map_id: query_params.map_id,
            building: Vec::new(),
            defenders: Vec::new(),
            mine_type: Vec::new(),
            road: Vec::new(),
            challenge: ChallengeData {
                start_tile: Coords { x: 0, y: 0 },
                end_tile: Coords { x: 0, y: 0 },
                attacker_health: 0,
                bomb_damage: 0,
                bomb_radius: 0,
            },
        })
        .clone();

    let shortest_paths =
        run_shortest_paths_challenges(&map_data.road).expect("Error getting shortest paths");

    let defenders: Vec<DefenderDetails> = map_data
        .defenders
        .iter()
        .map(|defender| DefenderDetails {
            block_id: defender.block_id,
            map_space_id: defender.map_space_id,
            name: defender.name.clone(),
            radius: defender.radius,
            speed: defender.speed,
            damage: defender.damage,
            defender_pos: Coords {
                x: defender.pos_x,
                y: defender.pos_y,
            },
            is_alive: true,
            damage_dealt: false,
            target_id: None,
            path_in_current_frame: Vec::new(),
            level: defender.level,
            current_health: defender.max_health,
            max_health: defender.max_health,
            frequency: 0,
            range: 0,
            last_attack: 0,
        })
        .collect();

    let mut hut_defenders: HashMap<i32, DefenderDetails> = HashMap::new();
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let hut_defender_types = web::block(move || get_hut_defender_types(&mut conn))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

    for building in &map_data.building {
        if building.name == "Defender_Hut" {
            let hut_defender = hut_defender_types
                .iter()
                .find(|defender_type| defender_type.level == building.level)
                .unwrap()
                .clone();
            hut_defenders.insert(building.map_space_id, hut_defender);
        }
    }

    let mines: Vec<MineDetails> = map_data
        .mine_type
        .iter()
        .map(|mine_save| MineDetails {
            damage: mine_save.damage,
            id: mine_save.id,
            position: Coords {
                x: mine_save.pos_x,
                y: mine_save.pos_y,
            },
            radius: mine_save.radius,
            name: mine_save.name.clone(),
        })
        .collect();

    let buildings: Vec<BuildingDetails> = map_data
        .building
        .iter()
        .map(|building| BuildingDetails {
            artifacts_obtained: 0,
            block_id: building.block_id,
            map_space_id: building.map_space_id,
            current_hp: building.hp,
            total_hp: building.hp,
            tile: Coords {
                x: building.pos_x,
                y: building.pos_y,
            },
            width: building.width_in_tiles,
            name: building.name.clone(),
            range: building.range,
            frequency: building.frequency,
            level: building.level,
        })
        .collect();

    let roads: HashSet<(i32, i32)> = map_data
        .road
        .iter()
        .map(|road_save| (road_save.pos_x, road_save.pos_y))
        .collect();

    // let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    // let bomb_types = web::block(move || get_bomb_types(&mut conn))
    //     .await?
    //     .map_err(|err| error::handle_error(err.into()))?;

    // let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    // let attacker_type = web::block(move || get_attacker_types(&mut conn))
    //     .await?
    //     .map_err(|err| error::handle_error(err.into()))?;

    let mut attacker_type = HashMap::new();
    let attacker_health = map_data.challenge.attacker_health;
    attacker_type.insert(
        0,
        AttackerType {
            amt_of_emps: 10,
            id: 0,
            max_health: attacker_health,
            speed: 1,
            level: 1,
            cost: 0,
            name: "ChallengeAttacker".to_string(),
            prop_id: -1,
        },
    );

    let mut bomb_types = Vec::new();
    let bomb_damage = map_data.challenge.bomb_damage;
    let bomb_radius = map_data.challenge.bomb_radius;
    bomb_types.push(BombType {
        damage: bomb_damage,
        id: 0,
        radius: bomb_radius,
        total_count: 10,
    });

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let defender_user_details = web::block(move || fetch_user(&mut conn, mod_user_id))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let attacker_user_details = web::block(move || fetch_user(&mut conn, attacker_id))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

    let map_spaces_response_with_artifacts_sim: Vec<MapSpacesResponseWithArifacts> = map_data
        .building
        .iter()
        .map(|building| MapSpacesResponseWithArifacts {
            artifacts: Some(building.artifacts),
            id: building.map_space_id,
            x_coordinate: building.pos_x,
            y_coordinate: building.pos_y,
            block_type_id: building.block_id,
        })
        .collect();

    let building_type_response_sim: Vec<BuildingTypeResponse> = map_data
        .building
        .iter()
        .map(|building| BuildingTypeResponse {
            id: building.id,
            block_id: building.block_id,
            name: building.name.clone(),
            width: building.width_in_tiles,
            height: building.length_in_tiles,
            level: building.level,
            capacity: building.capacity,
            cost: building.cost,
            hp: building.hp,
            range: building.range,
            frequency: building.frequency,
        })
        .collect();

    let defender_type_response_sim: Vec<DefenderTypeResponse> = map_data
        .defenders
        .iter()
        .map(|defender| DefenderTypeResponse {
            id: defender.id,
            radius: defender.radius,
            speed: defender.speed,
            damage: defender.damage,
            block_id: defender.block_id,
            level: defender.level,
            cost: defender.cost,
            name: defender.name.clone(),
            defender_id: defender.map_space_id,
            max_health: defender.max_health,
        })
        .collect();

    let mine_type_response_sim: Vec<MineTypeResponse> = map_data
        .mine_type
        .iter()
        .map(|mine| MineTypeResponse {
            id: mine.id,
            radius: mine.radius,
            damage: mine.damage,
            block_id: mine.block_id,
            level: mine.level,
            cost: mine.cost,
            name: mine.name.clone(),
        })
        .collect();

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let attacker_types_sim = web::block(move || fetch_attacker_types(&mut conn, &attacker_id))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let bomb_types_sim = web::block(move || fetch_emp_types(&mut conn, &attacker_id))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let challenge_type = web::block(move || get_challenge_type_enum(&mut conn, challenge_id))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

    //log::info!("Challenge Type: {:?}", challenge_type);

    let maze = if challenge_type.as_ref().unwrap() == &ChallengeType::Maze {
        Some(MazeChallenge {
            coins: 0,
            start_time: SystemTime::now(),
        })
    } else {
        None
    };

    let fall_guys = if challenge_type.as_ref().unwrap() == &ChallengeType::FallGuys {
        Some(FallGuys {
            hut_frequency_increment: 1000,
            hut_range_increment: 1,
            sentry_frequency_increment: 1,
            sentry_range_increment: 1,
            last_intensity_update_tick: 0,
            update_intensity_interval: 10,
            multipliers: FallGuysMultipliers {
                treasury_level_1: 1,
                treasury_level_2: 2,
                treasury_level_3: 3,
            },
        })
    } else {
        None
    };

    let challenge = Challenge {
        challenge_completed: false,
        challenge_type,
        score: 0,
        start_tile: map_data.challenge.start_tile.clone(),
        end_tile: map_data.challenge.end_tile.clone(),
        maze,
        fall_guys,
    };

    let defender_base_details = SimulationBaseResponse {
        m: map_data.map_id,
        ms: map_spaces_response_with_artifacts_sim,
        b: building_type_response_sim,
        d: defender_type_response_sim,
        mt: mine_type_response_sim,
        at: attacker_types_sim,
        bt: bomb_types_sim,
    };

    if attacker_user_details.is_none() {
        return Err(ErrorBadRequest("Attacker Does not exist"));
    }

    let mut damaged_base_items: BaseItemsDamageResponse = BaseItemsDamageResponse {
        buildings_damaged: Vec::new(),
        defenders_damaged: Vec::new(),
    };

    let game_log = GameLog {
        g: -1,
        a: attacker_user_details.unwrap(),
        d: defender_user_details.unwrap(),
        b: defender_base_details,
        e: Vec::new(),
        r: ResultResponse {
            d: 0,
            a: 0,
            b: 0,
            au: 0,
            na: 0,
            nd: 0,
            oa: 0,
            od: 0,
            sc: 0,
        },
    };

    log::info!(
        "Challenge:{} is ready for player:{}",
        mod_user_id,
        attacker_id
    );

    let (response, session, mut msg_stream) = actix_ws::handle(&req, body)?;
    log::info!(
        "Socket connection established for Challenge:{} and Player:{}",
        mod_user_id,
        attacker_id,
    );

    let mut session_clone1 = session.clone();
    let mut session_clone2 = session.clone();

    actix_rt::spawn(async move {
        let mut game_state = State::new(
            attacker_id,
            mod_user_id,
            defenders,
            hut_defenders,
            mines,
            buildings,
            Some(challenge),
        );
        game_state.set_total_hp_buildings();
        game_state.get_sentries();

        let game_logs = &mut game_log.clone();

        let mut conn = pool
            .get()
            .map_err(|err| error::handle_error(err.into()))
            .unwrap();

        // let mut redis_conn = redis_pool
        //     .clone()
        //     .get()
        //     .map_err(|err| error::handle_error(err.into()))
        //     .unwrap();

        let shortest_path = &shortest_paths.clone();
        let roads = &roads.clone();
        let bomb_types = &bomb_types.clone();
        let attacker_type = &attacker_type.clone();

        log::info!(
            "Challenge:{} is ready to be played for Player:{}",
            mod_user_id,
            attacker_id,
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
                                            log::info!("Error closing the socket connection for Challenge:{} and Player:{}", mod_user_id, attacker_id);
                                        }
                                        if terminate_challenge(
                                            &mut conn,
                                            game_logs,
                                            query_params.map_id,
                                            query_params.challenge_id,
                                        )
                                        .is_err()
                                        {
                                            log::info!(
                                                "Error terminating challenge for Challenge:{} and Player:{}",
                                                query_params.challenge_id,
                                                attacker_id
                                            );
                                        }
                                    } else if response.result_type == ResultType::MinesExploded {
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    } else if response.result_type == ResultType::DefendersDamaged
                                        || response.result_type == ResultType::DefendersTriggered
                                        || response.result_type == ResultType::SpawnHutDefender
                                    {
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    }
                                    // else if response.result_type == ResultType::DefendersTriggered
                                    // {
                                    //     if session_clone1.text(response_json).await.is_err() {
                                    //         return;
                                    //     }
                                    // } else if response.result_type == ResultType::SpawnHutDefender {
                                    //     // game_state.hut.hut_defenders_count -= 1;
                                    //     if session_clone1.text(response_json).await.is_err() {
                                    //         return;
                                    //     }
                                    // }
                                    else if response.result_type == ResultType::BuildingsDamaged {
                                        damaged_base_items.buildings_damaged.extend(
                                            response
                                                .damaged_base_items
                                                .clone()
                                                .unwrap()
                                                .buildings_damaged,
                                        );
                                        damaged_base_items.defenders_damaged.extend(
                                            response
                                                .damaged_base_items
                                                .clone()
                                                .unwrap()
                                                .defenders_damaged,
                                        );
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
                                    log::info!(
                                        "Error serializing JSON for Challenge:{} and Player:{}",
                                        mod_user_id,
                                        attacker_id
                                    );
                                    if session_clone1.text("Error serializing JSON").await.is_err()
                                    {
                                        return;
                                    }
                                }
                            }
                            Some(Err(err)) => {
                                log::info!(
                                    "Error: {:?} while handling for Challenge:{} and Player{}",
                                    err,
                                    mod_user_id,
                                    attacker_id,
                                );
                            }
                            None => {
                                // Handle the case where game_handler returned None (e.g., ActionType::PlaceAttacker)
                                // Add appropriate logic here based on the requirements.
                                log::info!("All fine for now");
                            }
                        }
                    } else {
                        log::info!(
                            "Error parsing JSON for Challenge:{} and Player{}",
                            mod_user_id,
                            attacker_id,
                        );

                        if session_clone1.text("Error parsing JSON").await.is_err() {
                            return;
                        }
                    }
                }
                Message::Close(_s) => {
                    if terminate_challenge(
                        &mut conn,
                        game_logs,
                        query_params.map_id,
                        query_params.challenge_id,
                    )
                    .is_err()
                    {
                        log::info!(
                            "Error terminating challenge for Challenge:{} and Player:{}",
                            query_params.challenge_id,
                            attacker_id
                        );
                    }
                    break;
                }
                _ => {
                    log::info!(
                        "Unknown message type for Challenge:{} and Player:{}",
                        mod_user_id,
                        attacker_id,
                    );
                }
            }
        }
    });

    actix_rt::spawn(async move {
        let timeout_duration = time::Duration::from_secs((GAME_AGE_IN_MINUTES as u64) * 60);
        let last_activity = time::Instant::now();

        log::info!(
            "Timer started for Challenge:{}, Player:{}",
            mod_user_id,
            attacker_id,
        );

        loop {
            actix_rt::time::sleep(time::Duration::from_secs(1)).await;

            if time::Instant::now() - last_activity > timeout_duration {
                log::info!(
                    "Challenge:{} is timed out for Player:{}",
                    mod_user_id,
                    attacker_id,
                );

                let response_json = serde_json::to_string(&SocketResponse {
                    frame_number: 0,
                    result_type: ResultType::GameOver,
                    is_alive: None,
                    attacker_health: None,
                    exploded_mines: None,
                    defender_damaged: None,
                    hut_triggered: false,
                    hut_defenders: None,
                    damaged_base_items: None,
                    total_damage_percentage: None,
                    is_sync: false,
                    shoot_bullets: None,
                    is_game_over: true,
                    message: Some("Connection timed out".to_string()),
                    companion: None,
                    challenge: None,
                    bullet_hits: None,
                    revealed_mines: None,
                })
                .unwrap();
                if session_clone2.text(response_json).await.is_err() {
                    return;
                }

                break;
            }
        }
    });

    log::info!("End of Challenge:{}, Player:{}", mod_user_id, attacker_id,);

    Ok(response)
}

async fn challenge_leaderboard(
    challenge_id: Path<i32>,
    pg_pool: Data<PgPool>,
) -> Result<impl Responder> {
    let challenge_id = challenge_id.into_inner();
    let mut conn = pg_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;
    let leader_board = web::block(move || get_leaderboard(&mut conn, challenge_id))
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

    Ok(Json(leader_board))
}

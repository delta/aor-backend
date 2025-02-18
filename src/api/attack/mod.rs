use self::util::{
    get_valid_road_paths, AttackResponse, GameLog, GeminiApiResponse, ResultResponse,
};
use super::auth::session::AuthUser;
use super::defense::shortest_path::run_shortest_paths;
use super::defense::util::{
    AttackBaseResponse, DefenseResponse, MineTypeResponseWithoutBlockId, SimulationBaseResponse,
};
use super::user::util::fetch_user;
use super::{error, PgPool, RedisPool};
use crate::api::attack::socket::{
    BuildingDamageResponse, ResultType, SocketRequest, SocketResponse,
};
use crate::api::util::HistoryboardQuery;
use crate::constants::{GAME_AGE_IN_MINUTES, MAX_BOMBS_PER_ATTACK, BASE_PROMPT};
use crate::models::{AttackerType, User};
use crate::validator::state::State;
use crate::validator::util::{BombType, BuildingDetails, DefenderDetails, MineDetails, Path};
use crate::validator::util::{Coords, SourceDestXY};
use actix_rt;
use actix_web::error::ErrorBadRequest;
use actix_web::web::{Data, Json};
use actix_web::{web, Error, HttpRequest, HttpResponse, Responder, Result};
use log;
use socket::BaseItemsDamageResponse;
use util::reset_taunt_status;
use std::collections::{HashMap, HashSet};
use std::time;
use self::util::TauntStatus;
use crate::validator::game_handler;
use actix_ws::Message;
use futures_util::stream::StreamExt;

mod rating;
pub mod socket;
pub mod util;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("").route(web::get().to(init_attack)))
        .service(web::resource("/start").route(web::get().to(socket_handler)))
        .service(web::resource("/history").route(web::get().to(attack_history)))
        .service(web::resource("/top").route(web::get().to(get_top_attacks)));
}

async fn init_attack(
    pool: web::Data<PgPool>,
    redis_pool: Data<RedisPool>,
    user: AuthUser,
    is_self: web::Query<HashMap<String, bool>>,
) -> Result<impl Responder> {
    let attacker_id = user.0;

    reset_taunt_status();
    let is_self_attack = *is_self.get("is_self").unwrap_or(&false);
    log::info!("Attacker:{} is trying to initiate an attack", attacker_id);
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    if let Ok(check) = util::can_attack_happen(&mut conn, attacker_id, true) {
        if !check {
            return Err(ErrorBadRequest("You've reached the max limit of attacks"));
        }
    }

    let mut redis_conn = redis_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;

    //Check if attacker is already in a game
    if let Ok(Some(_)) = util::get_game_id_from_redis(attacker_id, &mut redis_conn, true) {
        log::info!("Attacker:{} has an ongoing game", attacker_id);
        return Err(ErrorBadRequest("Attacker has an ongoing game"));
    }

    log::info!("Attacker:{} has no ongoing game", attacker_id);

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let redis_conn = redis_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;

    let opponent_id: i32;
    if is_self_attack {
        opponent_id = attacker_id;
    } else {
        let random_opponent_id = web::block(move || {
            Ok(util::get_random_opponent_id(
                attacker_id,
                &mut conn,
                redis_conn,
            )?) as anyhow::Result<Option<i32>>
        })
        .await?
        .map_err(|err| error::handle_error(err.into()))?;

        opponent_id = if let Some(id) = random_opponent_id {
            id
        } else {
            log::info!("No opponent found for Attacker:{}", attacker_id);
            return Err(ErrorBadRequest("No opponent found"));
        };
    }

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

    for defender in opponent_base.defender_types.iter() {
        log::info!("defender ids {} ", defender.defender_id)
    }

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

    //Create a new game
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let game_id = if !is_self_attack {
        web::block(move || {
            Ok(util::add_game(
                attacker_id,
                opponent_id,
                map_id,
                &mut conn,
                is_self_attack,
            )?) as anyhow::Result<i32>
        })
        .await?
        .map_err(|err| error::handle_error(err.into()))?
    } else {
        -2
    };

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
    let is_self_attack = query_params[2]
        .split('=')
        .collect::<Vec<&str>>()
        .get(1)
        .map(|&s| s == "true")
        .unwrap_or(false);

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
    if !is_self_attack {
        if attacker_id == defender_id {
            log::info!("Attacker:{} is trying to attack himself", attacker_id);
            return Err(ErrorBadRequest("Can't attack yourself"));
        }
    } else {
        if attacker_id != defender_id {
            log::info!("Attacker:{} is trying to attack someone else", attacker_id);
            return Err(ErrorBadRequest("Can't attack someone else"));
        }
    }

    let mut redis_conn = redis_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;

    if !is_self_attack {
        if let Ok(Some(_)) = util::get_game_id_from_redis(attacker_id, &mut redis_conn, true) {
            log::info!("Attacker:{} has an ongoing game", attacker_id);
            return Err(ErrorBadRequest("Attacker has an ongoing game"));
        }

        if let Ok(Some(_)) = util::get_game_id_from_redis(defender_id, &mut redis_conn, false) {
            log::info!("Defender:{} has an ongoing game", defender_id);
            return Err(ErrorBadRequest("Defender has an ongoing game"));
        }
    }

    if util::check_and_remove_incomplete_game(&attacker_id, &defender_id, &game_id, &mut conn)
        .is_err()
    {
        log::info!(
            "Failed to remove incomplete games for Attacker:{} and Defender:{}",
            attacker_id,
            defender_id
        );
    }

    log::info!(
        "Game:{} is valid for Attacker:{} and Defender:{}",
        game_id,
        attacker_id,
        defender_id
    );

    //Fetch map_id of the defender
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;

    let map = web::block(move || {
        let map = util::get_map_id(&defender_id, &mut conn)?;
        Ok(map) as anyhow::Result<Option<i32>>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    let map_id = if let Some(map) = map {
        map
    } else {
        return Err(ErrorBadRequest("Invalid base"));
    };

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;

    let shortest_paths = web::block(move || {
        Ok(run_shortest_paths(&mut conn, map_id)?) as anyhow::Result<HashMap<SourceDestXY, Path>>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let defenders: Vec<DefenderDetails> = web::block(move || {
        Ok(util::get_defenders(&mut conn, map_id, defender_id)?)
            as anyhow::Result<Vec<DefenderDetails>>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let hut_defenders: HashMap<i32, DefenderDetails> = web::block(move || {
        Ok(util::get_hut_defender(&mut conn, defender_id)?)
            as anyhow::Result<HashMap<i32, DefenderDetails>>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    log::info!("hut defender map: {:?}", hut_defenders);

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let mines = web::block(move || {
        Ok(util::get_mines(&mut conn, map_id)?) as anyhow::Result<Vec<MineDetails>>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let buildings = web::block(move || {
        Ok(util::get_buildings(&mut conn, map_id)?) as anyhow::Result<Vec<BuildingDetails>>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let roads = web::block(move || {
        Ok(get_valid_road_paths(map_id, &mut conn)?) as anyhow::Result<HashSet<(i32, i32)>>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let bomb_types =
        web::block(move || Ok(util::get_bomb_types(&mut conn)?) as anyhow::Result<Vec<BombType>>)
            .await?
            .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let attacker_type = web::block(move || {
        Ok(util::get_attacker_types(&mut conn)?) as anyhow::Result<HashMap<i32, AttackerType>>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;

    let attacker_user_details =
        web::block(move || Ok(fetch_user(&mut conn, attacker_id)?) as anyhow::Result<Option<User>>)
            .await?
            .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;

    let defender_user_details =
        web::block(move || Ok(fetch_user(&mut conn, defender_id)?) as anyhow::Result<Option<User>>)
            .await?
            .map_err(|err| error::handle_error(err.into()))?;

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

    if !is_self_attack {
        if util::add_game_id_to_redis(attacker_id, defender_id, game_id, redis_conn).is_err() {
            println!("Cannot add game:{} to redis", game_id);
            return Err(ErrorBadRequest("Internal Server Error"));
        }
    }

    let mut damaged_base_items: BaseItemsDamageResponse = BaseItemsDamageResponse {
        buildings_damaged: Vec::new(),
        defenders_damaged: Vec::new(),
    };

    let game_log = GameLog {
        g: game_id,
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
        "Game:{} is ready for Attacker:{} and Defender:{}",
        game_id,
        attacker_id,
        defender_id
    );

    let (response, session, mut msg_stream) = actix_ws::handle(&req, body)?;

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
            None,
        );
        game_state.set_total_hp_buildings();
        game_state.get_sentries();

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
                                            &damaged_base_items.buildings_damaged,
                                            &mut redis_conn,
                                            is_self_attack,
                                        )
                                        .is_err()
                                        {
                                            log::info!("Error terminating the game 1 for game:{} and attacker:{} and opponent:{}", game_id, attacker_id, defender_id);
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
                                    } else if response.result_type == ResultType::BulletHit {
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    } else if response.result_type == ResultType::UAV {
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
                    if game_logs.d.is_pragyan {
                        //challenge game terminate
                    }
                    if util::terminate_game(
                        game_logs,
                        &mut conn,
                        &damaged_base_items.buildings_damaged,
                        &mut redis_conn,
                        is_self_attack,
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

        loop {
            actix_rt::time::sleep(time::Duration::from_secs(1)).await;

            if time::Instant::now() - last_activity > timeout_duration {
                log::info!(
                    "Game:{} is timed out for Attacker:{} and Defender:{}",
                    game_id,
                    attacker_id,
                    defender_id
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
                    new_taunt: None,
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

    log::info!(
        "End of Game:{}, Attacker:{} and Defender:{}",
        game_id,
        attacker_id,
        defender_id,
    );

    Ok(response)
}

async fn attack_history(
    pool: web::Data<PgPool>,
    user: AuthUser,
    query: web::Query<HistoryboardQuery>,
) -> Result<impl Responder> {
    let user_id = user.0;
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    if page <= 0 || limit <= 0 {
        return Err(ErrorBadRequest("Invalid query params"));
    }
    let response = web::block(move || {
        let mut conn = pool.get()?;
        util::fetch_attack_history(user_id, page, limit, &mut conn)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;
    Ok(web::Json(response))
}

async fn get_top_attacks(pool: web::Data<PgPool>, user: AuthUser) -> Result<impl Responder> {
    let user_id = user.0;
    let response = web::block(move || {
        let mut conn = pool.get()?;
        util::fetch_top_attacks(user_id, &mut conn)
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;
    Ok(web::Json(response))
}

pub async fn get_taunt(
    event_description: String,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut response_text: String = "".to_string();
    let google_api_key =
        std::env::var("GEMINI_API_KEY_FINE_TUNED").unwrap_or_else(|_| "YOUR_API_KEY".to_string());
    let model_id =
        std::env::var("GEMINI_MODEL_ID").unwrap_or_else(|_| "YOUR_MODEL_ID".to_string());

    let url = format!(
        "https://generativelanguage.googleapis.com/v1/{}:generateContent?key={}",
        model_id,
        google_api_key
    );
    let prompt = format!("You are a defender robot who is supposed to demotivate the attacker bot in the game of Attack on Robots. Every response of yours is supposed to be against the attacker. There are only five moods. The mood must be one of these: Exhilarated – When the attacker is doing badly. Surprised – When the attacker seems to be changing the game around. Sad – When the attacker seems like winning. Frustrated – When the attacker is crushing the game. Angry – When the attacker is successful in changing the course of the game. Your response should be in this format : 'Reaction Type: reaction_type, Response: generated_text'.This has happened now: {}", event_description);
    let body = serde_json::json!({
        "contents": [   
            {
                "role": "user", 
                "parts": [
                    { "text": prompt }
                ] 
            }
        ]
    });
    let client = reqwest::Client::new();
    unsafe {
        util::TAUNTS.taunt_count += 1;
        util::TAUNTS.prev_taunt_time = time::SystemTime::now();
    }
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if response.status().is_success() {
        response_text = response.text().await?;
        // log::info!("Response: {}", response_text);
        let api_response: GeminiApiResponse = serde_json::from_str(&response_text)?;
        if let Some(candidate) = api_response.candidates.first() {
            if let Some(part) = candidate.content.parts.first() {
                log::info!("prompt event: {}", event_description);
                log::info!("Extracted text: {}", part.text.trim());
                unsafe {
                    util::TAUNTS.taunt_list.push(part.text.trim().to_string());
                    util::TAUNTS.taunt_status = TauntStatus::NewTauntAvailable;
                };
                return Ok(part.text.trim().to_string());
            }
        }
    } else {
        log::info!(
            "Gemini API request failed, and Failed with status: {}",
            response.status()
        );
    }

    Ok(response_text)
}


use serde::{Deserialize, Serialize};
use serde_json::json;

// Add response structures
#[derive(Deserialize, Debug)]
struct Candidate {
    content: Content,
}

#[derive(Deserialize, Debug)]
struct Content {
    parts: Vec<ContentPart>,
}

#[derive(Deserialize, Debug)]
struct ContentPart {
    text: String,
}

#[derive(Deserialize, Debug)]
struct ApiResponse {
    candidates: Vec<Candidate>,
}

// pub async fn get_taunt(event_description: String) -> Result<String, Box<dyn std::error::Error>> {
//     let google_api_key = std::env::var("GEMINI_API_KEY_FINE_TUNED")
//         .unwrap_or_else(|_| "YOUR_API_KEY".to_string());
    
//     // Correct URL format for tuned models
//     let url = format!(
//         "https://generativelanguage.googleapis.com/v1/tunedModels/copy-of-copy-of-aor-tuning-uk3nmdlu547p:generateContent?key={}",
//         google_api_key
//     );

//     let prompt = format!("{}", event_description);

//     let body = json!({
//         "contents": [{
//             "parts": [{
//                 "text": prompt
//             }]
//         }]
//     });

//     let client = reqwest::Client::new();
    
//     // Your existing taunt code
//     unsafe {
//         util::TAUNTS.taunt_count += 1;
//         util::TAUNTS.prev_taunt_time = std::time::SystemTime::now();
//     }

//     let response = client
//         .post(&url)
//         .header("Content-Type", "application/json")
//         .json(&body)
//         .send()
//         .await?;

//     // Parse the JSON response
//     let api_response: ApiResponse = response.json().await?;

//     // Extract response text
//     let response_text = api_response
//         .candidates
//         .first()
//         .and_then(|c| c.content.parts.first())
//         .map(|p| p.text.clone())
//         .unwrap_or_else(|| "No response generated".to_string());

//     Ok(response_text)
// }
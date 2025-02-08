use self::util::{get_valid_road_paths, AttackResponse, GameLog, ResultResponse};
use super::auth::session::AuthUser;
use super::defense::shortest_path::run_shortest_paths;
use super::defense::util::{
    AttackBaseResponse, MineTypeResponseWithoutBlockId,
};
use super::user::util::fetch_user;
use super::{error, PgPool, RedisPool};
use crate::api::attack::socket::{BuildingResponse, ResultType, SocketRequest, SocketResponse};
use crate::api::util::HistoryboardQuery;
use crate::constants::{GAME_AGE_IN_MINUTES, MAX_BOMBS_PER_ATTACK};
use crate::validator::state::State;
use crate::validator::util::DefenderDetails;
use actix_rt;
use actix_web::error::ErrorBadRequest;
use actix_web::web::{Data, Json};
use actix_web::{web, Error, HttpRequest, HttpResponse, Responder, Result};
use log;
use std::collections::HashMap;
use std::time;

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
) -> Result<impl Responder> {
    let attacker_id = user.0;

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

    let random_opponent_id = util::get_random_opponent_id(attacker_id, &mut conn, redis_conn)
        .map_err(|err| error::handle_error(err.into()))?;

    let opponent_id = if let Some(id) = random_opponent_id {
        id
    } else {
        log::info!("No opponent found for Attacker:{}", attacker_id);
        return Err(ErrorBadRequest("No opponent found"));
    };

    log::info!(
        "Opponent:{} found for Attacker:{}",
        opponent_id,
        attacker_id
    );

    //Fetch base details and shortest paths data
    let (map_id, opponent_base) =
        (util::get_opponent_base_details_for_attack(opponent_id, &mut conn, attacker_id))
            .map_err(|err| error::handle_error(err.into()))?;

    log::info!("Base details of Opponent:{} fetched", opponent_id);

    let obtainable_artifacts = util::artifacts_obtainable_from_base(map_id, &mut conn)
        .map_err(|err| error::handle_error(err.into()))?;

    log::info!(
        "Artifacts obtainable from opponent: {} base is {}",
        opponent_id,
        obtainable_artifacts
    );

    let user_details =
        fetch_user(&mut conn, opponent_id).map_err(|err| error::handle_error(err.into()))?;

    log::info!("User details fetched for Opponent:{}", opponent_id);

    //Create game
    let game_id = (util::add_game(attacker_id, opponent_id, map_id, &mut conn))
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

    let map =
        util::get_map_id(&defender_id, &mut conn).map_err(|err| error::handle_error(err.into()))?;

    let map_id = if let Some(map) = map {
        map
    } else {
        return Err(ErrorBadRequest("Invalid base"));
    };

    let shortest_paths =
        run_shortest_paths(&mut conn, map_id).map_err(|err| error::handle_error(err.into()))?;

    let defenders: Vec<DefenderDetails> = util::get_defenders(&mut conn, map_id, defender_id)
        .map_err(|err| error::handle_error(err.into()))?;

    let hut_defenders: HashMap<i32, DefenderDetails> =
        util::get_hut_defender(&mut conn, defender_id)
            .map_err(|err| error::handle_error(err.into()))?;

    log::info!("hut defender map: {:?}", hut_defenders);

    let mines =
        util::get_mines(&mut conn, map_id).map_err(|err| error::handle_error(err.into()))?;

    let buildings =
        util::get_buildings(&mut conn, map_id).map_err(|err| error::handle_error(err.into()))?;

    let roads =
        get_valid_road_paths(map_id, &mut conn).map_err(|err| error::handle_error(err.into()))?;

    let bomb_types =
        util::get_bomb_types(&mut conn).map_err(|err| error::handle_error(err.into()))?;

    let attacker_type =
        util::get_attacker_types(&mut conn).map_err(|err| error::handle_error(err.into()))?;

    let attacker_user_details =
        fetch_user(&mut conn, attacker_id).map_err(|err| error::handle_error(err.into()))?;

    let defender_user_details =
        fetch_user(&mut conn, defender_id).map_err(|err| error::handle_error(err.into()))?;

    let defender_base_details =
        util::get_opponent_base_details_for_simulation(defender_id, &mut conn)
            .map_err(|err| error::handle_error(err.into()))?;

    if attacker_user_details.is_none() || defender_user_details.is_none() {
        return Err(ErrorBadRequest("User details not found"));
    }

    if util::add_game_id_to_redis(attacker_id, defender_id, game_id, redis_conn).is_err() {
        println!("Cannot add game:{} to redis", game_id);
        return Err(ErrorBadRequest("Internal Server Error"));
    }

    let mut damaged_buildings: Vec<BuildingResponse> = Vec::new();

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
                    damaged_buildings: None,
                    total_damage_percentage: None,
                    is_sync: false,
                    shoot_bullets: None,
                    is_game_over: true,
                    message: Some("Connection timed out".to_string()),
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
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let response = util::fetch_attack_history(user_id, page, limit, &mut conn)
        .map_err(|err| error::handle_error(err.into()))?;
    Ok(web::Json(response))
}

async fn get_top_attacks(pool: web::Data<PgPool>, user: AuthUser) -> Result<impl Responder> {
    let user_id = user.0;
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let response = util::fetch_top_attacks(user_id, &mut conn)
        .map_err(|err| error::handle_error(err.into()))?;
    Ok(web::Json(response))
}

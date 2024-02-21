use self::util::{get_valid_road_paths, AttackResponse, GameLog, ResultResponse};
use super::auth::session::AuthUser;
use super::defense::shortest_path::run_shortest_paths;
use super::defense::util::{
    AttackBaseResponse, DefenseResponse, MineTypeResponseWithoutBlockId, SimulationBaseResponse,
};
use super::user::util::fetch_user;
use super::{error, PgPool, RedisPool};
use crate::api::attack::socket::{ResultType, SocketRequest, SocketResponse};
use crate::api::util::HistoryboardQuery;
use crate::constants::{GAME_AGE_IN_MINUTES, MAX_BOMBS_PER_ATTACK};
use crate::models::{AttackerType, User};
use crate::validator::state::State;
use crate::validator::util::{BombType, BuildingDetails, DefenderDetails, MineDetails};
use crate::validator::util::{Coords, SourceDestXY};
use actix_rt;
use actix_web::error::ErrorBadRequest;
use actix_web::web::{Data, Json};
use actix_web::{web, Error, HttpRequest, HttpResponse, Responder, Result};
use log;
use std::collections::{HashMap, HashSet};
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

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let redis_conn = redis_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;

    let random_opponent_id = web::block(move || {
        Ok(util::get_random_opponent_id(
            attacker_id,
            &mut conn,
            redis_conn,
        )?) as anyhow::Result<Option<i32>>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    let opponent_id = if let Some(id) = random_opponent_id {
        id
    } else {
        return Err(ErrorBadRequest("No opponent found"));
    };

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;

    //Fetch base details and shortest paths data
    let (map_id, opponent_base) = web::block(move || {
        Ok(util::get_opponent_base_details_for_attack(
            opponent_id,
            &mut conn,
        )?) as anyhow::Result<(i32, DefenseResponse)>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;

    let obtainable_artifacts = web::block(move || {
        Ok(util::artifacts_obtainable_from_base(map_id, &mut conn)?) as anyhow::Result<i32>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;

    println!("obatain {}", obtainable_artifacts);

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;

    let user_details =
        web::block(move || Ok(fetch_user(&mut conn, opponent_id)?) as anyhow::Result<Option<User>>)
            .await?
            .map_err(|err| error::handle_error(err.into()))?;

    //Create game
    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;
    let game_id = web::block(move || {
        Ok(util::add_game(attacker_id, opponent_id, map_id, &mut conn)?) as anyhow::Result<i32>
    })
    .await?
    .map_err(|err| error::handle_error(err.into()))?;
    println!("Added game: {}", game_id);

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

    println!("Sent game id {} to frontend", game_id);
    Ok(Json(response))
}

async fn socket_handler(
    pool: web::Data<PgPool>,
    redis_pool: Data<RedisPool>,
    req: HttpRequest,
    body: web::Payload,
) -> Result<HttpResponse, Error> {
    println!("error bug fs");
    let query_params = req.query_string().split('&').collect::<Vec<&str>>();
    let user_token = query_params[0].split('=').collect::<Vec<&str>>()[1];
    let attack_token = query_params[1].split('=').collect::<Vec<&str>>()[1];

    let attacker_id =
        util::decode_user_token(user_token).map_err(|err| error::handle_error(err.into()))?;
    let attack_token_data =
        util::decode_attack_token(attack_token).map_err(|err| error::handle_error(err.into()))?;
    let game_id = attack_token_data.game_id;

    println!("Received game id {} from frontend", game_id);

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;

    if attacker_id != attack_token_data.attacker_id {
        println!("Attacker:{} is not authorised", attacker_id);
        return Err(ErrorBadRequest("User not authorised"));
    }

    let defender_id = attack_token_data.defender_id;
    if attacker_id == defender_id {
        println!("Attacker:{} can't attack yourself", attacker_id);
        return Err(ErrorBadRequest("Can't attack yourself"));
    }

    let mut redis_conn = redis_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;

    if let Ok(Some(_)) = util::get_game_id_from_redis(attacker_id, &mut redis_conn, true) {
        println!("Attacker:{} has an ongoing game", attacker_id);
        return Err(ErrorBadRequest("Attacker has an ongoing game"));
    }

    if let Ok(Some(_)) = util::get_game_id_from_redis(defender_id, &mut redis_conn, false) {
        println!("Defender:{} has an ongoing game", defender_id);
        return Err(ErrorBadRequest("Defender has an ongoing game"));
    }

    println!("Checking if there are incomplte games");
    if util::check_and_remove_incomplete_game(&attacker_id, &defender_id, &game_id, &mut conn)
        .is_err()
    {
        println!(
            "Failed to remove incomplete games for Attacker:{} and Defender:{}",
            attacker_id, defender_id
        );
    }

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
        println!("Defender:{}'s base is invalid", defender_id);
        return Err(ErrorBadRequest("Invalid base"));
    };

    let mut conn = pool.get().map_err(|err| error::handle_error(err.into()))?;

    let shortest_paths = web::block(move || {
        Ok(run_shortest_paths(&mut conn, map_id)?) as anyhow::Result<HashMap<SourceDestXY, Coords>>
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

    let (response, session, mut msg_stream) = actix_ws::handle(&req, body)?;

    let mut session_clone1 = session.clone();
    let mut session_clone2 = session.clone();

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
        return Err(ErrorBadRequest("Internal Server Error"));
    }

    let redis_conn = redis_pool
        .get()
        .map_err(|err| error::handle_error(err.into()))?;

    if util::add_game_id_to_redis(attacker_id, defender_id, game_id, redis_conn).is_err() {
        println!("Cannot add game:{} to redis", game_id);
        return Err(ErrorBadRequest("Internal Server Error"));
    }

    println!("Added game:{} to redis", game_id);

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

    actix_rt::spawn(async move {
        let mut game_state = State::new(attacker_id, defender_id, defenders, mines, buildings);
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
                                        println!("Game over. Terminating the socket...");
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                        if (session_clone1.clone().close(None).await).is_err() {
                                            println!("Error closing the socket connection");
                                        }
                                        if util::terminate_game(
                                            game_logs,
                                            &mut conn,
                                            &mut redis_conn,
                                        )
                                        .is_err()
                                        {
                                            println!("Error terminating the game 1");
                                        }
                                    } else if response.result_type == ResultType::MinesExploded {
                                        println!("MinesExploded response sent");
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    } else if response.result_type == ResultType::DefendersDamaged {
                                        println!("DefendersDamaged response sent");
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    } else if response.result_type == ResultType::DefendersTriggered
                                    {
                                        println!("DefendersTriggered response sent");
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    } else if response.result_type == ResultType::BuildingsDamaged {
                                        println!("BuildingsDamaged response sent");
                                        if util::deduct_artifacts_from_building(
                                            response.damaged_buildings.unwrap(),
                                            &mut conn,
                                        )
                                        .is_err()
                                        {
                                            println!("Failed to deduct artifacts from building");
                                        }
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    } else if response.result_type == ResultType::PlacedAttacker {
                                        println!("PlacedAttacker response sent");
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    } else if response.result_type == ResultType::Nothing {
                                        // println!("Nothing response sent");
                                        if session_clone1.text(response_json).await.is_err() {
                                            return;
                                        }
                                    }
                                } else {
                                    println!("Error serializing JSON");
                                    if session_clone1.text("Error serializing JSON").await.is_err()
                                    {
                                        return;
                                    }
                                }
                            }
                            Some(Err(err)) => {
                                println!("Error handling game: {:?}", err);
                            }
                            None => {
                                // Handle the case where game_handler returned None (e.g., ActionType::PlaceAttacker)
                                // Add appropriate logic here based on the requirements.
                                println!("All fine");
                            }
                        }
                    } else {
                        println!("Error parsing JSON");
                        if session_clone1.text("Error parsing JSON").await.is_err() {
                            return;
                        }
                    }
                }
                Message::Close(s) => {
                    println!("Received close: {:?}", s);
                    if util::terminate_game(game_logs, &mut conn, &mut redis_conn).is_err() {
                        println!("Error terminating the game 2");
                    }
                    break;
                }
                _ => (),
            }
        }
    });

    actix_rt::spawn(async move {
        let timeout_duration = time::Duration::from_secs((GAME_AGE_IN_MINUTES as u64) * 60);
        let last_activity = time::Instant::now();

        println!("Started timer");
        loop {
            actix_rt::time::sleep(time::Duration::from_secs(1)).await;

            if time::Instant::now() - last_activity > timeout_duration {
                let response_json = serde_json::to_string(&SocketResponse {
                    frame_number: 0,
                    result_type: ResultType::GameOver,
                    is_alive: None,
                    attacker_health: None,
                    exploded_mines: None,
                    defender_damaged: None,
                    damaged_buildings: None,
                    total_damage_percentage: None,
                    is_sync: false,
                    is_game_over: true,
                    message: Some("Connection timed out".to_string()),
                })
                .unwrap();
                if session_clone2.text(response_json).await.is_err() {
                    return;
                }

                println!("Connection timed out");

                break;
            }
        }
    });

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

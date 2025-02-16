use std::collections::{HashMap, HashSet};

use crate::{
    api::attack::socket::{ActionType, BaseItemsDamageResponse, ResultType, SocketRequest, SocketResponse, DirectionType},
    api::game::util::{AttackLog, EventResponse, EventType, EventLog},
    constants::COMPANION_BOT_RANGE,
    models::AttackerType,
    validator::util::{Coords, SourceDestXY},
};
use anyhow::{Ok, Result};
use util::{Companion, CompanionResult, MineResponse, Path};

use self::{
    state::State,
    util::{send_terminate_game_message, Attacker, BombType, DefenderReturnType},
};

pub mod error;
pub mod state;
pub mod util;

pub fn game_handler(
    attacker_type: &HashMap<i32, AttackerType>,
    socket_request: SocketRequest,
    _game_state: &mut State,
    _shortest_path: &HashMap<SourceDestXY, Path>,
    _roads: &HashSet<(i32, i32)>,
    _bomb_types: &Vec<BombType>,
    attack_log: &mut AttackLog,
) -> Option<Result<SocketResponse>> {
    let defender_damaged_result: DefenderReturnType;
    let exploded_mines_result: Vec<MineResponse>;
    let base_items_damaged_result: BaseItemsDamageResponse;
    // log::info!("ATTACKER DIRECTION: {:?}", socket_request.attacker_direction);
    match socket_request.action_type {
        ActionType::PlaceAttacker => {
            _game_state.update_frame_number(socket_request.frame_number);
            let event_response = EventResponse {
                attacker_initial_position: socket_request.current_position,
                attacker_type: socket_request.attacker_id,
                mine_details: None,
                hut_defender_details: None,
                defender_details: None,
                companion_result: None,
                bullets_details: None,
                event_type: EventType::PlaceAttacker,
                bomb_type: socket_request.bomb_id,
                bomb_details: None,
            };
            attack_log.game_log.push(EventLog {
                event: event_response.clone(),
                frame_no: socket_request.frame_number,
                // date: chrono::Utc::now().naive_utc(),
            });

            if let Some(attacker_id) = socket_request.attacker_id {
                let attacker: AttackerType = attacker_type.get(&attacker_id).unwrap().clone();
                _game_state.place_attacker(Attacker {
                    id: attacker.id,
                    // path_in_current_frame: Vec::new(),
                    attacker_pos: socket_request.current_position.unwrap(),
                    attacker_health: attacker.max_health,
                    attacker_speed: attacker.speed,
                    bombs: Vec::new(),
                    trigger_defender: false,
                    bomb_count: attacker.amt_of_emps,
                    attacker_direction: DirectionType::stationary,
                });

                for bomb_type in _bomb_types {
                    if let Some(bomb_id) = socket_request.bomb_id {
                        if bomb_type.id == bomb_id {
                            _game_state.set_bombs(bomb_type.clone(), attacker.amt_of_emps);
                        }
                    }
                }
            }

            // _game_state.set_mines(mine_positions);

            if _game_state.in_validation.is_invalidated {
                return Some(Ok(send_terminate_game_message(
                    socket_request.frame_number,
                    _game_state.in_validation.message.clone(),
                )));
            }

            for defender in _game_state.defenders.iter() {
                log::info!(
                    "defender id : {} , position x {}, y {} ",
                    defender.map_space_id,
                    defender.defender_pos.x,
                    defender.defender_pos.y
                );
            }

            let attacker_health = _game_state
                .attacker
                .as_ref()
                .map(|attacker| attacker.attacker_health);

            return Some(Ok(SocketResponse {
                frame_number: socket_request.frame_number,
                result_type: ResultType::PlacedAttacker,
                is_alive: Some(true),
                attacker_health,
                exploded_mines: None,
                // triggered_defenders: None,
                defender_damaged: None,
                damaged_base_items: None,
                hut_triggered: false,
                hut_defenders: None,
                total_damage_percentage: Some(_game_state.damage_percentage),
                is_sync: false,
                is_game_over: false,
                shoot_bullets: None,
                message: Some(String::from(
                    "Place Attacker, set attacker and bomb response",
                )),
                companion: None,
            }));
        }
        ActionType::PlaceCompanion => {
            _game_state.update_frame_number(socket_request.frame_number);

            if let Some(attacker_id) = socket_request.attacker_id {
                let attacker: AttackerType = attacker_type.get(&attacker_id).unwrap().clone();
                _game_state.place_companion(Companion {
                    id: attacker.id,
                    path_in_current_frame: Vec::new(),
                    companion_pos: socket_request.current_position.unwrap(),
                    companion_health: attacker.max_health,
                    companion_speed: attacker.speed,
                    bombs: Vec::new(),
                    trigger_defender: false,
                    bomb_count: attacker.amt_of_emps,
                    range: COMPANION_BOT_RANGE,
                    target_building: None,
                    target_defender: None,
                    target_tile: None,
                    current_target: None,
                    reached_dest: false,
                    last_attack_tick: 0,
                    attack_interval: 10,
                    damage: 30,
                });

                for bomb_type in _bomb_types {
                    if let Some(bomb_id) = socket_request.bomb_id {
                        if bomb_type.id == bomb_id {
                            _game_state
                                .set_companion_bombs(bomb_type.clone(), attacker.amt_of_emps);
                        }
                    }
                }
            }

            let event_response = EventResponse {
                attacker_initial_position: None,
                attacker_type: None,
                companion_result: None,
                hut_defender_details: None,
                defender_details: None,
                mine_details: None,
                event_type: EventType::PlaceCompanion,
                bullets_details: None,
                bomb_type: None,
                bomb_details: None,
            };
            attack_log.game_log.push(EventLog {
                event: event_response.clone(),
                frame_no: socket_request.frame_number,
                // date: chrono::Utc::now().naive_utc(),
            });

            return Some(Ok(SocketResponse {
                frame_number: socket_request.frame_number,
                result_type: ResultType::PlacedCompanion,
                is_alive: Some(true),

                attacker_health: None,
                exploded_mines: None,
                // triggered_defenders: None,
                defender_damaged: None,
                damaged_base_items: None,
                hut_triggered: false,
                hut_defenders: None,
                total_damage_percentage: Some(_game_state.damage_percentage),
                is_sync: false,
                is_game_over: false,
                message: Some(String::from("Placed companion")),
                companion: None,
                shoot_bullets: None,
            }));
        }

        ActionType::MoveAttacker => {
            if let Some(attacker_id) = socket_request.attacker_id {
                let attacker: AttackerType = attacker_type.get(&attacker_id).unwrap().clone();
                // let attacker_delta: Vec<Coords> = socket_request.attacker_path;
                // let attacker_delta_clone = attacker_delta.clone();

                let _attacker_result = _game_state.attacker_movement(
                    socket_request.frame_number,
                    _roads,
                    Attacker {
                        id: attacker.id,
                        // path_in_current_frame: attacker_delta.clone(),
                        attacker_pos: socket_request.current_position.unwrap(),
                        attacker_health: attacker.max_health,
                        attacker_speed: attacker.speed,
                        bombs: Vec::new(),
                        trigger_defender: false,
                        bomb_count: attacker.amt_of_emps,
                        attacker_direction: socket_request.attacker_direction.unwrap_or(DirectionType::none),
                    },
                    attack_log,
                );

                // let attacker_result_clone = attacker_result.clone().unwrap();

                defender_damaged_result = _game_state
                    .defender_movement_one_tick(socket_request.current_position?, _shortest_path, attack_log, socket_request.frame_number);

                let mut is_attacker_alive = true;

                if let Some(attacker) = &_game_state.attacker {
                    if attacker.attacker_health == 0 {
                        is_attacker_alive = false;
                    }
                }

                if _game_state.in_validation.is_invalidated {
                    return Some(Ok(send_terminate_game_message(
                        socket_request.frame_number,
                        _game_state.in_validation.message.clone(),
                    )));
                }

                let spawn_result = _game_state
                    .spawn_hut_defender(
                        _roads,
                        // Attacker {
                        //     id: attacker.id,
                        //     path_in_current_frame: attacker_delta_clone.clone(),
                        //     attacker_pos: socket_request.start_position.unwrap(),
                        //     attacker_health: attacker.max_health,
                        //     attacker_speed: attacker.speed,
                        //     bombs: Vec::new(),
                        //     trigger_defender: false,
                        //     bomb_count: attacker.amt_of_emps,
                        // },
                    )
                    .unwrap();

                _game_state.activate_sentry();
                let shoot_bullets = _game_state.shoot_bullets();
                if _game_state.attacker.is_some() || _game_state.attacker.is_some() {
                    _game_state.cause_bullet_damage();
                }
                if !shoot_bullets.is_empty() {
                    let event_response = EventResponse {
                        attacker_initial_position: None,
                        attacker_type: None,
                        companion_result: None,
                        hut_defender_details: None,
                        mine_details: None,
                        event_type: EventType::BulletShooting,
                        defender_details: None,
                        bullets_details: Some(shoot_bullets.clone()),
                        bomb_type: None,
                        bomb_details: None,
                    };
                    attack_log.game_log.push(EventLog {
                        event: event_response.clone(),
                        frame_no: socket_request.frame_number,
                        // // date: chrono::Utc::now().naive_utc(),
                    });
                }

                let companion_res = _game_state
                    .move_companion(_roads, _shortest_path, attack_log, socket_request.frame_number)
                    .unwrap_or(CompanionResult {
                        current_target: None,
                        map_space_id: -1,
                        current_target_tile: None,
                        is_alive: false,
                        health: -1,
                        building_damaged: None,
                        defender_damaged: None,
                    });

                let initially_triggered_defenders_count = _game_state
                    .defenders
                    .clone()
                    .iter()
                    .filter(|d| d.target_id.is_some())
                    .count();
                _game_state.defender_trigger();
                let triggered_defenders_count = _game_state
                    .defenders
                    .iter()
                    .filter(|d| d.target_id.is_some())
                    .count();
                
                let hut_triggered = !spawn_result.is_empty();

                let result_type = if hut_triggered {
                    let event_response = EventResponse {
                        attacker_initial_position: None,
                        attacker_type: None,
                        event_type: EventType::HutDefenderSpawn,
                        mine_details: None,
                        bullets_details: None,
                        companion_result: Some(companion_res.clone()),
                        hut_defender_details: None,
                        defender_details: None,
                        bomb_type: None,
                        bomb_details: None,
                    };
                    attack_log.game_log.push(EventLog {
                        event: event_response.clone(),
                        frame_no: socket_request.frame_number,
                        // date: chrono::Utc::now().naive_utc(),
                    });
                    ResultType::SpawnHutDefender
                } else if defender_damaged_result.clone().defender_response.len() > 0 {
                    ResultType::DefendersDamaged
                } else {
                    ResultType::BuildingsDamaged
                };

                let buildings_damaged =
                    if let Some(building_damaged) = &companion_res.building_damaged {
                        vec![building_damaged.clone()]
                    } else {
                        Vec::new()
                    };

                let defenders_damaged =
                    if let Some(defender_damaged) = &companion_res.defender_damaged {
                        vec![defender_damaged.clone()]
                    } else {
                        Vec::new()
                    };

                if initially_triggered_defenders_count != triggered_defenders_count {
                    let event_response = EventResponse {
                        attacker_initial_position: None,
                        attacker_type: None,
                        event_type: EventType::DefenderActivated,
                        bullets_details: None,
                        companion_result: None,
                        mine_details: None,
                        hut_defender_details: None,
                        defender_details: Some(defender_damaged_result.clone().defender_response),
                        bomb_details: None,
                        bomb_type: None,
                    };
                    attack_log.game_log.push(EventLog {
                        event: event_response.clone(),
                        frame_no: socket_request.frame_number,
                        // date: chrono::Utc::now().naive_utc(),
                    });
                }
                let damaged_base_items = Some(BaseItemsDamageResponse {
                    buildings_damaged,
                    defenders_damaged,
                });

                let response = SocketResponse {
                    frame_number: socket_request.frame_number,
                    result_type,
                    is_alive: Some(is_attacker_alive),
                    attacker_health: Some(defender_damaged_result.clone().attacker_health),
                    exploded_mines: None,
                    // triggered_defenders: Some(defender_damaged_result.clone().defender_response),
                    defender_damaged: Some(defender_damaged_result.clone().defender_response),
                    damaged_base_items,
                    hut_triggered,
                    hut_defenders: Some(spawn_result),
                    total_damage_percentage: Some(_game_state.damage_percentage),
                    is_sync: false,
                    is_game_over: false,
                    shoot_bullets: Some(shoot_bullets),
                    message: Some(String::from("Movement Response")),
                    companion: Some(companion_res),
                };
                return Some(Ok(response));
            }
        }
        ActionType::IsMine => {
            // is_mine
            let start_pos: Option<Coords> = socket_request.current_position;
            exploded_mines_result = _game_state.mine_blast(start_pos);

            let mut bool_temp = false;
            if !exploded_mines_result.is_empty() {
                bool_temp = true;
            }
            let result_type = if bool_temp {
                let event_response = EventResponse {
                    attacker_initial_position: None,
                    attacker_type: None,
                    event_type: EventType::MineBlast,
                    bullets_details: None,
                    mine_details: Some(exploded_mines_result.clone()),
                    companion_result: None,
                    hut_defender_details: None,
                    defender_details: None,
                    bomb_type: None,
                    bomb_details: None,
                };
                attack_log.game_log.push(EventLog {
                    event: event_response.clone(),
                    frame_no: socket_request.frame_number,
                    // date: chrono::Utc::now().naive_utc(),
                });
                ResultType::MinesExploded
            } else {
                ResultType::Nothing
            };

            let mut is_attacker_alive = true;

            if let Some(attacker) = &_game_state.attacker {
                if attacker.attacker_health == 0 {
                    is_attacker_alive = false;
                }
            }

            if _game_state.in_validation.is_invalidated {
                return Some(Ok(send_terminate_game_message(
                    socket_request.frame_number,
                    _game_state.in_validation.message.clone(),
                )));
            }

            let attacker_health = _game_state
                .attacker
                .as_ref()
                .map(|attacker| attacker.attacker_health);

            return Some(Ok(SocketResponse {
                frame_number: socket_request.frame_number,
                result_type,
                is_alive: Some(is_attacker_alive),
                attacker_health,
                exploded_mines: Some(exploded_mines_result),
                // triggered_defenders: None,
                defender_damaged: None,
                damaged_base_items: None,
                hut_triggered: false,
                hut_defenders: None,
                total_damage_percentage: Some(_game_state.damage_percentage),
                is_sync: false,
                is_game_over: false,
                shoot_bullets: None,
                message: Some(String::from("Is Mine Response")),
                companion: None,
            }));
        }
        ActionType::PlaceBombs => {
            // let attacker_delta: Vec<Coords> = socket_request.attacker_path.clone();
            let current_pos = socket_request.current_position.unwrap();
            let bomb_coords = socket_request.bomb_position;

            if _game_state.bombs.total_count == 0 {
                return Some(Ok(send_terminate_game_message(
                    socket_request.frame_number,
                    "No bombs left".to_string(),
                )));
            }

            // for coord in attacker_delta.clone() {
            //     let mut direction = Direction::Up;

            //     let prev_pos = _game_log.e.last().unwrap().coords;
            //     if prev_pos.x < coord.x {
            //         direction = Direction::Down;
            //     } else if prev_pos.x > coord.x {
            //         direction = Direction::Up;
            //     } else if prev_pos.y < coord.y {
            //         direction = Direction::Left;
            //     } else if prev_pos.y > coord.y {
            //         direction = Direction::Right;
            //     }

            //     let event_response = EventResponse {
            //         attacker_id: None,
            //         bomb_id: None,
            //         coords: coord,
            //         direction,
            //         is_bomb: coord == bomb_coords,
            //     };

            //     _game_log.e.push(event_response.clone());
            // }

            base_items_damaged_result = _game_state.place_bombs(current_pos, bomb_coords);

            attack_log.result.bombs_used += 1;
            attack_log.result.damage_done = _game_state.damage_percentage as i32;
            attack_log.result.artifacts_collected = _game_state.artifacts;

            let mut bool_temp = false;
            if !base_items_damaged_result.buildings_damaged.is_empty()
                || !base_items_damaged_result.defenders_damaged.is_empty()
            {
                bool_temp = true;
            }

            let event_response = EventResponse {
                attacker_initial_position: None,
                attacker_type: None,
                event_type: EventType::PlaceBomb,
                bullets_details: None,
                mine_details: None,
                bomb_details: Some(base_items_damaged_result.clone()),
                companion_result: None,
                hut_defender_details: None,
                defender_details: None,
                bomb_type: None,
            };
            attack_log.game_log.push(EventLog {
                event: event_response.clone(),
                frame_no: socket_request.frame_number,
                // date: chrono::Utc::now().naive_utc(),
            });
            let result_type = if bool_temp {
                ResultType::BuildingsDamaged
            } else {
                ResultType::Nothing
            };

            if _game_state.in_validation.is_invalidated {
                return Some(Ok(send_terminate_game_message(
                    socket_request.frame_number,
                    _game_state.in_validation.message.clone(),
                )));
            }

            let attacker_health = _game_state
                .attacker
                .as_ref()
                .map(|attacker| attacker.attacker_health);

            return Some(Ok(SocketResponse {
                frame_number: socket_request.frame_number,
                result_type,
                is_alive: Some(true),
                attacker_health,
                exploded_mines: None,
                // triggered_defenders: None,
                defender_damaged: None,
                damaged_base_items: Some(base_items_damaged_result),
                hut_triggered: false,
                hut_defenders: None,
                total_damage_percentage: Some(_game_state.damage_percentage),
                is_sync: false,
                is_game_over: false,
                shoot_bullets: None,
                message: Some(String::from("Place Bomb Response")),
                companion: None,
            }));
        }
        ActionType::Idle => {
            let attacker_health = _game_state
                .attacker
                .as_ref()
                .map(|attacker| attacker.attacker_health);
            return Some(Ok(SocketResponse {
                frame_number: socket_request.frame_number,
                result_type: ResultType::Nothing,
                is_alive: Some(true),
                attacker_health,
                exploded_mines: None,
                // triggered_defenders: None,
                defender_damaged: None,
                damaged_base_items: None,
                hut_triggered: false,
                hut_defenders: None,
                total_damage_percentage: Some(_game_state.damage_percentage),
                is_sync: false,
                is_game_over: false,
                shoot_bullets: None,
                message: Some(String::from("Idle Response")),
                companion: None,
            }));
        }
        ActionType::Terminate => {
            let attacker_health = _game_state
                .attacker
                .as_ref()
                .map(|attacker| attacker.attacker_health);
            let socket_response = SocketResponse {
                frame_number: socket_request.frame_number,
                result_type: ResultType::GameOver,
                is_alive: None,
                attacker_health,
                exploded_mines: None,
                // triggered_defenders: None,
                defender_damaged: None,
                damaged_base_items: None,
                hut_triggered: false,
                hut_defenders: None,
                total_damage_percentage: Some(_game_state.damage_percentage),
                is_sync: false,
                is_game_over: true,
                shoot_bullets: None,
                message: Some(String::from("Game over")),
                companion: None,
            };
            let event_response = EventResponse {
                attacker_initial_position: None,
                attacker_type: None,
                event_type: EventType::GameOver,
                bullets_details: None,
                companion_result: None,
                mine_details: None,
                hut_defender_details: None,
                defender_details: None,
                bomb_details: None,
                bomb_type: None,
            };
            attack_log.game_log.push(EventLog {
                event: event_response.clone(),
                frame_no: socket_request.frame_number,
                // date: chrono::Utc::now().naive_utc(),
            });
            return Some(Ok(socket_response));
        }
        ActionType::SelfDestruct => {
            _game_state.self_destruct();
            let attacker_health = _game_state
                .attacker
                .as_ref()
                .map(|attacker| attacker.attacker_health);
            let socket_response = SocketResponse {
                frame_number: socket_request.frame_number,
                result_type: ResultType::Nothing,
                is_alive: Some(false),
                attacker_health,
                exploded_mines: None,
                // triggered_defenders: None,
                defender_damaged: Some(Vec::new()),
                damaged_base_items: Some(BaseItemsDamageResponse {
                    buildings_damaged: Vec::new(),
                    defenders_damaged: Vec::new(),
                }),
                hut_triggered: false,
                hut_defenders: None,
                total_damage_percentage: Some(_game_state.damage_percentage),
                is_sync: false,
                is_game_over: false,
                shoot_bullets: None,
                message: Some(String::from("Self Destructed")),
                companion: None,
            };
            let event_response = EventResponse {
                attacker_initial_position: None,
                attacker_type: None,
                event_type: EventType::SelfDestruction,
                bullets_details: None,
                companion_result: None,
                mine_details: None,
                hut_defender_details: None,
                defender_details: None,
                bomb_details: None,
                bomb_type: None,
            };
            attack_log.game_log.push(EventLog {
                event: event_response.clone(),
                frame_no: socket_request.frame_number,
                // date: chrono::Utc::now().naive_utc(),
            });
            return Some(Ok(socket_response));
        }
    }
    None
}

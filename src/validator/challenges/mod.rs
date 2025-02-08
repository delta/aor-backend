use actix::System;

use crate::{api::game, constants::GAME_AGE_IN_MINUTES};

use super::{
    state::State,
    util::{Attacker, BuildingDetails, Challenge, ChallengeType, FallGuys, InValidation},
};
use std::{collections::HashSet, time::SystemTime};
pub mod util;

pub fn attacker_movement_challenge_handle(
    game_state: &mut State,
    roads: &HashSet<(i32, i32)>,
    attacker_current: &Attacker,
) {
    if let Some(ref mut challenge) = game_state.challenge {
        if let Some(challenge_type) = challenge.challenge_type {
            match challenge_type {
                ChallengeType::Maze => {
                    if let Some(maze) = challenge.maze.as_mut() {
                        let attacker = game_state.attacker.as_ref().unwrap();
                        let mut collided: i32 = -1;
                        for (i, building) in game_state.buildings.iter().enumerate() {
                            if building.name == "coin"
                                && attacker.attacker_pos.x == building.tile.x
                                && attacker.attacker_pos.y == building.tile.y
                            {
                                challenge.score += 1;
                                collided = i as i32;
                                maze.coins += 1;
                                break;
                            }
                        }
                        if collided != -1 {
                            game_state.buildings.remove(collided as usize);
                        }
                        if attacker_current.attacker_pos.x == challenge.end_tile.x
                            && attacker_current.attacker_pos.y == challenge.end_tile.y
                        {
                            let time_elapsed = SystemTime::now()
                                .duration_since(maze.start_time)
                                .expect("Time went backwards")
                                .as_secs();
                            let score_increment =
                                GAME_AGE_IN_MINUTES as i32 * 60 - time_elapsed as i32;
                            challenge.score += score_increment;
                            challenge.challenge_completed = true;
                            game_state.in_validation = InValidation {
                                message: "Maze Challenge Completed".to_string(),
                                is_invalidated: true,
                            }
                        }
                    }
                }
                ChallengeType::FallGuys => {
                    if let Some(fall_guys) = challenge.fall_guys.as_mut() {
                        if game_state.frame_no
                            > fall_guys.last_intensity_update_tick
                                + fall_guys.update_intensity_interval
                        {
                            for building in game_state.buildings.iter_mut() {
                                if building.name == "Defender_Hut" {
                                    building.range += fall_guys.hut_range_increment;
                                    building.frequency += fall_guys.hut_frequency_increment;
                                } else if building.name == "Sentry" {
                                    building.range += fall_guys.sentry_range_increment;
                                    building.frequency += fall_guys.sentry_frequency_increment;
                                }
                            }
                            fall_guys.last_intensity_update_tick = game_state.frame_no;
                        }

                        let attacker_pos = game_state.attacker.as_ref().unwrap().attacker_pos;

                        if attacker_pos.x == challenge.end_tile.x
                            && attacker_pos.y == challenge.end_tile.y
                        {
                            challenge.challenge_completed = true;
                            game_state.in_validation = InValidation {
                                is_invalidated: true,
                                message: "Fall Guys challenge completed".to_string(),
                            }
                        }

                        if attacker_pos.x == challenge.start_tile.x
                            && attacker_pos.y == challenge.start_tile.y
                            && challenge.score > 10
                        {
                            challenge.challenge_completed = true;
                            game_state.in_validation = InValidation {
                                is_invalidated: true,
                                message: "Fall Guys challenge completed".to_string(),
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn bomb_blast_fallguys_handle(
    challenge: &mut Option<Challenge>,
    damage_buildings: i32,
    building: &BuildingDetails,
) {
    if let Some(challenge) = challenge {
        if let Some(challenge_type) = challenge.challenge_type {
            if challenge_type == ChallengeType::FallGuys {
                if let Some(ref mut fallguys) = challenge.fall_guys {
                    if building.name == "Treasury1" {
                        challenge.score += damage_buildings * fallguys.multipliers.treasury_level_1;
                    } else if building.name == "Treasury2" {
                        challenge.score += damage_buildings * fallguys.multipliers.treasury_level_2;
                    } else if building.name == "Treasury3" {
                        challenge.score += damage_buildings * fallguys.multipliers.treasury_level_3;
                    }
                }
            }
        }
    }
}

pub fn maze_place_attacker_handle(challenge: &mut Option<Challenge>) {
    if let Some(challenge) = challenge {
        if let Some(challenge_type) = challenge.challenge_type {
            if challenge_type == ChallengeType::Maze {
                if let Some(ref mut maze) = challenge.maze {
                    maze.start_time = SystemTime::now();
                }
            }
        }
    }
}

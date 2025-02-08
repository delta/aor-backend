use crate::api::game;

use super::{
    state::State,
    util::{Attacker, ChallengeType, InValidation},
};
use std::collections::HashSet;
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
                                break;
                            }
                        }
                        if collided != -1 {
                            game_state.buildings.remove(collided as usize);
                        }
                        if attacker_current.attacker_pos.x == challenge.end_tile.x
                            && attacker_current.attacker_pos.y == challenge.end_tile.y
                        {
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
                    }
                }
            }
        }
    }
}

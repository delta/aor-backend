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
                        for building in &game_state.buildings {
                            if building.name == "Coin" {
                                challenge.score += 1;
                            }
                        }
                        if attacker_current.attacker_pos.x == maze.end_tile.x
                            && attacker_current.attacker_pos.y == maze.end_tile.y
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
                    }
                }
            }
        }
    }
}

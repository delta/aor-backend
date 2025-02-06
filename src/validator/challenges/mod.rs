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
                    for building in &game_state.buildings {
                        if building.name == "Coin" {
                            challenge.score += 1;
                        }
                    }
                    if attacker_current.attacker_pos.x == challenge.maze.end_tile.x
                        && attacker_current.attacker_pos.y == challenge.maze.end_tile.y
                    {
                        challenge.challenge_completed = true;
                        game_state.in_validation = InValidation {
                            message: "Game Over".to_string(),
                            is_invalidated: true,
                        }
                    }
                }
            }
        }
    }
}

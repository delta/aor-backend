use std::collections::HashSet;

use maze::attacker_movement_handle;

use super::{
    state::State,
    util::{Attacker, ChallengeType, InValidation},
};

pub mod maze;
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
                    let is_game_over = attacker_movement_handle(
                        attacker_current,
                        challenge,
                        &game_state.buildings,
                    );
                    if is_game_over {
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

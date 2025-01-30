use crate::validator::util::{Attacker, BuildingDetails, Challenge};

pub fn attacker_movement_handle(
    attacker_current: &Attacker,
    challenge: &mut Challenge,
    buildings: &Vec<BuildingDetails>,
) -> bool {
    let mut is_game_over = false;
    for building in buildings {
        if building.name == "Coin" {
            challenge.score += 1;
        }
    }

    if attacker_current.attacker_pos.x == challenge.maze.end_tile.x
        && attacker_current.attacker_pos.y == challenge.maze.end_tile.y
    {
        is_game_over = true;
    }
    is_game_over
}

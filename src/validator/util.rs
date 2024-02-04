use crate::{constants::ROAD_ID, validator::state::State};
use serde::{Deserialize, Serialize};

// Structs present in the state
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Coords {
    pub x: i32,
    pub y: i32,
}

#[derive(Serialize)]
pub struct Bomb {
    pub id: i32,
    pub blast_radius: i32,
    pub damage: i32,
}

#[derive(Serialize)]
pub struct Attacker {
    pub id: i32,
    pub attacker_pos: Coords,
    pub attacker_health: i32,
    pub attacker_speed: i32,
    pub path_in_current_frame: Vec<Coords>,
    pub bomb: Bomb,
}

#[derive(Serialize)]
pub struct Defender {
    pub id: i32,
    pub radius: i32,
    pub speed: i32,
    pub damage: i32,
    pub defender_pos: Coords,
    pub is_alive: bool,
    pub damage_dealt: bool,
    pub target_id: Option<i32>,
    pub path_in_current_frame: Vec<Coords>,
}

// Structs for sending response
#[derive(Serialize)]
pub struct MineDetails {
    pub id: i32,
    pub pos: Coords,
    pub radius: i32,
    pub damage: i32,
}

#[derive(Serialize)]
pub struct BuildingDetails {
    pub id: i32,
    pub current_hp: i32,
    pub artifacts_obtained: i32,
}

#[derive(Serialize)]
pub struct ValidatorResponse {
    pub frame_no: i32,
    pub attacker_pos: Coords,
    pub mines_triggered: Vec<MineDetails>,
    pub buildings_damaged: Vec<BuildingDetails>,
    pub artifacts_gained: i32,
    pub state: Option<State>,
    pub is_sync: bool,
}

// pub fn is_road(pos: &Coords) -> bool {
//     // create user_map_space with id and block_type_id stored with the map_id for base (also redis?)
//     let block_type_id = user_map_space[pos.x][pos.y].block_type_id;
//     // have a global block_types (same as BlockType table) (redis)
//     block_types[block_type_id][BUILDING_TYPE_INDEX] == ROAD_ID
// }

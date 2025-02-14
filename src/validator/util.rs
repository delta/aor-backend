use std::collections::HashSet;
use std::hash::Hash;
use std::time::SystemTime;

use crate::api::attack::socket::DefenderResponse;
use crate::api::attack::socket::{ResultType, SocketResponse};
use crate::validator::state::State;
use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, Hash, PartialEq, Serialize, Clone)]
pub struct SourceDestXY {
    pub source_x: i32,
    pub source_y: i32,
    pub dest_x: i32,
    pub dest_y: i32,
}

#[derive(Serialize, Clone, Copy, Deserialize, Debug)]
pub struct Bomb {
    pub id: i32,
    pub blast_radius: i32,
    pub damage: i32,
    pub pos: Coords,
    pub is_dropped: bool,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Attacker {
    pub id: i32,
    pub attacker_pos: Coords,
    pub attacker_health: i32,
    pub attacker_speed: i32,
    // pub path_in_current_frame: Vec<Coords>,
    pub bombs: Vec<Bomb>,
    pub trigger_defender: bool,
    pub bomb_count: i32,
}

#[derive(Serialize, Clone, Deserialize)]
pub struct IsTriggered {
    pub is_triggered: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DefenderDetails {
    pub map_space_id: i32,
    pub name: String,
    pub radius: i32,
    pub speed: i32,
    pub damage: i32,
    pub defender_pos: Coords,
    pub is_alive: bool,
    pub damage_dealt: bool,
    pub target_id: Option<f32>,
    pub path_in_current_frame: Vec<Coords>,
    pub max_health: i32,
    pub block_id: i32,
    pub level: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HutDefenderDetails {
    pub hut_defender: DefenderDetails,
    pub hut_triggered: bool,
    pub hut_defenders_count: i32,
    pub hut_defender_latest_time: Option<u128>,
}

// Structs for sending response
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MineDetails {
    pub id: i32,
    pub position: Coords,
    pub radius: i32,
    pub damage: i32,
}

#[derive(Serialize, Clone, Deserialize)]
pub struct BombType {
    pub id: i32,
    pub radius: i32,
    pub damage: i32,
    pub total_count: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BuildingDetails {
    pub block_id: i32,
    pub map_space_id: i32,
    pub current_hp: i32,
    pub total_hp: i32,
    pub artifacts_obtained: i32,
    pub tile: Coords,
    pub width: i32,
    pub name: String,
    pub range: i32,
    pub frequency: i32,
    // pub block_id: i32,
    pub level: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InValidation {
    pub message: String,
    pub is_invalidated: bool,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, Hash, Copy, Deserialize)]
pub struct Coords {
    pub x: i32,
    pub y: i32,
}

#[derive(Serialize, Clone, Copy)]
pub struct SourceDest {
    pub source: Coords,
    pub dest: Coords,
}
#[derive(Serialize, Clone)]

pub struct DefenderReturnType {
    pub attacker_health: i32,
    pub defender_response: Vec<DefenderResponse>,
    pub state: State,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BulletSpawnResponse {
    pub sentry_id: i32,
    pub bullet_id: i32,
    pub damage: i32,
    pub shot_time: SystemTime,
    pub target_id: i32,
    pub has_collided: bool,
}

pub fn send_terminate_game_message(frame_number: i32, message: String) -> SocketResponse {
    SocketResponse {
        frame_number,
        result_type: ResultType::GameOver,
        is_alive: None,
        attacker_health: None,
        exploded_mines: None,
        defender_damaged: None,
        damaged_buildings: None,
        hut_triggered: false,
        hut_defenders: None,
        total_damage_percentage: None,
        is_sync: false,
        is_game_over: true,
        shoot_bullets: None,
        message: Some(message),
    }
}

pub fn select_side_hut_defender(
    shadow_tiles: &Vec<(i32, i32)>,
    roads: &HashSet<(i32, i32)>,
    hut_building: &BuildingDetails,
    hut_defender: &DefenderDetails,
) -> Option<DefenderDetails> {
    let tile2 = (
        shadow_tiles[shadow_tiles.len() - 2].0 + 1,
        shadow_tiles[shadow_tiles.len() - 2].1,
    );
    let tile4 = (
        shadow_tiles[(2 * hut_building.width - 1) as usize].0,
        shadow_tiles[(2 * hut_building.width - 1) as usize].1 + 1,
    );
    let tile3 = (
        shadow_tiles[hut_building.width as usize].0,
        shadow_tiles[hut_building.width as usize].1 - 1,
    );
    let tile1 = (shadow_tiles[1].0 - 1, shadow_tiles[1].1);

    let mut hut_defender_clone = hut_defender.clone();
    if roads.contains(&tile2) {
        hut_defender_clone.defender_pos.x = tile2.0;
        hut_defender_clone.defender_pos.y = tile2.1;
        hut_defender_clone.target_id = Some(0.0);
        Some(hut_defender_clone)
    } else if roads.contains(&tile4) {
        hut_defender_clone.defender_pos.x = tile4.0;
        hut_defender_clone.defender_pos.y = tile4.1;
        hut_defender_clone.target_id = Some(0.0);
        Some(hut_defender_clone)
    } else if roads.contains(&tile3) {
        hut_defender_clone.defender_pos.x = tile3.0;
        hut_defender_clone.defender_pos.y = tile3.1;
        hut_defender_clone.target_id = Some(0.0);
        Some(hut_defender_clone)
    } else if roads.contains(&tile1) {
        hut_defender_clone.defender_pos.x = tile1.0;
        hut_defender_clone.defender_pos.y = tile1.1;
        hut_defender_clone.target_id = Some(0.0);
        Some(hut_defender_clone)
    } else {
        None
    }
}

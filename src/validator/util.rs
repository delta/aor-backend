use crate::api::attack::socket::DefenderResponse;
use crate::api::attack::socket::{BulletHit, ResultType, SocketResponse};
use crate::validator::state::State;
use serde::{Deserialize, Serialize};

#[derive(Eq, Hash, PartialEq, Serialize, Clone)]
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
    pub path_in_current_frame: Vec<Coords>,
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
    pub id: i32,
    pub radius: i32,
    pub speed: i32,
    pub damage: i32,
    pub defender_pos: Coords,
    pub is_alive: bool,
    pub damage_dealt: bool,
    pub target_id: Option<f32>,
    pub path_in_current_frame: Vec<Coords>,
    pub range: i32,
    pub frequency: i32,
    pub last_attack: i32,
}

// Structs for sending response
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MineDetails {
    pub id: i32,
    pub position: Coords,
    pub radius: i32,
    pub damage: i32,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct BombType {
    pub id: i32,
    pub radius: i32,
    pub damage: i32,
    pub total_count: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BuildingDetails {
    pub id: i32,
    pub current_hp: i32,
    pub total_hp: i32,
    pub artifacts_obtained: i32,
    pub tile: Coords,
    pub width: i32,
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
    pub bullet_hits: Vec<BulletHit>,
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

pub fn send_terminate_game_message(frame_number: i32, message: String) -> SocketResponse {
    SocketResponse {
        frame_number,
        result_type: ResultType::GameOver,
        is_alive: None,
        attacker_health: None,
        exploded_mines: None,
        defender_damaged: None,
        damaged_buildings: None,
        total_damage_percentage: None,
        is_sync: false,
        is_game_over: true,
        message: Some(message),
        bullet_hits: None,
        revealed_mines: None,
    }
}

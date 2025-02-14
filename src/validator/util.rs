use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::time::SystemTime;

use crate::api::attack::socket::{
    BuildingDamageResponse, DefenderDamageResponse, DefenderResponse,
};
use crate::api::attack::socket::{ResultType, SocketResponse};
use crate::constants::COMPANION_PRIORITY;
use crate::validator::state::State;
use futures::future::LocalBoxFuture;
use oauth2::url::OpaqueOrigin;
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

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct Companion {
    pub id: i32,
    pub companion_pos: Coords,
    pub companion_health: i32,
    pub companion_speed: i32,
    pub path_in_current_frame: Vec<Coords>,
    pub bombs: Vec<Bomb>,
    pub trigger_defender: bool,
    pub bomb_count: i32,
    pub range: i32,
    pub target_building: Option<BuildingDetails>,
    pub target_defender: Option<DefenderDetails>,
    pub target_tile: Option<Coords>,
    pub current_target: Option<CompanionTarget>,
    pub reached_dest: bool,
    pub last_attack_tick: i32,
    pub attack_interval: i32,
    pub damage: i32,
}

#[derive(Serialize, Clone, Deserialize)]
pub struct IsTriggered {
    pub is_triggered: bool,
}

#[derive(Debug, Serialize, Clone, PartialEq, Copy, Deserialize)]
pub enum DefenderTarget {
    Attacker,
    Companion,
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
    pub target_id: Option<DefenderTarget>,
    pub path_in_current_frame: Vec<Coords>,
    pub max_health: i32,
    pub block_id: i32,
    pub level: i32,
    pub current_health: i32,
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
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MineResponse {
    pub id: i32,
    pub position: Coords,
    pub radius: i32,
    pub damage: i32,
    pub target_id: i32,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InValidation {
    pub message: String,
    pub is_invalidated: bool,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, Hash, Copy, Deserialize)]
pub struct Coords {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, Hash, Copy, Deserialize)]
pub struct Path {
    pub x: i32,
    pub y: i32,
    pub l: i32,
}

#[derive(Serialize, Clone, Copy)]
pub struct SourceDest {
    pub source: Coords,
    pub dest: Coords,
}
#[derive(Serialize, Clone)]

pub struct DefenderReturnType {
    pub attacker_health: i32,
    pub companion_health: i32,
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

#[derive(Debug, Serialize, Clone, PartialEq, Copy, Deserialize)]
pub enum CompanionTarget {
    Building,
    Defender,
}
#[derive(Serialize, Clone, Debug)]
pub struct CompanionPriorityResponse {
    pub high_prior_building: (Option<BuildingDetails>, Option<Coords>, f64),
    pub second_prior_building: (Option<BuildingDetails>, Option<Coords>, f64),
    pub high_prior_defender: (Option<DefenderDetails>, f64),
    pub high_prior_tile: Option<Coords>,
    pub current_target: Option<CompanionTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionResult {
    pub current_target: Option<CompanionTarget>,
    pub map_space_id: i32,
    pub current_target_tile: Option<Coords>,
    pub is_alive: bool,
    pub health: i32,
    pub building_damaged: Option<BuildingDamageResponse>,
    pub defender_damaged: Option<DefenderDamageResponse>,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Sentry {
    pub id: i32,
    pub building_data: BuildingDetails,
    pub is_sentry_activated: bool,
    pub current_collided_bullet_id: i32,
    pub sentry_start_time: SystemTime,
    pub current_bullet_shot_id: i32,
    pub current_bullet_shot_time: SystemTime,
    pub bullets_shot: Vec<BulletSpawnResponse>,
    pub shoot_bullet: bool,
    pub hut_defenders_released: i32,
    pub target_id: i32,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, Hash, Copy, Deserialize)]
pub enum ChallengeType {
    Maze,
    FallGuys,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, Hash, Copy, Deserialize)]
pub struct MazeChallenge {
    pub coins: i32,
    pub start_time: SystemTime,
}
#[derive(Debug, Serialize, Clone, PartialEq, Eq, Hash, Copy, Deserialize)]
pub struct FallGuys {
    pub hut_range_increment: i32,
    pub hut_frequency_increment: i32,
    pub sentry_range_increment: i32,
    pub sentry_frequency_increment: i32,
    pub last_intensity_update_tick: i32,
    pub update_intensity_interval: i32,
    pub multipliers: FallGuysMultipliers,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, Hash, Copy, Deserialize)]
pub struct FallGuysMultipliers {
    pub treasury_level_1: i32,
    pub treasury_level_2: i32,
    pub treasury_level_3: i32,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, Hash, Copy, Deserialize)]
pub struct Challenge {
    pub challenge_type: Option<ChallengeType>,
    pub score: i32,
    pub challenge_completed: bool,
    pub start_tile: Coords,
    pub end_tile: Coords,
    pub maze: Option<MazeChallenge>,
    pub fall_guys: Option<FallGuys>,
}

pub fn send_terminate_game_message(frame_number: i32, message: String) -> SocketResponse {
    SocketResponse {
        frame_number,
        result_type: ResultType::GameOver,
        is_alive: None,
        attacker_health: None,
        exploded_mines: None,
        defender_damaged: None,
        damaged_base_items: None,
        hut_triggered: false,
        hut_defenders: None,
        total_damage_percentage: None,
        is_sync: false,
        is_game_over: true,
        shoot_bullets: None,
        message: Some(message),
        companion: None,
        challenge: None,
    }
}

pub fn select_side_hut_defender(
    shadow_tiles: &Vec<(i32, i32)>,
    roads: &HashSet<(i32, i32)>,
    hut_building: &BuildingDetails,
    hut_defender: &DefenderDetails,
    previous_hut_defender_id: &mut i32,
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
        hut_defender_clone.map_space_id = *previous_hut_defender_id + 1;
        *previous_hut_defender_id += 1;
        hut_defender_clone.target_id = None;
        Some(hut_defender_clone)
    } else if roads.contains(&tile4) {
        hut_defender_clone.defender_pos.x = tile4.0;
        hut_defender_clone.defender_pos.y = tile4.1;
        hut_defender_clone.map_space_id = *previous_hut_defender_id + 1;
        *previous_hut_defender_id += 1;
        hut_defender_clone.target_id = None;
        Some(hut_defender_clone)
    } else if roads.contains(&tile3) {
        hut_defender_clone.defender_pos.x = tile3.0;
        hut_defender_clone.defender_pos.y = tile3.1;
        hut_defender_clone.map_space_id = *previous_hut_defender_id + 1;
        *previous_hut_defender_id += 1;
        hut_defender_clone.target_id = None;
        Some(hut_defender_clone)
    } else if roads.contains(&tile1) {
        hut_defender_clone.defender_pos.x = tile1.0;
        hut_defender_clone.defender_pos.y = tile1.1;
        hut_defender_clone.map_space_id = *previous_hut_defender_id + 1;
        *previous_hut_defender_id += 1;
        hut_defender_clone.target_id = None;
        Some(hut_defender_clone)
    } else {
        None
    }
}

pub fn get_roads_around_building(
    building: &BuildingDetails,
    roads: &HashSet<(i32, i32)>,
) -> Vec<(i32, i32)> {
    let mut road_tiles: Vec<(i32, i32)> = Vec::new();
    let building_coords = (building.tile.x, building.tile.y);
    let left = (
        building_coords.0 + building.width,
        building_coords.1 + (building.width / 2),
    );
    if roads.contains(&left) {
        road_tiles.push(left);
    }

    let right = (
        building_coords.0 - 1,
        building_coords.1 + (building.width / 2),
    );
    if roads.contains(&right) {
        road_tiles.push(right);
    }

    let top = (
        building_coords.0 + (building.width / 2),
        building_coords.1 - 1,
    );
    if roads.contains(&top) {
        road_tiles.push(top);
    }

    let bottom = (
        building_coords.0 + (building.width / 2),
        building_coords.1 + building.width,
    );
    if roads.contains(&bottom) {
        road_tiles.push(bottom);
    }

    return road_tiles;
}

pub fn get_companion_priority(
    buildings: &Vec<BuildingDetails>,
    defenders: &Vec<DefenderDetails>,
    companion: &mut Companion,
    roads: &HashSet<(i32, i32)>,
    shortest_path: &HashMap<SourceDestXY, Path>,
) -> CompanionPriorityResponse {
    let mut high_prior_building: (Option<BuildingDetails>, Option<Coords>, f64) =
        (None, None, -1.0);
    let mut high_prior_defender: (Option<DefenderDetails>, f64) = (None, -1.0);
    let mut second_prior_building: (Option<BuildingDetails>, Option<Coords>, f64) =
        (None, None, -1.0);
    let mut high_prior_tile: Option<Coords> = None;

    //handle buildings
    for building in buildings {
        let mut visible = false;
        if building.current_hp == 0 {
            continue;
        }
        let companion_start_x = companion.companion_pos.x - companion.range;
        let companion_start_y = companion.companion_pos.y - companion.range;
        let companion_end_x = companion.companion_pos.x + companion.range;
        let companion_end_y = companion.companion_pos.y + companion.range;

        let start_x = building.tile.x;
        let start_y = building.tile.y;
        let end_x = building.tile.x + building.width;
        let end_y = building.tile.y + building.width;

        let building_coords = vec![
            (start_x, start_y),
            (start_x, end_y),
            (end_x, start_y),
            (end_x, end_y),
        ];
        for coords in building_coords {
            if !visible {
                if companion_start_x <= coords.0
                    && companion_end_x >= coords.0
                    && companion_start_y <= coords.1
                    && companion_end_y >= coords.1
                {
                    visible = true;
                }
            } else {
                break;
            }
        }
        if visible {
            let mut dist = 1000000;

            let building_road_tiles = get_roads_around_building(&building, roads);

            let mut local_target_tile = Coords { x: -1, y: -1 };
            for road_tile in building_road_tiles {
                let next_hop = shortest_path.get(&SourceDestXY {
                    source_x: companion.companion_pos.x,
                    source_y: companion.companion_pos.y,
                    dest_x: road_tile.0,
                    dest_y: road_tile.1,
                });

                if let Some(next_hop) = next_hop {
                    if next_hop.l < dist {
                        dist = next_hop.l;
                        local_target_tile.x = road_tile.0;
                        local_target_tile.y = road_tile.1;
                    }
                } else {
                    if 0 < dist {
                        dist = 0;
                        local_target_tile.x = road_tile.0;
                        local_target_tile.y = road_tile.1;
                    }
                }
            }

            let is_defending_building =
                building.name == "Defender_Hut" || building.name == "Sentry";

            let priority = if is_defending_building {
                COMPANION_PRIORITY.defender_buildings
            } else {
                COMPANION_PRIORITY.buildings
            } as f64;

            let priority = priority + 1.0 / ((dist + 1) as f64);
            if priority > high_prior_building.2 {
                high_prior_building.0 = Some(building.clone());
                high_prior_building.1 = Some(local_target_tile.clone());
                high_prior_building.2 = priority;
            }
            // log::info!(
            //     "Visible: name: {}, building min path: {:?}, distance: {}, priority: {}",
            //     building.name,
            //     local_target_tile,
            //     dist,
            //     priority,
            // );
        } else {
            let mut dist = 1000000;

            let building_road_tiles = get_roads_around_building(&building, roads);

            let mut local_target_tile = Coords { x: -1, y: -1 };
            for road_tile in building_road_tiles {
                let next_hop = shortest_path.get(&SourceDestXY {
                    source_x: companion.companion_pos.x,
                    source_y: companion.companion_pos.y,
                    dest_x: road_tile.0,
                    dest_y: road_tile.1,
                });

                if let Some(next_hop) = next_hop {
                    if next_hop.l < dist {
                        dist = next_hop.l;
                        local_target_tile.x = road_tile.0;
                        local_target_tile.y = road_tile.1;
                    }
                } else {
                    if 0 < dist {
                        dist = 0;
                        local_target_tile.x = road_tile.0;
                        local_target_tile.y = road_tile.1;
                    }
                }
            }

            let priority = 1.0 / ((dist + 1) as f64);

            if priority > second_prior_building.2 {
                second_prior_building.0 = Some(building.clone());
                second_prior_building.1 = Some(local_target_tile.clone());
                second_prior_building.2 = priority;
            }
            // log::info!(
            //     "name: {}, building min path: {:?}, distance: {}, priority: {}",
            //     building.name,
            //     local_target_tile,
            //     dist,
            //     priority,
            // );
        }
    }

    //handle defenders
    for defender in defenders {
        let companion_start_x = companion.companion_pos.x - companion.range;
        let companion_start_y = companion.companion_pos.y - companion.range;
        let companion_end_x = companion.companion_pos.x + companion.range;
        let companion_end_y = companion.companion_pos.y + companion.range;

        let coords = (defender.defender_pos.x, defender.defender_pos.y);

        let mut visible = false;
        if !visible {
            if companion_start_x <= coords.0
                && companion_end_x >= coords.0
                && companion_start_y <= coords.1
                && companion_end_y >= coords.1
            {
                visible = true;
            }
        } else {
            break;
        }
        if defender.current_health == 0 {
            continue;
        }
        if visible {
            let defender_pos = defender.defender_pos;
            let next_hop = shortest_path.get(&SourceDestXY {
                source_x: companion.companion_pos.x,
                source_y: companion.companion_pos.y,
                dest_x: defender_pos.x,
                dest_y: defender_pos.y,
            });

            let distance = if let Some(next_hop) = next_hop {
                next_hop.l
            } else {
                0
            };

            let priority = COMPANION_PRIORITY.defenders as f64 + 1.0 / ((distance + 1) as f64);

            if priority > high_prior_defender.1 {
                high_prior_defender.0 = Some(defender.clone());
                high_prior_defender.1 = priority;
            }
        }
    }
    let companion_target = if high_prior_building.0.is_some() && high_prior_defender.0.is_some() {
        if high_prior_building.2 > high_prior_defender.1 {
            high_prior_tile = high_prior_building.1.clone();
            Some(CompanionTarget::Building)
        } else {
            high_prior_tile = Some(companion.companion_pos);
            Some(CompanionTarget::Defender)
        }
    } else if high_prior_building.0.is_some() {
        high_prior_tile = high_prior_building.1.clone();
        Some(CompanionTarget::Building)
    } else if high_prior_defender.0.is_some() {
        high_prior_tile = Some(companion.companion_pos);
        Some(CompanionTarget::Defender)
    } else if second_prior_building.0.is_some() {
        high_prior_tile = second_prior_building.1.clone();
        Some(CompanionTarget::Building)
    } else {
        None
    };

    CompanionPriorityResponse {
        high_prior_building,
        second_prior_building,
        high_prior_defender,
        high_prior_tile,
        current_target: companion_target,
    }
}

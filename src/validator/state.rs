use std::{
    cmp::max,
    collections::{HashMap, HashSet},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use super::util::{
    select_side_hut_defender, BombType, BulletSpawnResponse, HutDefenderDetails, MineResponse,
    Sentry,
};
use crate::api::attack::{self, socket::BaseItemsDamageResponse};
use crate::constants::{
    BOMB_DAMAGE_MULTIPLIER, BULLET_COLLISION_TIME, DAMAGE_PER_BULLET_LEVEL_1,
    DAMAGE_PER_BULLET_LEVEL_2, DAMAGE_PER_BULLET_LEVEL_3, LEVEL, LIVES,
    PERCENTANGE_ARTIFACTS_OBTAINABLE,
};
use crate::{
    api::attack::socket::{
        BuildingDamageResponse, BulletHit, DefenderDamageResponse, DefenderResponse,
    },
    validator::util::{
        get_companion_priority, Attacker, BuildingDetails, CompanionTarget, Coords,
        DefenderDetails, DefenderReturnType, InValidation, MineDetails, SourceDestXY,
    },
};

use serde::{Deserialize, Serialize};

use super::{
    challenges::{attacker_movement_challenge_handle, bomb_blast_fallguys_handle},
    util::{Challenge, Companion, CompanionResult, DefenderTarget, Path},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct State {
    pub frame_no: i32,
    pub attacker_user_id: i32,
    pub defender_user_id: i32,
    pub attacker: Option<Attacker>,
    pub companion: Option<Companion>,
    pub attacker_death_count: i32,
    pub bombs: BombType,
    pub companion_bomb: BombType,
    pub damage_percentage: f32,
    pub artifacts: i32,
    pub defenders: Vec<DefenderDetails>,
    pub hut: HashMap<i32, HutDefenderDetails>,
    pub mines: Vec<MineDetails>,
    pub buildings: Vec<BuildingDetails>,
    pub total_hp_buildings: i32,
    pub in_validation: InValidation,
    pub sentries: Vec<Sentry>,
    pub hut_defenders_released: i32,
    pub challenge: Option<Challenge>,
    pub revealed_mines: Vec<MineDetails>,
    pub last_processed_frame: i32,
    pub revealed_checkpoints: Vec<f32>,
    pub uav_revealed: bool,
    pub uav_checkpoints: Vec<f32>,
}

impl State {
    pub fn new(
        attacker_user_id: i32,
        defender_user_id: i32,
        defenders: Vec<DefenderDetails>,
        hut_defenders: HashMap<i32, DefenderDetails>,
        mines: Vec<MineDetails>,
        buildings: Vec<BuildingDetails>,
        challenge: Option<Challenge>,
    ) -> State {
        let mut hut = HashMap::new();
        for building in buildings.clone() {
            if building.name == "Defender_Hut" {
                //get defender_level for the hut
                //log::info!("hut map space id: {}", building.map_space_id);
                let defender_level = hut_defenders.get(&building.map_space_id).unwrap().level;

                let defenders_count = LEVEL[(defender_level - 1) as usize].hut.defenders_limit;
                let hut_defender_details = HutDefenderDetails {
                    hut_defender: hut_defenders.get(&building.map_space_id).unwrap().clone(),
                    hut_triggered: false,
                    hut_defenders_count: defenders_count,
                    hut_defender_latest_time: None,
                };
                // log::info!(
                //     "hutttt: {:?} {:?}",
                //     hut_defender_details,
                //     building.map_space_id
                // );
                hut.insert(building.map_space_id, hut_defender_details);
            }
        }

        State {
            frame_no: 0,
            attacker_user_id,
            defender_user_id,
            attacker: None,
            companion: None,
            attacker_death_count: 0,
            bombs: BombType {
                id: -1,
                radius: 0,
                damage: 0,
                total_count: 0,
            },
            companion_bomb: BombType {
                id: -1,
                radius: 0,
                damage: 0,
                total_count: 0,
            },
            damage_percentage: 0.0,
            artifacts: 0,
            defenders,
            hut,
            mines,
            buildings,
            total_hp_buildings: 0,
            in_validation: InValidation {
                message: "".to_string(),
                is_invalidated: false,
            },
            challenge,
            hut_defenders_released: 0,
            sentries: Vec::new(),
            uav_revealed: false,
            uav_checkpoints: vec![20.0, 50.0, 70.0],
            revealed_checkpoints: Vec::new(),
            revealed_mines: Vec::new(),
            last_processed_frame: -1,
        }
    }
    pub fn get_sentries(&mut self) {
        let mut sentries = Vec::new();
        for building in self.buildings.iter() {
            if building.name == "Sentry" {
                sentries.push(Sentry {
                    id: building.map_space_id,
                    is_sentry_activated: false,
                    current_collided_bullet_id: 0,
                    sentry_start_time: SystemTime::now(),
                    current_bullet_shot_id: 0,
                    current_bullet_shot_time: SystemTime::now(),
                    shoot_bullet: false,
                    building_data: building.clone(),
                    bullets_shot: Vec::new(),
                    hut_defenders_released: 0,
                    target_id: -1,
                });
            }
        }
        //log::info!("sentries: {:?}", sentries);
        self.sentries = sentries;
    }
    pub fn check_uav_reveal(&mut self) -> Option<Vec<MineDetails>> {
        let mut revealed: Vec<MineDetails> = Vec::new();
        for checkpoint in &self.uav_checkpoints {
            if self.damage_percentage >= *checkpoint
                && !self.revealed_checkpoints.contains(checkpoint)
            {
                self.revealed_checkpoints.push(*checkpoint);
                let mines: Vec<MineDetails> = self
                    .mines
                    .iter()
                    .map(|mine| MineDetails {
                        name: mine.name.clone(),
                        id: mine.id,
                        position: mine.position.clone(),
                        radius: mine.radius,
                        damage: mine.damage,
                    })
                    .collect();
                revealed.extend(mines);
            }
        }
        if !revealed.is_empty() {
            self.revealed_mines = revealed.clone();
            return Some(revealed);
        }

        None
    }

    pub fn self_destruct(&mut self) {
        self.attacker_death_count += 1;
        self.attacker.as_mut().unwrap().attacker_health = 0;
        for defender in self.defenders.iter_mut() {
            defender.target_id = None;
        }
    }

    pub fn set_total_hp_buildings(&mut self) {
        let mut total_hp = 0;
        for building in self.buildings.iter() {
            total_hp += building.total_hp;
        }
        self.total_hp_buildings = total_hp;
    }

    pub fn set_bombs(&mut self, bomb_type: BombType, bombs: i32) {
        self.bombs = BombType {
            id: bomb_type.id,
            radius: bomb_type.radius,
            damage: bomb_type.damage,
            total_count: bombs,
        };
    }

    pub fn set_companion_bombs(&mut self, bomb_type: BombType, bombs: i32) {
        self.companion_bomb = BombType {
            id: bomb_type.id,
            radius: bomb_type.radius,
            damage: bomb_type.damage,
            total_count: bombs,
        }
    }
    pub fn place_attacker(&mut self, attacker: Attacker) {
        self.attacker = Some(attacker);
        //self.activate_sentry(attacker_position, 0);
        // println!("defnders: {:?}",self.defenders);
    }

    pub fn place_companion(&mut self, companion: Companion) {
        self.companion = Some(companion);
        //self.activate_sentry(self.companion.as_ref().unwrap().companion_pos.clone(), 1);
    }

    pub fn mine_blast_update(
        &mut self,
        _id: i32,
        damage_to_attacker: i32,
        damage_to_companion: i32,
    ) {
        let attacker = self.attacker.as_mut().unwrap();
        let companion = self.companion.as_mut();

        if attacker.attacker_health > 0 {
            attacker.attacker_health =
                std::cmp::max(0, attacker.attacker_health - damage_to_attacker);
            if attacker.attacker_health == 0 {
                self.attacker_death_count += 1;
                for defender in self.defenders.iter_mut() {
                    defender.target_id = None;
                }
                attacker.attacker_pos = Coords { x: -1, y: -1 };
            }
        }

        if let Some(companion) = companion {
            if companion.companion_health > 0 {
                companion.companion_health =
                    std::cmp::max(0, companion.companion_health - damage_to_companion);
            }
        }

        self.mines.retain(|mine| mine.id != _id);
    }

    pub fn update_frame_number(&mut self, frame_no: i32) {
        self.frame_no = frame_no;
    }

    pub fn attacker_movement(
        &mut self,
        frame_no: i32,
        roads: &HashSet<(i32, i32)>,
        attacker_current: Attacker,
    ) -> Option<Attacker> {
        if (frame_no - self.frame_no) != 1 {
            self.in_validation = InValidation {
                message: "Frame number mismatch".to_string(),
                is_invalidated: true,
            };
            // GAME_OVER
        }

        if self.attacker_death_count == LIVES {
            log::info!("Attacker lives forged");
            self.in_validation = InValidation {
                message: "Attacker Lives forged!".to_string(),
                is_invalidated: true,
            };
        }

        //challenges handler
        attacker_movement_challenge_handle(self, roads, &attacker_current);

        if !roads.contains(&(
            attacker_current.attacker_pos.x,
            attacker_current.attacker_pos.y,
        )) {
            // GAME_OVER

            println!("attacker out of road at {} frame", frame_no);
        }

        let mut attacker = attacker_current.clone();

        // if attacker.attacker_speed + 1 != attacker.path_in_current_frame.len() as i32 {
        //     println!(
        //         "attacker speed abuse at {} frame --- speed  :{}, length: {}",
        //         frame_no,
        //         attacker.attacker_speed,
        //         attacker.path_in_current_frame.len()
        //     );
        // }

        // let mut coord_temp: Coords = Coords {
        //     x: attacker_current.path_in_current_frame[0].x,
        //     y: attacker_current.path_in_current_frame[0].y,
        // };

        // for (i, coord) in attacker_current
        //     .path_in_current_frame
        //     .into_iter()
        //     .enumerate()
        // {
        if (attacker.attacker_pos.x - attacker_current.attacker_pos.x > 1)
            || (attacker.attacker_pos.y - attacker_current.attacker_pos.y > 1)
            || ((attacker.attacker_pos.x - attacker_current.attacker_pos.x).abs() == 1
                && attacker.attacker_pos.y != attacker_current.attacker_pos.y)
            || ((attacker.attacker_pos.y - attacker_current.attacker_pos.y).abs() == 1
                && attacker.attacker_pos.x != attacker_current.attacker_pos.x)
        {
            // GAME_OVER
            // println!("attacker skipped a tile at {} frame", frame_no);
            self.in_validation = InValidation {
                message: "attacker skipped a tile".to_string(),
                is_invalidated: true,
            };
        }

        // let new_pos = coord;

        let hut_buildings: Vec<BuildingDetails> = self
            .buildings
            .iter()
            .filter(|&r| r.name == "Defender_Hut")
            .cloned()
            .collect();

        for hut_building in hut_buildings {
            let distance = (hut_building.tile.x - attacker_current.attacker_pos.x).abs()
                + (hut_building.tile.y - attacker_current.attacker_pos.y).abs();

            if distance <= hut_building.range {
                if let Some(hut) = self.hut.get_mut(&hut_building.map_space_id) {
                    if !hut.hut_triggered {
                        // Hut triggered
                        //log::info!("Inside hut range!");
                        //trigger hut
                        hut.hut_triggered = true;
                    }
                }
            }
        }

        self.frame_no += 1;
        attacker.attacker_pos = attacker_current.attacker_pos;
        self.attacker.as_mut().unwrap().attacker_pos = attacker_current.attacker_pos.clone();

        let attacker_result = Attacker {
            id: attacker.id,
            attacker_pos: attacker.attacker_pos,
            attacker_health: attacker.attacker_health,
            attacker_speed: attacker.attacker_speed,
            // path_in_current_frame: attacker.path_in_current_frame.clone(),
            bombs: attacker.bombs.clone(),
            trigger_defender: attacker.trigger_defender,
            bomb_count: attacker.bomb_count,
        };

        // log::info!(
        //     "attacker health: {}, companion health: {}",
        //     self.attacker.as_ref().unwrap().attacker_health,
        //     self.companion.as_ref().unwrap().companion_health
        // );
        Some(attacker_result)
    }

    pub fn defender_trigger(&mut self) {
        let attacker = self.attacker.as_mut().unwrap();

        for defender in self.defenders.iter_mut() {
            if defender.target_id.is_none() && defender.is_alive {
                if defender.name == "Hut_Defender" {
                    if attacker.attacker_health > 0 {
                        defender.target_id = Some(DefenderTarget::Attacker);
                        attacker.trigger_defender = true;
                    }
                } else {
                    let companion = self.companion.as_mut();
                    if let Some(companion) = companion {
                        let attacker_manhattan_dist =
                            (defender.defender_pos.x - attacker.attacker_pos.x).abs()
                                + (defender.defender_pos.y - attacker.attacker_pos.y).abs();

                        let companion_manhattan_dist =
                            (defender.defender_pos.x - companion.companion_pos.x).abs()
                                + (defender.defender_pos.y - companion.companion_pos.y).abs();

                        if companion_manhattan_dist <= defender.radius
                            && attacker_manhattan_dist <= defender.radius
                        {
                            if attacker_manhattan_dist <= companion_manhattan_dist {
                                if attacker.attacker_health > 0 {
                                    defender.target_id = Some(DefenderTarget::Attacker);
                                    attacker.trigger_defender = true;
                                } else if companion.companion_health > 0 {
                                    defender.target_id = Some(DefenderTarget::Companion);
                                    companion.trigger_defender = true;
                                } else {
                                    defender.target_id = None;
                                }
                            } else {
                                if companion.companion_health > 0 {
                                    defender.target_id = Some(DefenderTarget::Companion);
                                    companion.trigger_defender = true;
                                } else if attacker.attacker_health > 0 {
                                    defender.target_id = Some(DefenderTarget::Attacker);
                                    attacker.trigger_defender = true;
                                } else {
                                    defender.target_id = None;
                                }
                            }
                        } else if companion_manhattan_dist <= defender.radius {
                            if companion.companion_health > 0 {
                                defender.target_id = Some(DefenderTarget::Companion);
                                companion.trigger_defender = true;
                            } else {
                                defender.target_id = None;
                            }
                        } else if attacker_manhattan_dist <= defender.radius {
                            if attacker.attacker_health > 0 {
                                defender.target_id = Some(DefenderTarget::Attacker);
                                attacker.trigger_defender = true;
                            } else {
                                defender.target_id = None;
                            }
                        } else {
                            defender.target_id = None;
                        }
                    } else {
                        let attacker_manhattan_dist =
                            (defender.defender_pos.x - attacker.attacker_pos.x).abs()
                                + (defender.defender_pos.y - attacker.attacker_pos.y).abs();

                        if attacker_manhattan_dist <= defender.radius {
                            if attacker.attacker_health > 0 {
                                defender.target_id = Some(DefenderTarget::Attacker);
                                attacker.trigger_defender = true;
                            } else {
                                defender.target_id = None;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn spawn_hut_defender(
        &mut self,
        roads: &HashSet<(i32, i32)>,
        // attacker_current: Attacker,
    ) -> Option<Vec<DefenderDetails>> {
        // let attacker = attacker_current.clone();
        let hut_buildings: Vec<&BuildingDetails> = self
            .buildings
            .iter()
            .filter(|&r| r.name == "Defender_Hut")
            .collect();

        let mut response = Vec::new();
        // for (i, _coord) in attacker_current
        //     .path_in_current_frame
        //     .into_iter()
        //     .enumerate()
        // {
        for &hut_building in &hut_buildings {
            //get shadow tile for each hut.
            let mut shadow_tiles: Vec<(i32, i32)> = Vec::new();
            for i in 0..hut_building.width {
                for j in 0..hut_building.width {
                    shadow_tiles.push((hut_building.tile.x + i, hut_building.tile.y + j));
                }
            }
            //see if hut is triggered
            let hut_triggered = self
                .hut
                .get(&hut_building.map_space_id)
                .unwrap()
                .hut_triggered;

            //if hut is triggered and hut defenders are > 0, get the hut defender.
            let time_elapsed = if let Some(time_stamp) = self
                .hut
                .get(&hut_building.map_space_id)
                .unwrap()
                .hut_defender_latest_time
            {
                let start = SystemTime::now();
                let now = start
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards");
                let time_interval = hut_building.frequency as u128;
                //check if time elapsed is greater than time stamp.
                now.as_millis() >= time_stamp + time_interval
            } else {
                true
            };
            if hut_triggered
                && self
                    .hut
                    .get(&hut_building.map_space_id)
                    .unwrap()
                    .hut_defenders_count
                    > 0
                && time_elapsed
                && hut_building.current_hp > 0
            {
                if let Some(hut_defender) = select_side_hut_defender(
                    &shadow_tiles,
                    roads,
                    &hut_building,
                    &self
                        .hut
                        .get(&hut_building.map_space_id)
                        .unwrap()
                        .hut_defender,
                    &mut self.hut_defenders_released,
                ) {
                    //push it to state.
                    println!("Hut defender spawned");
                    self.defenders.push(hut_defender.clone());
                    //push it to frontend response.
                    response.push(hut_defender);

                    //update time
                    let start = SystemTime::now();
                    let now = start
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards");
                    self.hut
                        .get_mut(&hut_building.map_space_id)
                        .unwrap()
                        .hut_defender_latest_time = Some(now.as_millis());

                    //update hut_defenders count.
                    let curr_count = self
                        .hut
                        .get(&hut_building.map_space_id)
                        .unwrap()
                        .hut_defenders_count;
                    self.hut
                        .get_mut(&hut_building.map_space_id)
                        .unwrap()
                        .hut_defenders_count = curr_count - 1;
                }
            }
            // }
        }
        return Some(response);
    }

    pub fn move_companion(
        &mut self,
        roads: &HashSet<(i32, i32)>,
        shortest_path: &HashMap<SourceDestXY, Path>,
    ) -> Option<CompanionResult> {
        //let companion = self.companion.as_mut().unwrap();
        if let Some(companion) = self.companion.as_mut() {
            let mut building_damaged: Option<BuildingDamageResponse> = None;
            let mut defender_damaged: Option<DefenderDamageResponse> = None;
            let is_companion_alive = companion.companion_health > 0;

            if is_companion_alive {
                //defender logic
                if companion.reached_dest {
                    //in destination.
                    let target_building = companion.target_building.clone();
                    let target_defender = companion.target_defender.clone();
                    let current_target = companion.current_target;

                    //check for defender or building to update reached.
                    if let Some(current_target) = current_target {
                        match current_target {
                            CompanionTarget::Building => {
                                let target_building = target_building.unwrap();
                                for building in self.buildings.iter_mut() {
                                    if building.map_space_id == target_building.map_space_id {
                                        if self.frame_no
                                            >= companion.last_attack_tick
                                                + companion.attack_interval
                                        {
                                            companion.last_attack_tick = self.frame_no;
                                            if building.current_hp <= 0 {
                                                //log::info!("Double down!!");
                                                companion.reached_dest = false;
                                                companion.target_building = None;
                                                companion.target_defender = None;
                                                companion.target_tile = None;
                                                companion.current_target = None;
                                                break;
                                            }
                                            let mut artifacts_taken_by_destroying_building = 0;
                                            building.current_hp =
                                                max(building.current_hp - companion.damage, 0);
                                            self.damage_percentage += companion.damage as f32
                                                / self.total_hp_buildings as f32
                                                * 100.0_f32;

                                            if building.current_hp <= 0 {
                                                companion.reached_dest = false;
                                                companion.target_building = None;
                                                companion.target_defender = None;
                                                companion.target_tile = None;
                                                companion.current_target = None;
                                                artifacts_taken_by_destroying_building =
                                                    (building.artifacts_obtained as f32
                                                        * PERCENTANGE_ARTIFACTS_OBTAINABLE)
                                                        .floor()
                                                        as i32;
                                                self.artifacts +=
                                                    artifacts_taken_by_destroying_building;
                                            }

                                            building_damaged = Some(BuildingDamageResponse {
                                                id: building.map_space_id,
                                                position: building.tile.clone(),
                                                hp: building.current_hp,
                                                artifacts_if_damaged:
                                                    artifacts_taken_by_destroying_building,
                                            });
                                        }
                                    }
                                }
                            }
                            CompanionTarget::Defender => {
                                let target_defender = target_defender.unwrap();
                                for defender in self.defenders.iter_mut() {
                                    if defender.map_space_id == target_defender.map_space_id {
                                        if self.frame_no
                                            >= companion.last_attack_tick
                                                + companion.attack_interval
                                        {
                                            //log::info!("Defender attacked");
                                            defender.current_health =
                                                max(defender.current_health - companion.damage, 0);
                                            companion.last_attack_tick = self.frame_no;
                                            defender_damaged = Some(DefenderDamageResponse {
                                                map_space_id: defender.map_space_id,
                                                position: defender.defender_pos,
                                                health: defender.current_health,
                                            });

                                            if defender.current_health <= 0 || !defender.is_alive {
                                                defender.is_alive = false;
                                                companion.reached_dest = false;
                                                companion.target_building = None;
                                                companion.target_defender = None;
                                                companion.target_tile = None;
                                                companion.current_target = None;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        companion.reached_dest = false;
                        companion.target_building = None;
                        companion.target_defender = None;
                        companion.current_target = None;
                        companion.target_tile = None;
                    }
                } else {
                    //get priorities if we don't have a target.
                    if companion.current_target.is_none() {
                        let priority = get_companion_priority(
                            &self.buildings,
                            &self.defenders,
                            companion,
                            roads,
                            shortest_path,
                        );
                        //log::info!("Priority: {:?}", priority);
                        let target_building = if priority.high_prior_building.0.is_some() {
                            priority.high_prior_building.0
                        } else {
                            priority.second_prior_building.0
                        };

                        let target_defender = priority.high_prior_defender.0;

                        let target_tile = priority.high_prior_tile;

                        let current_target = priority.current_target;

                        companion.target_building = target_building;
                        companion.target_defender = target_defender;
                        companion.target_tile = target_tile;
                        companion.current_target = current_target;
                    }

                    //move to destination.
                    let target_tile = companion.target_tile.clone().unwrap();
                    let next_hop = shortest_path.get(&SourceDestXY {
                        source_x: companion.companion_pos.x,
                        source_y: companion.companion_pos.y,
                        dest_x: target_tile.x,
                        dest_y: target_tile.y,
                    });

                    if let Some(next_hop) = next_hop {
                        companion.companion_pos = Coords {
                            x: next_hop.x,
                            y: next_hop.y,
                        };
                        //log::info!("moved");
                        if companion.companion_pos.x == target_tile.x
                            && companion.companion_pos.y == target_tile.y
                        {
                            companion.reached_dest = true;
                        }
                    } else {
                        //log::info!("In the same place");
                        companion.reached_dest = true;
                    }
                }
            } else {
                companion.reached_dest = false;
                companion.target_building = None;
                companion.target_defender = None;
                companion.target_tile = None;
                companion.current_target = None;
            }

            let target_mapspace_id = if let Some(current_target) = &companion.current_target {
                match current_target {
                    CompanionTarget::Building => {
                        if let Some(target_building) = &companion.target_building {
                            target_building.map_space_id
                        } else {
                            -1
                        }
                    }
                    CompanionTarget::Defender => {
                        if let Some(target_defender) = &companion.target_defender {
                            target_defender.map_space_id
                        } else {
                            -1
                        }
                    }
                }
            } else {
                -1
            };

            Some(CompanionResult {
                current_target: companion.current_target,
                map_space_id: target_mapspace_id,
                current_target_tile: companion.target_tile,
                is_alive: companion.companion_health > 0,
                health: companion.companion_health,
                building_damaged,
                defender_damaged,
            })
        } else {
            None
        }
    }

    pub fn place_bombs(
        &mut self,
        current_pos: Coords,
        bomb_position: Coords,
    ) -> BaseItemsDamageResponse {
        // if attacker_current.bombs.len() - attacker.bombs.len() > 1 {

        // }

        if self.bombs.total_count <= 0 {
            self.in_validation = InValidation {
                message: "Bomb Count forged".to_string(),
                is_invalidated: true,
            };
        }

        if let Some(attacker) = &mut self.attacker {
            attacker.bomb_count -= 1;
        }

        if current_pos.x != bomb_position.x || current_pos.y != bomb_position.y {
            //GAME_OVER
            println!("Bomb placed out of path");
            self.in_validation = InValidation {
                message: "Bomb placed out of path".to_string(),
                is_invalidated: true,
            };
        }

        self.bomb_blast(bomb_position)
    }

    pub fn mine_blast(&mut self, start_pos: Option<Coords>) -> Vec<MineResponse> {
        let mut damage_to_attacker = 0;
        let mut damage_to_companion = 0;
        let attack_current_pos = start_pos.unwrap();
        let companion_current_pos = if let Some(companion) = self.companion.as_ref() {
            companion.companion_pos.clone()
        } else {
            Coords { x: -1, y: -1 }
        };

        let mut triggered_mines: Vec<MineResponse> = Vec::new();

        for mine in self.mines.clone().iter_mut() {
            if (attack_current_pos.x == mine.position.x && attack_current_pos.y == mine.position.y)
                && (companion_current_pos.x == mine.position.x
                    && companion_current_pos.y == mine.position.y)
            {
                damage_to_attacker = mine.damage;
                damage_to_companion = mine.damage;
                triggered_mines.push(MineResponse {
                    id: mine.id,
                    position: mine.position,
                    radius: mine.radius,
                    damage: mine.damage,
                    target_id: 2,
                });
                self.mine_blast_update(mine.id, damage_to_attacker, damage_to_companion);
            } else if attack_current_pos.x == mine.position.x
                && attack_current_pos.y == mine.position.y
            {
                damage_to_attacker = mine.damage;
                triggered_mines.push(MineResponse {
                    id: mine.id,
                    position: mine.position,
                    radius: mine.radius,
                    damage: mine.damage,
                    target_id: 0,
                });
                self.mine_blast_update(mine.id, damage_to_attacker, damage_to_companion);
            } else if companion_current_pos.x == mine.position.x
                && companion_current_pos.y == mine.position.y
            {
                damage_to_companion = mine.damage;
                triggered_mines.push(MineResponse {
                    id: mine.id,
                    position: mine.position,
                    radius: mine.radius,
                    damage: mine.damage,
                    target_id: 1,
                });
                self.mine_blast_update(mine.id, damage_to_attacker, damage_to_companion);
            }
        }

        triggered_mines
    }

    pub fn bomb_blast(&mut self, bomb_position: Coords) -> BaseItemsDamageResponse {
        let bomb = &mut self.bombs;
        let mut buildings_damaged: Vec<BuildingDamageResponse> = Vec::new();
        let challenge = &mut self.challenge;
        let bomb_matrix: HashSet<Coords> = (bomb_position.y - bomb.radius
            ..bomb_position.y + bomb.radius + 1)
            .flat_map(|y| {
                (bomb_position.x - bomb.radius..bomb_position.x + bomb.radius + 1)
                    .map(move |x| Coords { x, y })
            })
            .collect();
        // log::info!(
        //     "Bomb position : {}, {}, range x: {} to {} range y: {} to {}",
        //     bomb_position.x,
        //     bomb_position.y,
        //     bomb_position.x - bomb.radius,
        //     bomb_position.x + bomb.radius,
        //     bomb_position.y - bomb.radius,
        //     bomb_position.y + bomb.radius
        // );
        for building in self.buildings.iter_mut() {
            if building.current_hp > 0 {
                let mut artifacts_taken_by_destroying_building: i32 = 0;

                let building_matrix: HashSet<Coords> = (building.tile.y
                    ..building.tile.y + building.width)
                    .flat_map(|y| {
                        (building.tile.x..building.tile.x + building.width)
                            .map(move |x| Coords { x, y })
                    })
                    .collect();

                let coinciding_coords_damage = building_matrix.intersection(&bomb_matrix).count();

                let damage_buildings: f32 =
                    coinciding_coords_damage as f32 / building_matrix.len() as f32;

                if damage_buildings != 0.0 {
                    bomb_blast_fallguys_handle(challenge, damage_buildings as i32, building);
                    let old_hp = building.current_hp;
                    let mut current_damage = (damage_buildings
                        * (bomb.damage as f32 * BOMB_DAMAGE_MULTIPLIER))
                        .round() as i32;

                    building.current_hp -= current_damage;

                    if building.current_hp <= 0 {
                        building.current_hp = 0;
                        current_damage = old_hp;
                        artifacts_taken_by_destroying_building =
                            (building.artifacts_obtained as f32 * PERCENTANGE_ARTIFACTS_OBTAINABLE)
                                .floor() as i32;
                        self.artifacts += artifacts_taken_by_destroying_building;
                        self.damage_percentage +=
                            (current_damage as f32 / self.total_hp_buildings as f32) * 100.0_f32;
                    } else {
                        self.damage_percentage +=
                            (current_damage as f32 / self.total_hp_buildings as f32) * 100.0_f32;
                    }

                    buildings_damaged.push(BuildingDamageResponse {
                        id: building.map_space_id,
                        position: building.tile,
                        hp: building.current_hp,
                        artifacts_if_damaged: artifacts_taken_by_destroying_building,
                    });
                }
            } else {
                continue;
            }
        }
        let mut defenders_damaged: Vec<DefenderDamageResponse> = Vec::new();
        for defender in self.defenders.iter_mut() {
            if defender.current_health > 0 {
                let defender_position = Coords {
                    x: defender.defender_pos.x,
                    y: defender.defender_pos.y,
                };
                let defender_position_matrix = HashSet::from([defender_position]);

                let bomb_matrix: HashSet<Coords> = (bomb_position.y - bomb.radius
                    ..bomb_position.y + bomb.radius + 1)
                    .flat_map(|y| {
                        (bomb_position.x - bomb.radius..bomb_position.x + bomb.radius + 1)
                            .map(move |x| Coords { x, y })
                    })
                    .collect();

                let coinciding_coords_damage =
                    defender_position_matrix.intersection(&bomb_matrix).count();
                if coinciding_coords_damage > 0 {
                    let current_damage = bomb.damage;

                    defender.current_health -= current_damage;

                    if defender.current_health <= 0 {
                        defender.current_health = 0;
                        defender.is_alive = false;
                    }

                    defenders_damaged.push(DefenderDamageResponse {
                        position: defender_position,
                        health: defender.current_health,
                        map_space_id: defender.map_space_id,
                    });
                }
            } else {
                continue;
            }
        }

        for defender in defenders_damaged.iter() {
            if defender.health > 0 {
                // log::info!(
                //     "Defender:{} is damaged, health is : {}, position is {}, {}",
                //     defender.map_space_id,
                //     defender.health,
                //     defender.position.x,
                //     defender.position.y
                // );
            }
            if defender.health == 0 {
                // log::info!(
                //     "Defender:{} is dead, died at position {}, {}",
                //     defender.map_space_id,
                //     defender.position.x,
                //     defender.position.y
                // );
            }
        }

        self.bombs.total_count -= 1;
        let base_items_damaged = BaseItemsDamageResponse {
            buildings_damaged: buildings_damaged.clone(),
            defenders_damaged: defenders_damaged.clone(),
        };
        base_items_damaged
    }

    pub fn activate_sentry(&mut self) {
        for sentry in self.sentries.iter_mut() {
            let mut current_sentry_data: BuildingDetails = BuildingDetails {
                map_space_id: 0,
                current_hp: 0,
                total_hp: 0,
                artifacts_obtained: 0,
                tile: Coords { x: 0, y: 0 },
                width: 0,
                name: "".to_string(),
                range: 0,
                frequency: 0,
                block_id: 0,
                level: 0,
            };
            for building in self.buildings.iter() {
                if building.map_space_id == sentry.building_data.map_space_id {
                    current_sentry_data = building.clone();
                }
            }

            if current_sentry_data.current_hp > 0 {
                //for attacker
                let attacker_pos = self.attacker.as_ref().unwrap().attacker_pos;
                let (companion_pos, companion_health) =
                    if let Some(companion) = self.companion.as_ref() {
                        (companion.companion_pos.clone(), companion.companion_health)
                    } else {
                        (Coords { x: -1, y: -1 }, 0)
                    };
                let attacker_health = self.attacker.as_ref().unwrap().attacker_health;
                let prev_state = sentry.is_sentry_activated;
                let is_attacker_in_range = (sentry.building_data.tile.x - attacker_pos.x).abs()
                    + (sentry.building_data.tile.y - attacker_pos.y).abs()
                    <= sentry.building_data.range
                    && attacker_health > 0;
                let is_companion_in_range = (sentry.building_data.tile.x - companion_pos.x).abs()
                    + (sentry.building_data.tile.y - companion_pos.y).abs()
                    <= sentry.building_data.range
                    && companion_health > 0;

                sentry.is_sentry_activated = is_attacker_in_range || is_companion_in_range;
                let new_state = sentry.is_sentry_activated;

                if prev_state != new_state && new_state == true {
                    //log::info!("sentry activated");
                    sentry.sentry_start_time = SystemTime::now();
                    if is_attacker_in_range {
                        sentry.target_id = 0;
                    } else if is_companion_in_range {
                        sentry.target_id = 1;
                    }
                } else if prev_state != new_state && new_state == false {
                    //log::info!("sentry deactivated");
                    sentry.current_bullet_shot_time = SystemTime::now() - Duration::new(2, 0);
                    sentry.target_id = -1;
                }
            } else {
                sentry.is_sentry_activated = false;
                sentry.target_id = -1;
            }
        }
    }

    pub fn cause_bullet_damage(&mut self) {
        let attacker = self.attacker.as_mut().unwrap();
        if attacker.attacker_health <= 0 {
            for sentry in self.sentries.iter_mut() {
                for bullet in sentry.bullets_shot.iter_mut() {
                    bullet.has_collided = true;
                }
            }
        } else {
            for sentry in self.sentries.iter_mut() {
                for bullet in sentry.bullets_shot.iter_mut() {
                    if SystemTime::now()
                        .duration_since(bullet.shot_time)
                        .unwrap()
                        .as_millis() as i32
                        >= BULLET_COLLISION_TIME
                        && !bullet.has_collided
                    {
                        if bullet.target_id == 0 {
                            attacker.attacker_health =
                                max(0, attacker.attacker_health - bullet.damage);
                            // log::info!(
                            //     "ATTACKER HEALTH : {}, bullet_id {}",
                            //     attacker.attacker_health,
                            //     bullet.bullet_id
                            // );
                            bullet.has_collided = true;
                        } else if bullet.target_id == 1 {
                            let companion = self.companion.as_mut().unwrap();
                            companion.companion_health =
                                max(0, companion.companion_health - bullet.damage);
                            // log::info!(
                            //     "COMPANION HEALTH : {}, bullet_id {}",
                            //     companion.companion_health,
                            //     bullet.bullet_id
                            // );
                            bullet.has_collided = true;
                        }
                    }
                }
            }
        }
    }

    pub fn shoot_bullets(&mut self) -> Vec<BulletSpawnResponse> {
        let mut bullet_damage: i32;
        let mut shoot_bullet_res_array: Vec<BulletSpawnResponse> = Vec::new();
        for sentry in self.sentries.iter_mut() {
            let sentry_frequency = sentry.building_data.frequency;
            if sentry.is_sentry_activated
                && SystemTime::now()
                    .duration_since(sentry.current_bullet_shot_time)
                    .unwrap()
                    .as_millis()
                    >= 1000 / (sentry_frequency as u128)
            {
                sentry.current_bullet_shot_id += 1;
                sentry.current_bullet_shot_time = SystemTime::now();
                // log::info!(
                //     "sentry id: {}, bullet id: {}",
                //     sentry.id,
                //     sentry.current_bullet_shot_id
                // );
                if sentry.building_data.level == 3 {
                    bullet_damage = DAMAGE_PER_BULLET_LEVEL_3;
                } else if sentry.building_data.level == 2 {
                    bullet_damage = DAMAGE_PER_BULLET_LEVEL_2;
                } else {
                    bullet_damage = DAMAGE_PER_BULLET_LEVEL_1;
                }
                let bullet_response = BulletSpawnResponse {
                    bullet_id: sentry.current_bullet_shot_id,
                    shot_time: sentry.current_bullet_shot_time,
                    sentry_id: sentry.id,
                    damage: bullet_damage,
                    has_collided: false,
                    target_id: sentry.target_id,
                };
                // log::info!(
                //     "bullet {} from sentry {}",
                //     sentry.current_bullet_shot_id,
                //     sentry.id
                // );
                shoot_bullet_res_array.push(bullet_response.clone());
                sentry.bullets_shot.push(bullet_response);
            }
        }
        shoot_bullet_res_array
    }

    pub fn defender_movement_one_tick(
        &mut self,
        attacker_position: Coords,
        shortest_path: &HashMap<SourceDestXY, Path>,
    ) -> DefenderReturnType {
        let attacker = self.attacker.as_mut().unwrap();
        let mut defenders_damaged: Vec<DefenderResponse> = Vec::new();

        for defender in self.defenders.iter_mut() {
            if !defender.is_alive {
                continue;
            }

            if let Some(target_id) = defender.target_id {
                match target_id {
                    DefenderTarget::Attacker => {
                        let default_next_hop = Path {
                            x: defender.defender_pos.x,
                            y: defender.defender_pos.y,
                            l: 0,
                        };
                        let target_id = 1;

                        let next_hop = shortest_path
                            .get(&SourceDestXY {
                                source_x: defender.defender_pos.x,
                                source_y: defender.defender_pos.y,
                                dest_x: attacker_position.x,
                                dest_y: attacker_position.y,
                            })
                            .unwrap_or(&default_next_hop);

                        defender.defender_pos = Coords {
                            x: next_hop.x,
                            y: next_hop.y,
                        };

                        // if defender.name.starts_with("Hut") {
                        if attacker_position.x == defender.defender_pos.x
                            && attacker_position.y == defender.defender_pos.y
                        {
                            // log::info!(
                            //     "Defender pos {} {} and id {}",
                            //     defender.defender_pos.x,
                            //     defender.defender_pos.y,
                            //     defender.map_space_id
                            // );

                            defenders_damaged.push(DefenderResponse {
                                map_space_id: defender.map_space_id,
                                position: defender.defender_pos,
                                damage: defender.damage,
                                target_id,
                            });
                            defender.is_alive = false;
                            defender.current_health = 0;
                            attacker.attacker_health =
                                max(0, attacker.attacker_health - defender.damage);
                        }
                    }

                    DefenderTarget::Companion => {
                        let companion = self.companion.as_mut().unwrap();
                        let default_next_hop = Path {
                            x: defender.defender_pos.x,
                            y: defender.defender_pos.y,
                            l: 0,
                        };
                        let target_id = -1;

                        let next_hop = shortest_path
                            .get(&SourceDestXY {
                                source_x: defender.defender_pos.x,
                                source_y: defender.defender_pos.y,
                                dest_x: companion.companion_pos.x,
                                dest_y: companion.companion_pos.y,
                            })
                            .unwrap_or(&default_next_hop);

                        defender.defender_pos = Coords {
                            x: next_hop.x,
                            y: next_hop.y,
                        };
                        // if defender.name.starts_with("Hut") {
                        if companion.companion_pos.x == defender.defender_pos.x
                            && companion.companion_pos.y == defender.defender_pos.y
                        {
                            // log::info!(
                            //     "Defender pos {} {} and id {}",
                            //     defender.defender_pos.x,
                            //     defender.defender_pos.y,
                            //     defender.map_space_id
                            // );

                            defenders_damaged.push(DefenderResponse {
                                map_space_id: defender.map_space_id,
                                position: defender.defender_pos,
                                damage: defender.damage,
                                target_id,
                            });
                            defender.is_alive = false;
                            defender.current_health = 0;
                            companion.companion_health =
                                max(0, companion.companion_health - defender.damage);
                        }
                    }
                }
            }
        }
        let companion_health = if let Some(companion) = self.companion.as_ref() {
            companion.companion_health
        } else {
            0
        };

        DefenderReturnType {
            attacker_health: attacker.attacker_health,
            companion_health,
            defender_response: defenders_damaged,
            state: self.clone(),
            bullet_hits: Vec::new(),
        }
    }

    pub fn defender_ranged_attack(
        &mut self,
        frame_number: i32,
        coords: Coords,
    ) -> DefenderReturnType {
        // log::info!("Starting defender_ranged_attack for frame_number: {}", frame_number);

        if frame_number <= self.last_processed_frame {
            return DefenderReturnType {
                attacker_health: self.attacker.as_ref().unwrap().attacker_health,
                defender_response: Vec::new(),
                bullet_hits: Vec::new(),
                state: self.clone(),
                companion_health: 0,
            };
        }

        let state_clone = self.clone(); // Clone the state before borrowing self mutably
        let attacker = self.attacker.as_mut().unwrap();
        let mut defenders_damaged: Vec<DefenderResponse> = Vec::new();
        let mut bullet_hits: Vec<BulletHit> = Vec::new();
        let mut attackers: Vec<u32> = Vec::new();
        let attacker_pos = coords;

        // log::info!(
        //     "Attacker position is ({:?})",
        //     attacker_posF
        // );
        for defender in self.defenders.iter_mut() {
            log::info!(
                "Defender ID: {}, Target ID: {:?}, Position: ({}, {})",
                defender.map_space_id,
                defender.target_id,
                defender.defender_pos.x,
                defender.defender_pos.y
            );
            if !defender.is_alive {
                continue;
            }
            if defender.range == 0 {
                continue;
            }
            if defender.name == "Hut_Defender" {
                continue;
            }
            // log::info!("Defender range is {}", defender.range);

            let start = SystemTime::now();
            let now = start
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            let time_interval = defender.frequency as u128;

            if defender.last_attack == 0 {
                defender.last_attack = now.as_millis();
                // log::info!("Initializing defender {} last_attack to {}", defender.map_space_id, defender.last_attack);
            }

            let time_elapsed = now.as_millis() >= defender.last_attack as u128 + time_interval;

            if time_elapsed {
                defender.last_attack = now.as_millis();
                let target_pos = if defender.target_id == Some(DefenderTarget::Attacker) {
                    attacker_pos
                } else if defender.target_id == Some(DefenderTarget::Companion) {
                    let companion = self.companion.as_mut().unwrap();
                    let companion_pos = companion.companion_pos;
                    companion_pos
                } else {
                    Coords { x: -1, y: -1 }
                };

                // Check if target is within defender's range
                let distance = (((defender.defender_pos.x - target_pos.x).pow(2)
                    + (defender.defender_pos.y - target_pos.y).pow(2))
                    as f32)
                    .sqrt();
                log::info!(
                    "Distance between defender {} , with position {:?}, and target: {}",
                    defender.map_space_id,
                    defender.defender_pos,
                    distance
                );

                if distance <= defender.range as f32 {
                    // log::info!("Positions of companion and defender are: ({}, {}), ({}, {})", companion_pos.x, companion_pos.y, defender.defender_pos.x, defender.defender_pos.y);
                    // log::info!("Defender {} is within range of target", defender.map_space_id);
                    if defender.defender_pos.x == target_pos.x
                        || defender.defender_pos.y == target_pos.y
                    {
                        attackers.push(defender.map_space_id.try_into().unwrap());
                        // Check if there are any buildings between the defender and the target
                        let mut blocked = false;
                        if defender.defender_pos.x == target_pos.x {
                            let min_y = defender.defender_pos.y.min(target_pos.y);
                            let max_y = defender.defender_pos.y.max(target_pos.y);
                            for building in &self.buildings {
                                if building.tile.x == defender.defender_pos.x
                                    && building.tile.y > min_y
                                    && building.tile.y < max_y
                                {
                                    blocked = true;
                                    // log::info!("Defender {} attack is blocked by building at ({}, {})", defender.map_space_id, building.tile.x, building.tile.y);
                                    break;
                                }
                            }
                        } else if defender.defender_pos.y == target_pos.y {
                            let min_x = defender.defender_pos.x.min(target_pos.x);
                            let max_x = defender.defender_pos.x.max(target_pos.x);
                            for building in &self.buildings {
                                if building.tile.y == defender.defender_pos.y
                                    && building.tile.x > min_x
                                    && building.tile.x < max_x
                                {
                                    blocked = true;
                                    // log::info!("Defender {} attack is blocked by building at ({}, {})", defender.map_space_id, building.tile.x, building.tile.y);
                                    break;
                                }
                            }
                        }

                        if !blocked {
                            if defender.target_id == Some(DefenderTarget::Attacker) {
                                log::info!(
                                    "Defender {} successfully attacks attacker",
                                    defender.map_space_id
                                );
                                log::info!(
                                    "Positions of attacker and defender are: ({}, {}), ({}, {})",
                                    attacker_pos.x,
                                    attacker_pos.y,
                                    defender.defender_pos.x,
                                    defender.defender_pos.y
                                );
                                attacker.attacker_health =
                                    attacker.attacker_health.saturating_sub(defender.damage);
                                if attacker.attacker_health <= 0 {
                                    defender.target_id = None;
                                }
                                defenders_damaged.push(DefenderResponse {
                                    map_space_id: defender.map_space_id,
                                    position: defender.defender_pos,
                                    damage: defender.damage,
                                    target_id: 0,
                                });
                                bullet_hits.push(BulletHit {
                                    defender_id: defender.map_space_id,
                                    target_id: 0,
                                    damage: defender.damage,
                                    position: defender.defender_pos,
                                });
                            } else if defender.target_id == Some(DefenderTarget::Companion) {
                                let companion = self.companion.as_mut().unwrap();
                                let companion_pos = companion.companion_pos;
                                log::info!(
                                    "Defender {} successfully attacks companion",
                                    defender.map_space_id
                                );
                                log::info!(
                                    "Positions of companion and defender are: ({}, {}), ({}, {})",
                                    companion_pos.x,
                                    companion_pos.y,
                                    defender.defender_pos.x,
                                    defender.defender_pos.y
                                );
                                companion.companion_health =
                                    companion.companion_health.saturating_sub(defender.damage);
                                if companion.companion_health <= 0 {
                                    defender.target_id = None;
                                }
                                defenders_damaged.push(DefenderResponse {
                                    map_space_id: defender.map_space_id,
                                    position: defender.defender_pos,
                                    damage: defender.damage,
                                    target_id: 1,
                                });
                                bullet_hits.push(BulletHit {
                                    defender_id: defender.map_space_id,
                                    target_id: 1,
                                    damage: defender.damage,
                                    position: defender.defender_pos,
                                });
                            }
                        } else {
                            // log::info!("Defender {} attack is blocked", defender.map_space_id);
                        }
                    } else {
                        log::info!(
                            "Defender {} is not aligned with target. Defender position: ({}, {}), Target position: ({}, {})",
                            defender.map_space_id,
                            defender.defender_pos.x,
                            defender.defender_pos.y,
                            target_pos.x,
                            target_pos.y
                        );
                    }
                } else {
                    log::info!(
                        "Defender {} is out of range of target",
                        defender.map_space_id
                    );
                    log::info!("Defender range is {}", defender.range);
                    defender.target_id = None;
                }
            } else {
                // log::info!("Time not elapsed for defender {}. Skipping attack.", defender.map_space_id);
            }
        }

        // log::info!("Defender ranged attack completed for frame_number: {}", frame_number);
        log::info!("Defenders who attacked in this frame: {:?}", attackers);

        let companion_health = if let Some(companion) = self.companion.as_mut() {
            companion.companion_health
        } else {
            0
        };

        DefenderReturnType {
            attacker_health: attacker.attacker_health,
            defender_response: defenders_damaged,
            bullet_hits,
            state: state_clone,
            companion_health,
        }
    }
}

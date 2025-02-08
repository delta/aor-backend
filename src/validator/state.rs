use std::{
    cmp::max,
    collections::{HashMap, HashSet},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::validator::util::BulletSpawnResponse;
use crate::{
    api::attack::socket::{BuildingResponse, DefenderResponse},
    validator::util::{
        Attacker, BuildingDetails, Coords, DefenderDetails, DefenderReturnType, InValidation,
        MineDetails, SourceDestXY,
    },
};
use crate::constants::{
        BOMB_DAMAGE_MULTIPLIER, BULLET_COLLISION_TIME, DAMAGE_PER_BULLET_LEVEL_1,
        DAMAGE_PER_BULLET_LEVEL_2, DAMAGE_PER_BULLET_LEVEL_3, LEVEL, LIVES,
        PERCENTANGE_ARTIFACTS_OBTAINABLE,
    };
use serde::{Deserialize, Serialize};

use super::util::{select_side_hut_defender, BombType, HutDefenderDetails};

#[derive(Serialize, Deserialize, Clone)]
pub struct State {
    pub frame_no: i32,
    pub attacker_user_id: i32,
    pub defender_user_id: i32,
    pub attacker: Option<Attacker>,
    pub attacker_death_count: i32,
    pub bombs: BombType,
    pub damage_percentage: f32,
    pub artifacts: i32,
    pub defenders: Vec<DefenderDetails>,
    pub hut: HashMap<i32, HutDefenderDetails>,
    pub mines: Vec<MineDetails>,
    pub buildings: Vec<BuildingDetails>,
    pub total_hp_buildings: i32,
    pub in_validation: InValidation,
    pub sentries: Vec<Sentry>,
}

#[derive(Serialize, Deserialize, Clone)]
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
}

impl State {
    pub fn new(
        attacker_user_id: i32,
        defender_user_id: i32,
        defenders: Vec<DefenderDetails>,
        hut_defenders: HashMap<i32, DefenderDetails>,
        mines: Vec<MineDetails>,
        buildings: Vec<BuildingDetails>,
    ) -> State {
        let mut hut = HashMap::new();
        for building in buildings.clone() {
            if building.name == "Defender_Hut" {
                //get defender_level for the hut
                log::info!("hut map space id: {}", building.map_space_id);
                let defender_level = hut_defenders.get(&building.map_space_id).unwrap().level;

                let defenders_count = LEVEL[(defender_level - 1) as usize].hut.defenders_limit;
                let hut_defender_details = HutDefenderDetails {
                    hut_defender: hut_defenders.get(&building.map_space_id).unwrap().clone(),
                    hut_triggered: false,
                    hut_defenders_count: defenders_count,
                    hut_defender_latest_time: None,
                };
                log::info!(
                    "hutttt: {:?} {:?}",
                    hut_defender_details,
                    building.map_space_id
                );
                hut.insert(building.map_space_id, hut_defender_details);
            }
        }
        State {
            frame_no: 0,
            attacker_user_id,
            defender_user_id,
            attacker: None,
            attacker_death_count: 0,
            bombs: BombType {
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
            sentries: Vec::new(),
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
                });
            }
        }
        self.sentries = sentries;
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
    pub fn place_attacker(&mut self, attacker: Attacker) {
        let attacker_position = attacker.attacker_pos;
        self.attacker = Some(attacker);
        self.activate_sentry(attacker_position);
        // println!("defnders: {:?}",self.defenders);
    }

    pub fn mine_blast_update(&mut self, _id: i32, damage_to_attacker: i32) {
        let attacker = self.attacker.as_mut().unwrap();

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
            self.in_validation = InValidation {
                message: "Attacker Lives forged!".to_string(),
                is_invalidated: true,
            };
        }

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
                        log::info!("Inside hut range!");
                        //trigger hut
                        hut.hut_triggered = true;
                    }
                }
            }
        }

        for defender in self.defenders.iter_mut() {
            if defender.target_id.is_none()
                && defender.is_alive
                && (((defender.defender_pos.x - attacker_current.attacker_pos.x).abs()
                    + (defender.defender_pos.y - attacker_current.attacker_pos.y).abs())
                    <= defender.radius)
            {
                // println!(
                //     "defender triggered when attacker was at ---- x:{}, y:{} and defender id: {}",
                //     new_pos.x, new_pos.y, defender.id
                // );
                defender.target_id = Some(0.0);
                attacker.trigger_defender = true;
            }
            // }
            // coord_temp = coord;
        }
        self.activate_sentry(attacker_current.attacker_pos);

        self.frame_no += 1;
        attacker.attacker_pos = attacker_current.attacker_pos;

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
        Some(attacker_result)
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
                    hut_building,
                    &self
                        .hut
                        .get(&hut_building.map_space_id)
                        .unwrap()
                        .hut_defender,
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
        Some(response)
    }

    pub fn place_bombs(
        &mut self,
        current_pos: Coords,
        bomb_position: Coords,
    ) -> Vec<BuildingResponse> {
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

    // pub fn defender_movement(
    //     &mut self,
    //     attacker_delta: Vec<Coords>,
    //     shortest_path: &HashMap<SourceDestXY, Coords>,
    // ) -> DefenderReturnType {
    //     let attacker = self.attacker.as_mut().unwrap();
    //     let mut defenders_damaged: Vec<DefenderResponse> = Vec::new();

    //     // if attacker is dead, no need to move the defenders
    //     if attacker.attacker_health == 0 {
    //         return DefenderReturnType {
    //             attacker_health: attacker.attacker_health,
    //             defender_response: defenders_damaged,
    //             state: self.clone(),
    //         };
    //     }

    //     let mut collision_array: Vec<(usize, f32)> = Vec::new();

    //     for (index, defender) in self.defenders.iter_mut().enumerate() {
    //         if !defender.is_alive || defender.target_id.is_none() {
    //             continue;
    //         }

    //         let attacker_ratio = attacker.attacker_speed as f32 / defender.speed as f32;
    //         let mut attacker_float_coords = (
    //             attacker.attacker_pos.x as f32,
    //             attacker.attacker_pos.y as f32,
    //         );
    //         let mut attacker_delta_index = 1;

    //         defender.path_in_current_frame.clear();
    //         defender.path_in_current_frame.push(defender.defender_pos);

    //         // for every tile of defender's movement
    //         for i in 1..=defender.speed {
    //             let next_hop = shortest_path
    //                 .get(&SourceDestXY {
    //                     source_x: defender.defender_pos.x,
    //                     source_y: defender.defender_pos.y,
    //                     dest_x: attacker.attacker_pos.x,
    //                     dest_y: attacker.attacker_pos.y,
    //                 })
    //                 .unwrap_or(&defender.defender_pos);

    //             let mut attacker_tiles_covered_fract = (((i - 1) as f32) * attacker_ratio).fract();

    //             let mut attacker_mov_x = 0.0;
    //             let mut attacker_mov_y = 0.0;

    //             let mut attacker_tiles_left = attacker_ratio;
    //             while attacker_tiles_left > 1e-6 {
    //                 let attacker_tiles_fract_left = attacker_tiles_left
    //                     .min(1.0)
    //                     .min(1.0 - attacker_tiles_covered_fract);

    //                 attacker_mov_x += attacker_tiles_fract_left
    //                     * ((attacker_delta[attacker_delta_index].x
    //                         - attacker_delta[attacker_delta_index - 1].x)
    //                         as f32);
    //                 attacker_mov_y += attacker_tiles_fract_left
    //                     * ((attacker_delta[attacker_delta_index].y
    //                         - attacker_delta[attacker_delta_index - 1].y)
    //                         as f32);

    //                 attacker_tiles_left -= attacker_tiles_fract_left;
    //                 attacker_tiles_covered_fract =
    //                     (attacker_tiles_covered_fract + attacker_tiles_fract_left).fract();
    //                 if attacker_tiles_covered_fract == 0.0 {
    //                     attacker_delta_index += 1;
    //                 }
    //             }

    //             attacker_float_coords.0 += attacker_mov_x;
    //             attacker_float_coords.1 += attacker_mov_y;

    //             attacker.attacker_pos = Coords {
    //                 x: attacker_float_coords.0.round() as i32,
    //                 y: attacker_float_coords.1.round() as i32,
    //             };

    //             // if defender lags
    //             if defender.target_id.unwrap() >= ((i as f32) / (defender.speed as f32)) {
    //                 defender.path_in_current_frame.push(defender.defender_pos);
    //                 continue;
    //             }
    //             defender.defender_pos = *next_hop;
    //             defender.path_in_current_frame.push(defender.defender_pos);

    //             // if defender and attacker are on the same tile, add the defender to the collision_array
    //             if (defender.defender_pos == attacker.attacker_pos)
    //                 || (defender.path_in_current_frame[(i - 1) as usize] == attacker.attacker_pos)
    //             {
    //                 collision_array.push((index, (i as f32) / (defender.speed as f32)));
    //                 defender.damage_dealt = true;
    //                 break;
    //             }
    //         }
    //         defender.target_id = Some(0.0);
    //         if !defender.damage_dealt {
    //             collision_array.push((index, 2.0));
    //         }
    //         attacker.attacker_pos = *attacker_delta.first().unwrap();
    //     }

    //     attacker.attacker_pos = *attacker_delta.last().unwrap();
    //     // sort the collision_array by the time of collision
    //     collision_array.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    //     let mut attacker_death_time = 0.0; // frame fraction at which attacker dies
    //     for (index, time) in collision_array {
    //         self.defenders[index].target_id = None;
    //         if time > 1.0 {
    //             break;
    //         }
    //         if attacker.attacker_health == 0 {
    //             self.defenders[index].defender_pos = self.defenders[index].path_in_current_frame
    //                 [(attacker_death_time * (self.defenders[index].speed as f32)) as usize];
    //             continue;
    //         }
    //         defenders_damaged.push(DefenderResponse {
    //             id: self.defenders[index].id,
    //             position: self.defenders[index].defender_pos,
    //             damage: self.defenders[index].damage,
    //         });
    //         self.defenders[index].damage_dealt = true;
    //         attacker.trigger_defender = true;
    //         attacker.attacker_health =
    //             max(0, attacker.attacker_health - self.defenders[index].damage);
    //         self.defenders[index].is_alive = false;

    //         if attacker.attacker_health == 0 {
    //             attacker_death_time = time;
    //             self.attacker_death_count += 1;
    //         }
    //     }

    //     DefenderReturnType {
    //         attacker_health: attacker.attacker_health,
    //         defender_response: defenders_damaged,
    //         state: self.clone(),
    //     }
    // }

    pub fn mine_blast(&mut self, start_pos: Option<Coords>) -> Vec<MineDetails> {
        let mut damage_to_attacker;
        let attack_current_pos = start_pos.unwrap();

        let mut triggered_mines: Vec<MineDetails> = Vec::new();

        for mine in self.mines.clone().iter_mut() {
            if attack_current_pos.x == mine.position.x && attack_current_pos.y == mine.position.y {
                damage_to_attacker = mine.damage;
                triggered_mines.push(MineDetails {
                    id: mine.id,
                    position: mine.position,
                    radius: mine.radius,
                    damage: mine.damage,
                });
                self.mine_blast_update(mine.id, damage_to_attacker);
            }
        }

        triggered_mines
    }

    pub fn bomb_blast(&mut self, bomb_position: Coords) -> Vec<BuildingResponse> {
        let bomb = &mut self.bombs;
        let mut buildings_damaged: Vec<BuildingResponse> = Vec::new();
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

                let bomb_matrix: HashSet<Coords> = (bomb_position.y - bomb.radius
                    ..bomb_position.y + bomb.radius + 1)
                    .flat_map(|y| {
                        (bomb_position.x - bomb.radius..bomb_position.x + bomb.radius + 1)
                            .map(move |x| Coords { x, y })
                    })
                    .collect();

                let coinciding_coords_damage = building_matrix.intersection(&bomb_matrix).count();

                let damage_buildings: f32 =
                    coinciding_coords_damage as f32 / building_matrix.len() as f32;

                if damage_buildings != 0.0 {
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

                    buildings_damaged.push(BuildingResponse {
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

        self.bombs.total_count -= 1;

        buildings_damaged
    }

    pub fn activate_sentry(&mut self, new_pos: Coords) {
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
                let prev_state = sentry.is_sentry_activated;
                sentry.is_sentry_activated = (sentry.building_data.tile.x - new_pos.x).abs()
                    + (sentry.building_data.tile.y - new_pos.y).abs()
                    <= sentry.building_data.range;
                let new_state = sentry.is_sentry_activated;
                if prev_state != new_state && new_state {
                    log::info!("sentry activated");
                    sentry.sentry_start_time = SystemTime::now();
                } else if prev_state != new_state && !new_state {
                    log::info!("sentry deactivated");
                    sentry.current_bullet_shot_time = SystemTime::now() - Duration::new(2, 0);
                }
            } else {
                sentry.is_sentry_activated = false;
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
                        >= BULLET_COLLISION_TIME && !bullet.has_collided
                    {
                        self.attacker.as_mut().unwrap().attacker_health -= bullet.damage;
                        log::info!(
                            "ATTACKER HEALTH : {}, bullet_id {}",
                            self.attacker.as_mut().unwrap().attacker_health,
                            bullet.bullet_id
                        );
                        bullet.has_collided = true;
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
                log::info!(
                    "sentry id: {}, bullet id: {}",
                    sentry.id,
                    sentry.current_bullet_shot_id
                );
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
                    target_id: 0,
                };
                log::info!(
                    "bullet {} from sentry {}",
                    sentry.current_bullet_shot_id,
                    sentry.id
                );
                shoot_bullet_res_array.push(bullet_response.clone());
                sentry.bullets_shot.push(bullet_response);
            }
        }
        shoot_bullet_res_array
    }

    pub fn defender_movement_one_tick(
        &mut self,
        attacker_position: Coords,
        shortest_path: &HashMap<SourceDestXY, Coords>,
    ) -> DefenderReturnType {
        let attacker = self.attacker.as_mut().unwrap();
        let mut defenders_damaged: Vec<DefenderResponse> = Vec::new();

        for defender in self.defenders.iter_mut() {
            if !defender.is_alive || defender.target_id.is_none() {
                continue;
            }

            let next_hop = shortest_path
                .get(&SourceDestXY {
                    source_x: defender.defender_pos.x,
                    source_y: defender.defender_pos.y,
                    dest_x: attacker_position.x,
                    dest_y: attacker_position.y,
                })
                .unwrap_or(&defender.defender_pos);

            defender.defender_pos = *next_hop;

            // if defender.name.starts_with("Hut") {
            if attacker_position.x == defender.defender_pos.x
                && attacker_position.y == defender.defender_pos.y
            {
                log::info!(
                    "Defender pos {} {} and id {}",
                    defender.defender_pos.x,
                    defender.defender_pos.y,
                    defender.map_space_id
                );

                defenders_damaged.push(DefenderResponse {
                    map_space_id: defender.map_space_id,
                    position: defender.defender_pos,
                    damage: defender.damage,
                });
                defender.is_alive = false;
                attacker.attacker_health = max(0, attacker.attacker_health - defender.damage);
            }
            // }
        }

        // if attacker is dead, no need to move the defenders
        // if attacker.attacker_health == 0 {
        //     return DefenderReturnType {
        //         attacker_health: attacker.attacker_health,
        //         defender_response: defenders_damaged,
        //         state: self.clone(),
        //     };
        // }

        DefenderReturnType {
            attacker_health: attacker.attacker_health,
            defender_response: defenders_damaged,
            state: self.clone(),
        }
    }
}

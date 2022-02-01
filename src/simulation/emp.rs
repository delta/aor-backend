use crate::error::DieselError;
use crate::models::{AttackType, AttackerPath};
use crate::simulation::attacker::Attacker;
use crate::simulation::blocks::{Building, BuildingsManager};
use crate::simulation::error::*;
use crate::simulation::robots::RobotsManager;
use crate::util::function;
use anyhow::Result;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Eq, Hash, PartialEq)]
struct Emp {
    path_id: i32,
    x_coord: i32,
    y_coord: i32,
    radius: i32,
    damage: i32,
}

pub struct Emps(HashMap<i32, HashSet<Emp>>);

impl Emps {
    // Returns a hashmap of Emp with time as key
    pub fn new(conn: &PgConnection, game_id: i32) -> Result<Self> {
        use crate::schema::{attack_type, attacker_path};

        let mut emps = HashMap::new();
        let emp_types: HashMap<i32, AttackType> = attack_type::table
            .load::<AttackType>(conn)
            .map_err(|err| DieselError {
                table: "attack_type",
                function: function!(),
                error: err,
            })?
            .into_iter()
            .map(|emp| (emp.id, emp))
            .collect();

        attacker_path::table
            .filter(attacker_path::game_id.eq(game_id))
            .filter(attacker_path::is_emp.eq(true))
            .load::<AttackerPath>(conn)
            .map_err(|err| DieselError {
                table: "attacker_path",
                function: function!(),
                error: err,
            })?
            .into_iter()
            .try_for_each(|path| {
                if let (Some(emp_type), Some(emp_time)) = (path.emp_type, path.emp_time) {
                    let emp_type = emp_types.get(&emp_type).ok_or(KeyError {
                        key: emp_type,
                        hashmap: "emp_types".to_string(),
                    })?;
                    let emp = Emp {
                        path_id: path.id,
                        x_coord: path.x_coord,
                        y_coord: path.y_coord,
                        radius: emp_type.attack_radius,
                        damage: emp_type.attack_damage,
                    };

                    emps.entry(emp_time).or_insert_with(HashSet::new);
                    emps.get_mut(&emp_time).unwrap().insert(emp);
                    anyhow::Ok(())
                } else {
                    Err(EmpDetailsError { path_id: path.id }.into())
                }
            })?;
        Ok(Emps(emps))
    }

    pub fn simulate(
        &self,
        minute: i32,
        robots_manager: &mut RobotsManager,
        buildings_manager: &mut BuildingsManager,
        attacker: &mut Attacker,
    ) -> Result<()> {
        let Emps(emps) = self;
        if !emps.contains_key(&minute) {
            return Ok(());
        }
        for emp in emps.get(&minute).ok_or(KeyError {
            key: minute,
            hashmap: "emps".to_string(),
        })? {
            if !attacker.is_planted(emp.path_id)? {
                continue;
            }
            let radius = emp.radius;

            let (attacker_x, attacker_y) = attacker.get_current_position()?;
            let attacker_distance =
                (attacker_x - emp.x_coord).pow(2) + (attacker_y - emp.y_coord).pow(2);
            if attacker_distance <= radius.pow(2) {
                attacker.kill();
            }

            let mut affected_buildings: HashSet<i32> = HashSet::new();

            for x in emp.x_coord - radius..=emp.x_coord + radius {
                for y in emp.y_coord - radius..=emp.y_coord + radius {
                    if !(0..40).contains(&x) || !(0..40).contains(&y) {
                        continue;
                    }
                    let distance = (x - emp.x_coord).pow(2) + (y - emp.y_coord).pow(2);
                    if distance > radius.pow(2) {
                        continue;
                    }

                    let building_id = buildings_manager.buildings_grid[x as usize][y as usize];
                    if building_id != 0 {
                        affected_buildings.insert(building_id);
                    } else {
                        // robots on road
                        robots_manager.damage_and_reassign_robots(
                            emp.damage,
                            x,
                            y,
                            buildings_manager,
                        )?;
                    }
                }
            }

            for building_id in &affected_buildings {
                buildings_manager.damage_building(minute, *building_id);
                let Building {
                    absolute_entrance_x: x,
                    absolute_entrance_y: y,
                    ..
                } = buildings_manager.buildings[building_id];
                // robots in affected building
                robots_manager.damage_and_reassign_robots(emp.damage, x, y, buildings_manager)?;
                // robots going to affected building
                let RobotsManager {
                    robots,
                    robots_destination,
                    ..
                } = robots_manager;
                let robots_going_to_building = robots_destination
                    .get(building_id)
                    .ok_or(KeyError {
                        key: *building_id,
                        hashmap: "robots_destination".to_string(),
                    })?
                    .clone();
                for robot_id in robots_going_to_building {
                    let robot = robots.get_mut(&robot_id).ok_or(KeyError {
                        key: robot_id,
                        hashmap: "robots".to_string(),
                    })?;
                    robot.assign_destination(buildings_manager, robots_destination)?;
                }
            }
        }
        Ok(())
    }
}

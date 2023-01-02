use crate::api::attack::util::NewAttacker;
use crate::constants::*;
use crate::error::DieselError;
use crate::util::function;
use anyhow::Result;
use blocks::BuildingsManager;
use diesel::prelude::*;
use robots::RobotsManager;
use serde::Serialize;

use self::attack::AttackManager;

pub mod attack;
pub mod blocks;
pub mod defense;
pub mod error;
pub mod robots;

#[derive(Debug, Serialize)]
pub struct RenderAttacker {
    pub attacker_id: i32,
    pub health: i32,
    pub x_position: i32,
    pub y_position: i32,
    pub is_alive: bool,
    pub emp_id: usize,
    pub attacker_type: i32,
}

#[derive(Debug, Serialize)]
pub struct RenderRobot {
    pub id: i32,
    pub health: i32,
    pub x_position: i32,
    pub y_position: i32,
    pub in_building: bool,
}

#[derive(Debug, Serialize)]
pub struct RenderSimulation {
    pub attackers: Vec<RenderAttacker>,
    pub robots: Vec<RenderRobot>,
}

pub struct Simulator {
    buildings_manager: BuildingsManager,
    robots_manager: RobotsManager,
    attack_manager: AttackManager,
    frames_passed: i32,
    pub no_of_robots: i32,
    pub rating_factor: f32,
}

impl Simulator {
    pub fn new(
        game_id: i32,
        attackers: &Vec<NewAttacker>,
        conn: &mut PgConnection,
    ) -> Result<Self> {
        use crate::schema::{game, levels_fixture, map_layout};

        let map_id = game::table
            .filter(game::id.eq(game_id))
            .select(game::map_layout_id)
            .first::<i32>(conn)
            .map_err(|err| DieselError {
                table: "game",
                function: function!(),
                error: err,
            })?;
        let (no_of_robots, rating_factor) = map_layout::table
            .inner_join(levels_fixture::table)
            .select((levels_fixture::no_of_robots, levels_fixture::rating_factor))
            .filter(map_layout::id.eq(map_id))
            .first::<(i32, f32)>(conn)
            .map_err(|err| DieselError {
                table: "map_layout levels_fixture",
                function: function!(),
                error: err,
            })?;

        let buildings_manager = BuildingsManager::new(conn, map_id)?;
        let robots_manager = RobotsManager::new(&buildings_manager, no_of_robots)?;
        let attack_manager = AttackManager::new(conn, attackers)?;

        Ok(Simulator {
            buildings_manager,
            robots_manager,
            attack_manager,
            frames_passed: 0,
            no_of_robots,
            rating_factor,
        })
    }

    pub fn attacker_allowed(frames_passed: i32) -> bool {
        frames_passed > ATTACKER_RESTRICTED_FRAMES
    }

    pub fn get_minute(frames_passed: i32) -> i32 {
        frames_passed * GAME_MINUTES_PER_FRAME
    }

    pub fn is_hour(frames_passed: i32) -> bool {
        Self::get_minute(frames_passed) % 60 == 0
    }

    pub fn get_hour(frames_passed: i32) -> i32 {
        GAME_START_HOUR + Self::get_minute(frames_passed) / 60
    }

    pub fn get_no_of_robots_destroyed(&self) -> i32 {
        let mut destroyed = 0;
        for r in self.robots_manager.robots.iter() {
            if r.1.health == 0 {
                destroyed += 1;
            }
        }
        destroyed
    }

    pub fn get_damage_done(&self) -> i32 {
        let mut sum_health = 0;
        for r in self.robots_manager.robots.iter() {
            sum_health += r.1.health;
        }
        HEALTH * self.no_of_robots - sum_health
    }

    pub fn get_scores(&self) -> (i32, i32) {
        let damage_done = self.get_damage_done();
        let no_of_robots_destroyed = self.get_no_of_robots_destroyed();
        let max_score = 2 * HEALTH * self.no_of_robots;
        let attack_score = damage_done + HEALTH * no_of_robots_destroyed;
        let defend_score = max_score - attack_score;
        (attack_score, defend_score)
    }

    pub fn simulate(&mut self) -> Result<RenderSimulation> {
        let Simulator {
            buildings_manager,
            robots_manager,
            attack_manager,
            frames_passed,
            ..
        } = self;
        *frames_passed += 1;

        let frames_passed = *frames_passed;

        robots_manager.move_robots(buildings_manager)?;

        //Simulate Emps and attackers
        attack_manager.simulate_attack(frames_passed, robots_manager, buildings_manager)?;

        if Self::is_hour(frames_passed) {
            buildings_manager.update_building_weights(Self::get_hour(frames_passed))?;
        }

        let render_robots: Vec<RenderRobot> = robots_manager
            .robots
            .values()
            .map(|robot| RenderRobot {
                id: robot.id,
                health: robot.health,
                x_position: robot.x_position,
                y_position: robot.y_position,
                in_building: robot.stay_in_time > 0,
            })
            .collect();

        let render_attackers: Result<Vec<RenderAttacker>> = attack_manager
            .attackers
            .values()
            .map(|attacker| {
                let (x_position, y_position) = attacker.get_current_position()?;
                Ok(RenderAttacker {
                    attacker_id: attacker.id,
                    health: attacker.health,
                    x_position,
                    y_position,
                    is_alive: attacker.is_alive,
                    attacker_type: attacker.attacker_type,
                    emp_id: match attacker.path.last() {
                        Some(path) => {
                            if path.is_emp {
                                path.id
                            } else {
                                0
                            }
                        }
                        None => 0,
                    },
                })
            })
            .collect();

        Ok(RenderSimulation {
            attackers: render_attackers?,
            robots: render_robots,
        })
    }
}

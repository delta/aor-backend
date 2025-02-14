use super::schema::*;
use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

#[derive(diesel_derive_enum::DbEnum, Debug, Serialize, Clone, PartialEq, Copy, QueryId)]
#[ExistingTypePath = "crate::schema::sql_types::BlockCategory"]
pub enum BlockCategory {
    Building,
    Defender,
    Mine,
}

// #[derive(diesel_derive_enum::DbEnum, Debug, Serialize, Clone, PartialEq, Copy, Deserialize)]
// #[DieselTypePath = "crate::schema::sql_types::ItemCategory"]
// pub enum ItemCategory {
//     Attacker,
//     Emp,
//     Block,
// }

#[derive(Queryable, Serialize, Clone, Debug)]
pub struct EmpType {
    pub id: i32,
    pub att_type: String,
    pub attack_radius: i32,
    pub attack_damage: i32,
    pub cost: i32,
    pub name: String,
    pub level: i32,
}

#[derive(Queryable, Serialize)]
pub struct AttackType {
    pub id: i32,
    pub att_type: String,
    pub attack_radius: i32,
    pub attack_damage: i32,
}

#[derive(Insertable)]
#[diesel(table_name = attack_type)]
pub struct NewAttackType<'a> {
    pub att_type: &'a str,
    pub attack_radius: &'a i32,
    pub attack_damage: &'a i32,
}

// #[derive(Debug, Clone, Copy)]
// pub struct AttackerPath {
//     pub id: usize,
//     pub y_coord: i32,
//     pub x_coord: i32,
//     pub is_emp: bool,
//     pub emp_type: Option<i32>,
//     pub emp_time: Option<i32>,
// }

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NewAttackerPath {
    pub y_coord: i32,
    pub x_coord: i32,
    pub is_emp: bool,
    pub emp_type: Option<i32>,
    pub emp_time: Option<i32>,
}

#[derive(Queryable, Clone, Debug, Serialize)]
pub struct BuildingType {
    pub id: i32,
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub capacity: i32,
    pub level: i32,
    pub cost: i32,
    pub hp: i32,
    pub prop_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = building_type)]
pub struct NewBuildingType<'a> {
    pub name: &'a str,
    pub width: &'a i32,
    pub height: &'a i32,
    pub capacity: &'a i32,
    pub level: &'a i32,
    pub cost: &'a i32,
}

#[derive(Queryable, Debug, Serialize, Deserialize)]
pub struct Artifact {
    pub map_space_id: i32,
    pub count: i32,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = artifact)]
pub struct NewArtifact {
    pub map_space_id: i32,
    pub count: i32,
}

#[derive(Queryable, Debug, Serialize, Deserialize)]
pub struct AvailableAttackers {
    pub id: i32,
    pub user_id: i32,
    pub attacker_type_id: i32,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = available_attackers)]
pub struct NewAvailableAttackers {
    pub user_id: i32,
    pub attacker_type_id: i32,
}

#[derive(Queryable, Debug, Serialize, Deserialize)]
pub struct AvailableEmps {
    pub id: i32,
    pub user_id: i32,
    pub emp_type_id: i32,
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = available_emps)]
pub struct NewAvailableEmps {
    pub user_id: i32,
    pub emp_type_id: i32,
}

#[derive(Queryable, Serialize, Deserialize, Debug)]
pub struct Game {
    pub id: i32,
    pub attack_id: i32,
    pub defend_id: i32,
    pub map_layout_id: i32,
    pub attack_score: i32,
    pub defend_score: i32,
    pub emps_used: i32,
    pub damage_done: i32,
    pub is_game_over: bool,
    pub artifacts_collected: i32,
    pub date: NaiveDate,
}

#[derive(Insertable)]
#[diesel(table_name = game)]
pub struct NewGame<'a> {
    pub attack_id: &'a i32,
    pub defend_id: &'a i32,
    pub map_layout_id: &'a i32,
    pub attack_score: &'a i32, //attacker's trophy change
    pub defend_score: &'a i32, //defender's trophy change
    pub artifacts_collected: &'a i32,
    pub emps_used: &'a i32,
    pub damage_done: &'a i32,
    pub is_game_over: &'a bool,
    pub date: &'a NaiveDate,
}

#[derive(Queryable, Serialize)]
pub struct LevelsFixture {
    pub id: i32,
    pub start_date: NaiveDateTime,
    pub end_date: NaiveDateTime,
    pub no_of_bombs: i32,
    pub rating_factor: f32,
    pub no_of_attackers: i32,
}

#[derive(Insertable)]
#[diesel(table_name = levels_fixture)]
pub struct NewLevelFixture<'a> {
    pub start_date: &'a NaiveDateTime,
    pub end_date: &'a NaiveDateTime,
    pub no_of_bombs: &'a i32,
    pub rating_factor: &'a f32,
    pub no_of_attackers: &'a i32,
}

#[derive(Queryable, Serialize)]
pub struct LevelConstraints {
    pub level_id: i32,
    pub no_of_blocks: i32,
    pub block_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = level_constraints)]
pub struct NewLevelConstraint<'a> {
    pub level_id: &'a i32,
    pub no_of_blocks: &'a i32,
    pub block_id: &'a i32,
}

#[derive(Clone, Queryable, Serialize)]
pub struct MapLayout {
    pub id: i32,
    pub player: i32,
    pub level_id: i32,
    pub is_valid: bool,
}

#[derive(Insertable)]
#[diesel(table_name = map_layout)]
pub struct NewMapLayout<'a> {
    pub player: &'a i32,
    pub level_id: &'a i32,
    pub is_valid: &'a bool,
}

#[derive(Queryable, Debug, Serialize, Deserialize, Clone)]
pub struct MapSpaces {
    pub id: i32,
    pub map_id: i32,
    pub x_coordinate: i32,
    pub y_coordinate: i32,
    pub block_type_id: i32,
}

#[derive(Deserialize, Insertable, Queryable)]
#[diesel(table_name = map_spaces)]
pub struct NewMapSpaces {
    pub map_id: i32,
    pub x_coordinate: i32,
    pub y_coordinate: i32,
    pub block_type_id: i32,
}

// #[derive(Queryable, Debug)]
// pub struct ShortestPath {
//     pub base_id: i32,
//     pub source_x: i32,
//     pub source_y: i32,
//     pub dest_x: i32,
//     pub dest_y: i32,
//     pub next_hop_x: i32,
//     pub next_hop_y: i32,
// }

#[derive(Insertable, PartialEq)]
#[diesel(table_name = shortest_path)]
pub struct NewShortestPath {
    pub base_id: i32,
    pub source_x: i32,
    pub source_y: i32,
    pub dest_x: i32,
    pub dest_y: i32,
    pub next_hop_x: i32,
    pub next_hop_y: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Queryable)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub username: String,
    pub is_pragyan: bool,
    pub attacks_won: i32,
    pub defenses_won: i32,
    pub trophies: i32,
    pub avatar_id: i32,
    pub artifacts: i32,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = user)]
pub struct NewUser<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub username: &'a str,
    pub is_pragyan: &'a bool,
    pub attacks_won: &'a i32,
    pub defenses_won: &'a i32,
    pub trophies: &'a i32,
    pub avatar_id: &'a i32,
    pub artifacts: &'a i32,
}

#[derive(Queryable, Deserialize, Serialize)]
pub struct SimulationLog {
    pub game_id: i32,
    pub log_text: String,
}

#[derive(Insertable)]
#[diesel(table_name = simulation_log)]
pub struct NewSimulationLog<'a> {
    pub game_id: &'a i32,
    pub log_text: &'a str,
}

#[derive(AsChangeset, Debug, Deserialize)]
#[diesel(table_name = user)]
pub struct UpdateUser {
    pub username: Option<String>,
    pub avatar_id: Option<i32>,
}

#[derive(Queryable, Clone, Debug, Serialize)]
pub struct MineType {
    pub id: i32,
    pub damage: i32,
    pub level: i32,
    pub cost: i32,
    pub name: String,
    pub prop_id: i32,
}

#[derive(Queryable, Clone, Debug, Serialize)]
pub struct DefenderType {
    pub id: i32,
    pub speed: i32,
    pub damage: i32,
    pub level: i32,
    pub cost: i32,
    pub name: String,
    pub prop_id: i32,
    pub max_health: i32,
}

#[derive(Queryable, Clone, Debug, Serialize)]
pub struct BlockType {
    pub id: i32,
    pub defender_type: Option<i32>,
    pub mine_type: Option<i32>,
    pub category: BlockCategory,
    pub building_type: i32,
}

#[derive(Queryable, Clone, Debug, Serialize)]
#[diesel(table_name = block_type)]
pub struct NewBlockType<'a> {
    pub category: &'a BlockCategory,
    pub category_id: &'a i32,
}

#[derive(Queryable, Clone, Debug, Serialize)]
pub struct AttackerType {
    pub id: i32,
    pub max_health: i32,
    pub speed: i32,
    pub amt_of_emps: i32,
    pub level: i32,
    pub cost: i32,
    pub name: String,
    pub prop_id: i32,
}

#[derive(Queryable, Clone, Debug, Serialize)]
pub struct Prop {
    pub id: i32,
    pub range: i32,
    pub frequency: i32,
}

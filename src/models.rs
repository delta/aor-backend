use super::schema::*;
use chrono::NaiveDate;

#[derive(Queryable)]
pub struct AttackType {
    pub id: i32,
    pub att_type: String,
    pub attack_radius: i32,
    pub attack_damage: i32,
}

#[derive(Insertable)]
#[table_name = "attack_type"]
pub struct NewAttackType<'a> {
    pub att_type: &'a str,
    pub attack_radius: &'a i32,
    pub attack_damage: &'a i32,
}

#[derive(Queryable)]
pub struct AttackerPath {
    pub id: i32,
    pub y_coord: i32,
    pub x_coord: i32,
    pub is_emp: bool,
    pub game_id: i32,
    pub emp_type: Option<i32>,
    pub emp_time: Option<i32>,
}

#[derive(Insertable)]
#[table_name = "attacker_path"]
pub struct NewAttackerPath<'a> {
    pub y_coord: &'a i32,
    pub x_coord: &'a i32,
    pub is_emp: &'a bool,
    pub game_id: &'a i32,
    pub emp_type: Option<&'a i32>,
    pub emp_time: Option<&'a i32>,
}

#[derive(Queryable)]
pub struct BlockType {
    pub id: i32,
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub entrance_x: i32,
    pub entrance_y: i32,
}

#[derive(Insertable)]
#[table_name = "block_type"]
pub struct NewBlockType<'a> {
    pub name: &'a str,
    pub width: &'a i32,
    pub height: &'a i32,
    pub entrance_x: &'a i32,
    pub entrance_y: &'a i32,
}

#[derive(Queryable)]
pub struct BuildingWeights {
    pub time: i32,
    pub building_id: i32,
    pub weight: i32,
}

#[derive(Insertable)]
#[table_name = "building_weights"]
pub struct NewBuildingWeights<'a> {
    pub time: &'a i32,
    pub building_id: &'a i32,
    pub weight: &'a i32,
}

#[derive(Queryable)]
pub struct Game {
    pub id: i32,
    pub attack_id: i32,
    pub defend_id: i32,
    pub map_layout_id: i32,
    pub attack_score: i32,
    pub defend_score: i32,
}

#[derive(Insertable)]
#[table_name = "game"]
pub struct NewGame<'a> {
    pub attack_id: &'a i32,
    pub defend_id: &'a i32,
    pub map_layout_id: &'a i32,
    pub attack_score: &'a i32,
    pub defend_score: &'a i32,
}

#[derive(Queryable)]
pub struct LevelsFixture {
    pub id: i32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub no_of_bombs: i32,
}

#[derive(Insertable)]
#[table_name = "levels_fixture"]
pub struct NewLevelFixture<'a> {
    pub start_date: &'a NaiveDate,
    pub end_date: &'a NaiveDate,
    pub no_of_bombs: &'a i32,
}

#[derive(Queryable)]
pub struct LevelConstraints {
    pub level_id: i32,
    pub block_id: i32,
    pub no_of_buildings: i32,
}

#[derive(Insertable)]
#[table_name = "level_constraints"]
pub struct NewLevelConstraint<'a> {
    pub level_id: &'a i32,
    pub block_id: &'a i32,
    pub no_of_buildings: &'a i32,
}

#[derive(Queryable)]
pub struct MapLayout {
    pub id: i32,
    pub player: i32,
    pub level_id: i32,
}

#[derive(Insertable)]
#[table_name = "map_layout"]
pub struct NewMapLayout<'a> {
    pub player: &'a i32,
    pub level_id: &'a i32,
}

#[derive(Queryable)]
pub struct MapSpaces {
    pub id: i32,
    pub map_id: i32,
    pub blk_type: i32,
    pub x_coordinate: i32,
    pub y_coordinate: i32,
    pub rotation: i32,
}

#[derive(Insertable)]
#[table_name = "map_spaces"]
pub struct NewMapSpaces<'a> {
    pub map_id: &'a i32,
    pub blk_type: &'a i32,
    pub x_coordinate: &'a i32,
    pub y_coordinate: &'a i32,
    pub rotation: &'a i32,
}

#[derive(Queryable)]
pub struct ShortestPath {
    pub base_id: i32,
    pub source_x: i32,
    pub source_y: i32,
    pub dest_x: i32,
    pub dest_y: i32,
    pub pathlist: String,
}

#[derive(Insertable)]
#[table_name = "shortest_path"]
pub struct NewShortestPath<'a> {
    pub base_id: &'a i32,
    pub source_x: &'a i32,
    pub source_y: &'a i32,
    pub dest_x: &'a i32,
    pub dest_y: &'a i32,
    pub pathlist: &'a str,
}

#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub username: String,
    pub overall_rating: i32,
    pub is_pragyan: bool,
    pub password: String,
    pub is_verified: bool,
}

#[derive(Insertable)]
#[table_name = "user"]
pub struct NewUser<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub phone: &'a str,
    pub username: &'a str,
    pub overall_rating: &'a i32,
    pub is_pragyan: &'a bool,
    pub password: &'a str,
    pub is_verified: &'a bool,
}

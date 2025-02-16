use crate::api::attack::socket::{BaseItemsDamageResponse, DefenderResponse};
use crate::api::defense::util::SimulationBaseResponse;
use crate::api::user::util::fetch_user;
use crate::api::util::can_show_replay;
use crate::error::DieselError;
use crate::models::{Game, LevelsFixture, MapLayout, SimulationLog, User};
use crate::util::function;
use crate::validator::util::{BulletSpawnResponse, CompanionResult, Coords, DefenderDetails, MineResponse};
use anyhow::Result;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl};
use serde::{Deserialize, Serialize};
use flate2::write;
use flate2::read;
use flate2::Compression;
use std::io::Write;
use std::io::Read;
use crate::schema::replays;

#[derive(Queryable, Deserialize, Serialize)]
pub struct UserDetail {
    pub user_id: i32,
    pub username: String,
    pub trophies: i32,
    pub avatar_id: i32,
}

#[derive(Deserialize)]
pub struct LeaderboardQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Deserialize, Serialize)]
pub struct LeaderboardResponse {
    pub leaderboard_entries: Vec<LeaderboardEntry>,
    pub last_page: i64,
}

#[derive(Queryable, Deserialize, Serialize)]
pub struct LeaderboardEntry {
    pub user_id: i32,
    pub name: String,
    pub trophies: i32,
    pub artifacts: i32,
    pub attacks_won: i32,
    pub defenses_won: i32,
    pub avatar_url: i32,
}

#[derive(Serialize, Clone, Debug, Deserialize)]
pub enum EventType {
    PlaceAttacker,
    PlaceCompanion,
    MoveAttackerUp,
    MoveAttackerDown,
    MoveAttackerLeft,
    MoveAttackerRight,
    AttackerStationary,
    NewCompanionTarget,
    MineBlast,
    PlaceBomb,
    BulletShooting,
    DefenderCollidedWithAttacker,
    DefenderCollidedWithCompanion,
    HutDefenderSpawn,
    DefenderActivated,
    SelfDestruction,
    GameOver,
    GameStart,
}

#[derive(Serialize, Clone, Debug, Deserialize)]
pub struct EventResponse {
    pub attacker_initial_position: Option<Coords>,
    // pub companion_initial_position: Option<Coords>,
    pub companion_result: Option<CompanionResult>,
    pub hut_defender_details: Option<Vec<DefenderDetails>>,
    pub defender_details: Option<Vec<DefenderResponse>>,
    pub mine_details: Option<Vec<MineResponse>>,
    pub bomb_details: Option<BaseItemsDamageResponse>,
    pub bullets_details: Option<Vec<BulletSpawnResponse>>,
    pub event_type: EventType,
    pub attacker_type: Option<i32>,
    pub bomb_type: Option<i32>,
}

#[derive(Serialize, Clone, Debug)]
pub struct  ResultResponse {
    pub damage_done: i32,           //damage_done
    pub artifacts_collected: i32,   //artifacts_collected
    pub bombs_used: i32,            //bombs_used
    pub attackers_used: i32,        //attackers_used
    pub new_attacker_trophies: i32, //new_attacker_trophies
    pub new_defender_trophies: i32, //new_defender_trophies
    pub old_attacker_trophies: i32, //old_attacker_trophies
    pub old_defender_trophies: i32, //old_defender_trophies
}

#[derive(Serialize, Clone, Debug, Deserialize)]
pub struct EventLog {
    // pub current_base_details: SimulationBaseResponse, //base
    pub event: EventResponse,        //events
    // pub date: chrono::NaiveDateTime, //date
    pub frame_no: i32,               //frame_no
}

#[derive(Serialize, Clone)]
pub struct AttackLog {
    pub game_id: i32,   //game_id
    pub attacker: User, //attacker
    pub defender: User, //defender
    pub base_details: SimulationBaseResponse,
    pub result: ResultResponse, //result
    pub game_log: Vec<EventLog>,
}

#[derive(Serialize, Clone, Insertable, Debug)]
#[diesel(table_name = replays)]
pub struct NewReplay {
    pub game_id: i32,
    pub attacker_id: i32,
    pub defender_id: i32,
    pub base_data: Vec<u8>,
    pub game_data: Vec<u8>,
}

#[derive(Serialize, Clone, Queryable, Debug)]
pub struct Replay {
    pub game_id: i32,
    pub attacker_id: i32,
    pub defender_id: i32,
    pub base_data: Vec<u8>,
    pub game_data: Vec<u8>,
}

pub fn get_leaderboard(
    page: i64,
    limit: i64,
    conn: &mut PgConnection,
) -> Result<LeaderboardResponse> {
    use crate::schema::user;

    let total_entries: i64 = user::table
        .count()
        .get_result(conn)
        .map_err(|err| DieselError {
            table: "user",
            function: function!(),
            error: err,
        })?;
    let off_set: i64 = (page - 1) * limit;
    let last_page: i64 = (total_entries as f64 / limit as f64).ceil() as i64;

    let leaderboard_entries = user::table
        .filter(user::is_pragyan.eq(false))
        .select((
            user::id,
            user::username,
            user::trophies,
            user::artifacts,
            user::attacks_won,
            user::defenses_won,
            user::avatar_id,
        ))
        .order_by(user::trophies.desc())
        .offset(off_set)
        .limit(limit)
        .load::<(i32, String, i32, i32, i32, i32, i32)>(conn)
        .map_err(|err| DieselError {
            table: "user_join_map_layout",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(
            |(id, name, trophies, artifacts, attacks_won, defenses_won, avatar_id)| {
                LeaderboardEntry {
                    user_id: id,
                    name,
                    trophies,
                    artifacts,
                    attacks_won,
                    defenses_won,
                    avatar_url: avatar_id,
                }
            },
        )
        .collect();

    Ok(LeaderboardResponse {
        leaderboard_entries,
        last_page,
    })
}

pub fn fetch_is_replay_allowed(
    game_id: i32,
    user_id: i32,
    conn: &mut PgConnection,
) -> Result<bool> {
    use crate::schema::{game, levels_fixture, map_layout};

    let joined_table = game::table.inner_join(map_layout::table.inner_join(levels_fixture::table));
    let result = joined_table
        .filter(game::id.eq(game_id))
        .first::<(Game, (MapLayout, LevelsFixture))>(conn)
        .optional()?;

    if let Some((game, (_, fixture))) = result {
        return Ok(can_show_replay(user_id, &game, &fixture));
    }

    Ok(false)
}

pub fn fetch_replay(game_id: i32, conn: &mut PgConnection) -> Result<SimulationLog> {
    use crate::schema::simulation_log;
    Ok(simulation_log::table
        .filter(simulation_log::game_id.eq(game_id))
        .first(conn)
        .map_err(|err| DieselError {
            table: "simulation_log",
            function: function!(),
            error: err,
        })?)
}

pub fn fetch_game_details(game_id: i32, user_id: i32, conn: &mut PgConnection) -> Result<Game> {
    use crate::schema::game;

    Ok(game::table
        .filter(game::id.eq(game_id))
        .filter(game::attack_id.eq(user_id).or(game::defend_id.eq(user_id)))
        .first(conn)
        .map_err(|err| DieselError {
            table: "game",
            function: function!(),
            error: err,
        })?)
}

pub fn add_game_to_replays(log: &mut AttackLog, conn: &mut PgConnection) -> Result<()> {
    use crate::schema::replays;
    sort_by_frames(&mut log.game_log);
    let game_data: String = serde_json::to_string(&log.game_log).unwrap();
    let base_data: String = serde_json::to_string(&log.base_details).unwrap();
    let compressed_base_data = compress_string(base_data);
    let compressed_game_data = compress_string(game_data);
    let new_replay = NewReplay {
        game_id: log.game_id,
        attacker_id: log.attacker.id,
        defender_id: log.defender.id,
        base_data: compressed_base_data,
        game_data: compressed_game_data,
    };

    diesel::insert_into(replays::table)
        .values(new_replay)
        .on_conflict_do_nothing()
        .execute(conn)
        .map_err(|err| DieselError {
            table: "replays",
            function: function!(),
            error: err,
        })?;
    // get_replay(log.game_id, conn)?;
    Ok(())
}

pub fn get_replay(game_id: i32, conn: &mut PgConnection) -> Result<AttackLog> {
    use crate::schema::replays;
    let replay = replays::table
        .filter(replays::game_id.eq(&game_id))
        .first::<Replay>(conn)
        .map_err(|err| DieselError {
            table: "replays",
            function: function!(),
            error: err,
        })?;
    let decompressed_replay = decompress_string(&replay.game_data);
    let decompressed_base = decompress_string(&replay.base_data);
    let jsonified_decompressed_replay: Vec<EventLog> = serde_json::from_str(&decompressed_replay).unwrap();
    let jsonified_decompressed_base: SimulationBaseResponse = serde_json::from_str(&decompressed_base).unwrap();
    let attacker = fetch_user(conn, replay.attacker_id)?.expect("Attacker not found");
    let defender = fetch_user(conn, replay.attacker_id)?.expect("Defender not found");
    let game_result = fetch_game_details(game_id, replay.defender_id, conn)?;
    let attack_log: AttackLog = AttackLog {
        game_id: replay.game_id,
        attacker: attacker,
        defender: defender,
        base_details: jsonified_decompressed_base,
        game_log: jsonified_decompressed_replay,
        result: ResultResponse {
            damage_done: game_result.damage_done,
            artifacts_collected: game_result.artifacts_collected,
            bombs_used: game_result.emps_used,
            attackers_used: 0,
            new_attacker_trophies: game_result.attack_score,
            new_defender_trophies: game_result.defend_score,
            old_attacker_trophies: 0,
            old_defender_trophies: 0,
        },
    };
    Ok(attack_log)
}

pub fn compress_string(input: String) -> Vec<u8> {
    let mut encoder = write::GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(input.as_bytes()).unwrap();
    encoder.finish().unwrap()
}

pub fn decompress_string(compressed: &[u8]) -> String {
    let mut decoder = read::GzDecoder::new(compressed);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed).unwrap();
    decompressed
}

pub fn sort_by_frames(game_log: &mut Vec<EventLog>) {
    game_log.sort_by(|a, b| a.frame_no.cmp(&b.frame_no));
}
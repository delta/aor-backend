use anyhow::Ok;
use aot_backend::constants::BANK_BUILDING_NAME;
use aot_backend::models::BlockCategory;
use aot_backend::schema::{artifact, block_type, building_type, map_spaces, user};
use aot_backend::util;
use diesel::prelude::*;
use diesel::QueryDsl;

fn main() {
    let artifacts_to_increase_for_all_players = 750;

    let pool = util::get_pg_conn_pool();
    let mut conn = pool.get().expect("Could not retrieve connection from pool");

    // list of all bank's (level 1, 2, 3) space ids
    let map_space_ids: Vec<i32> = map_spaces::table
        .inner_join(block_type::table.inner_join(building_type::table))
        .filter(block_type::category.eq(BlockCategory::Building))
        .filter(building_type::name.like(BANK_BUILDING_NAME))
        .select(map_spaces::id)
        .load::<i32>(&mut conn)
        .expect("Could not get map space ids");

    let _ = Ok(conn.transaction(|conn| {
        for map_space_id in map_space_ids {
            diesel::update(artifact::table.filter(artifact::map_space_id.eq(map_space_id)))
                .set(artifact::count.eq(artifact::count + artifacts_to_increase_for_all_players))
                .execute(conn)?;
        }

        diesel::update(user::table)
            .set(user::artifacts.eq(user::artifacts + artifacts_to_increase_for_all_players))
            .execute(conn)?;

        Ok(())
    }));

    println!(
        "Added {} artifacts to all users",
        artifacts_to_increase_for_all_players
    );
}

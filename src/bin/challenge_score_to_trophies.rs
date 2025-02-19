use aot_backend::api::defense::util::{get_bank_map_space_id, get_block_id_of_bank};
use aot_backend::api::inventory::util::get_user_map_id;
use aot_backend::schema::artifact;
use aot_backend::{schema::challenges_responses, schema::user, util};
use diesel::dsl::sum;
use diesel::prelude::*;

fn main() {
    let pg_pool = util::get_pg_conn_pool();
    let mut conn = pg_pool.get().expect("Cannot get connection from pool");

    let resp = challenges_responses::table
        .inner_join(user::table.on(challenges_responses::attacker_id.eq(user::id)))
        .filter(challenges_responses::challenge_id.eq(1))
        .group_by(user::id)
        .select(user::id)
        .order(sum(challenges_responses::score).nullable().desc())
        .load::<i32>(&mut conn)
        .expect("Can't get responses");

    let artifacts_diff = 100;
    let trophies_diff = 50;
    let max_artifacts = 1000;
    let max_trophies = 550;
    let comp_artifacts = 250;
    let comp_trophies = 100;

    for (i, res) in resp.into_iter().enumerate() {
        let rank = i + 1;
        if rank == 1 {
            let artifacts_inc = max_artifacts;
            let trophies_inc = max_trophies;

            diesel::update(user::table.find(res))
                .set((
                    user::artifacts.eq(user::artifacts + artifacts_inc),
                    user::trophies.eq(user::trophies + trophies_inc),
                ))
                .execute(&mut conn)
                .expect("Cant update user");

            let attacker_map_id = get_user_map_id(res, &mut conn).expect("db error");
            let attacker_bank_block_type_id =
                get_block_id_of_bank(&mut conn, &res).expect("db error");
            let attacker_bank_map_space_id =
                get_bank_map_space_id(&mut conn, &attacker_map_id, &attacker_bank_block_type_id)
                    .expect("db error");

            diesel::update(artifact::table.find(attacker_bank_map_space_id))
                .set(artifact::count.eq(artifact::count + artifacts_inc))
                .execute(&mut conn)
                .expect("unable to add artifacts to bank");
        } else if rank == 2 {
            let artifacts_inc = max_artifacts - artifacts_diff;
            let trophies_inc = max_trophies - trophies_diff;

            diesel::update(user::table.find(res))
                .set((
                    user::artifacts.eq(user::artifacts + artifacts_inc),
                    user::trophies.eq(user::trophies + trophies_inc),
                ))
                .execute(&mut conn)
                .expect("Cant update user");

            let attacker_map_id = get_user_map_id(res, &mut conn).expect("db error");
            let attacker_bank_block_type_id =
                get_block_id_of_bank(&mut conn, &res).expect("db error");
            let attacker_bank_map_space_id =
                get_bank_map_space_id(&mut conn, &attacker_map_id, &attacker_bank_block_type_id)
                    .expect("db error");

            diesel::update(artifact::table.find(attacker_bank_map_space_id))
                .set(artifact::count.eq(artifact::count + artifacts_inc))
                .execute(&mut conn)
                .expect("unable to add artifacts to bank");
        } else if rank == 3 {
            let artifacts_inc = max_artifacts - 2 * artifacts_diff;
            let trophies_inc = max_trophies - 2 * trophies_diff;

            diesel::update(user::table.find(res))
                .set((
                    user::artifacts.eq(user::artifacts + artifacts_inc),
                    user::trophies.eq(user::trophies + trophies_inc),
                ))
                .execute(&mut conn)
                .expect("Cant update user");

            let attacker_map_id = get_user_map_id(res, &mut conn).expect("db error");
            let attacker_bank_block_type_id =
                get_block_id_of_bank(&mut conn, &res).expect("db error");
            let attacker_bank_map_space_id =
                get_bank_map_space_id(&mut conn, &attacker_map_id, &attacker_bank_block_type_id)
                    .expect("db error");

            diesel::update(artifact::table.find(attacker_bank_map_space_id))
                .set(artifact::count.eq(artifact::count + artifacts_inc))
                .execute(&mut conn)
                .expect("unable to add artifacts to bank");
        } else if rank == 4 {
            let artifacts_inc = max_artifacts - 3 * artifacts_diff;
            let trophies_inc = max_trophies - 3 * trophies_diff;

            diesel::update(user::table.find(res))
                .set((
                    user::artifacts.eq(user::artifacts + artifacts_inc),
                    user::trophies.eq(user::trophies + trophies_inc),
                ))
                .execute(&mut conn)
                .expect("Cant update user");

            let attacker_map_id = get_user_map_id(res, &mut conn).expect("db error");
            let attacker_bank_block_type_id =
                get_block_id_of_bank(&mut conn, &res).expect("db error");
            let attacker_bank_map_space_id =
                get_bank_map_space_id(&mut conn, &attacker_map_id, &attacker_bank_block_type_id)
                    .expect("db error");

            diesel::update(artifact::table.find(attacker_bank_map_space_id))
                .set(artifact::count.eq(artifact::count + artifacts_inc))
                .execute(&mut conn)
                .expect("unable to add artifacts to bank");
        } else if rank == 5 {
            let artifacts_inc = max_artifacts - 4 * artifacts_diff;
            let trophies_inc = max_trophies - 4 * trophies_diff;

            diesel::update(user::table.find(res))
                .set((
                    user::artifacts.eq(user::artifacts + artifacts_inc),
                    user::trophies.eq(user::trophies + trophies_inc),
                ))
                .execute(&mut conn)
                .expect("Cant update user");

            let attacker_map_id = get_user_map_id(res, &mut conn).expect("db error");
            let attacker_bank_block_type_id =
                get_block_id_of_bank(&mut conn, &res).expect("db error");
            let attacker_bank_map_space_id =
                get_bank_map_space_id(&mut conn, &attacker_map_id, &attacker_bank_block_type_id)
                    .expect("db error");

            diesel::update(artifact::table.find(attacker_bank_map_space_id))
                .set(artifact::count.eq(artifact::count + artifacts_inc))
                .execute(&mut conn)
                .expect("unable to add artifacts to bank");
        } else {
            let artifacts_inc = comp_artifacts;
            let trophies_inc = comp_trophies;

            diesel::update(user::table.find(res))
                .set((
                    user::artifacts.eq(user::artifacts + artifacts_inc),
                    user::trophies.eq(user::trophies + trophies_inc),
                ))
                .execute(&mut conn)
                .expect("Cant update user");

            let attacker_map_id = get_user_map_id(res, &mut conn).expect("db error");
            let attacker_bank_block_type_id =
                get_block_id_of_bank(&mut conn, &res).expect("db error");
            let attacker_bank_map_space_id =
                get_bank_map_space_id(&mut conn, &attacker_map_id, &attacker_bank_block_type_id)
                    .expect("db error");

            diesel::update(artifact::table.find(attacker_bank_map_space_id))
                .set(artifact::count.eq(artifact::count + artifacts_inc))
                .execute(&mut conn)
                .expect("unable to add artifacts to bank");
        }
    }

    println!("Added respective scores to every challenge player");
}

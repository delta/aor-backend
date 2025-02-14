// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType, QueryId)]
    #[diesel(postgres_type(name = "block_category"))]
    pub struct BlockCategory;
}

diesel::table! {
    artifact (map_space_id) {
        map_space_id -> Int4,
        count -> Int4,
    }
}

diesel::table! {
    attack_type (id) {
        id -> Int4,
        att_type -> Varchar,
        attack_radius -> Int4,
        attack_damage -> Int4,
    }
}

diesel::table! {
    attacker_type (id) {
        id -> Int4,
        max_health -> Int4,
        speed -> Int4,
        amt_of_emps -> Int4,
        level -> Int4,
        cost -> Int4,
        name -> Varchar,
        prop_id -> Int4,
    }
}

diesel::table! {
    available_attackers (id) {
        id -> Int4,
        user_id -> Int4,
        attacker_type_id -> Int4,
    }
}

diesel::table! {
    available_emps (id) {
        id -> Int4,
        user_id -> Int4,
        emp_type_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::BlockCategory;

    block_type (id) {
        id -> Int4,
        defender_type -> Nullable<Int4>,
        mine_type -> Nullable<Int4>,
        category -> BlockCategory,
        building_type -> Int4,
    }
}

diesel::table! {
    building_type (id) {
        id -> Int4,
        name -> Varchar,
        width -> Int4,
        height -> Int4,
        capacity -> Int4,
        level -> Int4,
        cost -> Int4,
        hp -> Int4,
        prop_id -> Int4,
    }
}

diesel::table! {
    defender_type (id) {
        id -> Int4,
        speed -> Int4,
        damage -> Int4,
        level -> Int4,
        cost -> Int4,
        name -> Varchar,
        prop_id -> Int4,
        max_health -> Int4,
    }
}

diesel::table! {
    emp_type (id) {
        id -> Int4,
        att_type -> Varchar,
        attack_radius -> Int4,
        attack_damage -> Int4,
        cost -> Int4,
        name -> Varchar,
        level -> Int4,
    }
}

diesel::table! {
    game (id) {
        id -> Int4,
        attack_id -> Int4,
        defend_id -> Int4,
        map_layout_id -> Int4,
        attack_score -> Int4,
        defend_score -> Int4,
        emps_used -> Int4,
        damage_done -> Int4,
        is_game_over -> Bool,
        artifacts_collected -> Int4,
        date -> Date,
    }
}

diesel::table! {
    level_constraints (level_id, block_id) {
        level_id -> Int4,
        no_of_blocks -> Int4,
        block_id -> Int4,
    }
}

diesel::table! {
    levels_fixture (id) {
        id -> Int4,
        start_date -> Timestamp,
        end_date -> Timestamp,
        no_of_bombs -> Int4,
        rating_factor -> Float4,
        no_of_attackers -> Int4,
    }
}

diesel::table! {
    map_layout (id) {
        id -> Int4,
        player -> Int4,
        level_id -> Int4,
        is_valid -> Bool,
    }
}

diesel::table! {
    map_spaces (id) {
        id -> Int4,
        map_id -> Int4,
        x_coordinate -> Int4,
        y_coordinate -> Int4,
        block_type_id -> Int4,
    }
}

diesel::table! {
    mine_type (id) {
        id -> Int4,
        damage -> Int4,
        level -> Int4,
        cost -> Int4,
        name -> Varchar,
        prop_id -> Int4,
    }
}

diesel::table! {
    prop (id) {
        id -> Int4,
        range -> Int4,
        frequency -> Int4,
    }
}

diesel::table! {
    shortest_path (base_id, source_x, source_y, dest_x, dest_y) {
        base_id -> Int4,
        source_x -> Int4,
        source_y -> Int4,
        dest_x -> Int4,
        dest_y -> Int4,
        next_hop_x -> Int4,
        next_hop_y -> Int4,
    }
}

diesel::table! {
    simulation_log (game_id) {
        game_id -> Int4,
        log_text -> Text,
    }
}

diesel::table! {
    user (id) {
        id -> Int4,
        name -> Varchar,
        email -> Varchar,
        username -> Varchar,
        is_pragyan -> Bool,
        attacks_won -> Int4,
        defenses_won -> Int4,
        trophies -> Int4,
        avatar_id -> Int4,
        artifacts -> Int4,
    }
}

diesel::joinable!(artifact -> map_spaces (map_space_id));
diesel::joinable!(available_attackers -> user(user_id));
diesel::joinable!(available_attackers -> attacker_type(attacker_type_id));
diesel::joinable!(available_emps -> user(user_id));
diesel::joinable!(available_emps -> emp_type(emp_type_id));
diesel::joinable!(attacker_type -> prop (prop_id));
diesel::joinable!(block_type -> building_type (building_type));
diesel::joinable!(block_type -> defender_type (defender_type));
diesel::joinable!(block_type -> mine_type (mine_type));
diesel::joinable!(building_type -> prop (prop_id));
diesel::joinable!(defender_type -> prop (prop_id));
diesel::joinable!(game -> map_layout (map_layout_id));
diesel::joinable!(level_constraints -> block_type (block_id));
diesel::joinable!(level_constraints -> levels_fixture (level_id));
diesel::joinable!(map_layout -> levels_fixture (level_id));
diesel::joinable!(map_layout -> user (player));
diesel::joinable!(map_spaces -> block_type (block_type_id));
diesel::joinable!(map_spaces -> map_layout (map_id));
diesel::joinable!(mine_type -> prop (prop_id));
diesel::joinable!(shortest_path -> map_layout (base_id));
diesel::joinable!(simulation_log -> game (game_id));

diesel::allow_tables_to_appear_in_same_query!(
    artifact,
    attack_type,
    attacker_type,
    available_attackers,
    available_emps,
    block_type,
    building_type,
    defender_type,
    emp_type,
    game,
    level_constraints,
    levels_fixture,
    map_layout,
    map_spaces,
    mine_type,
    prop,
    shortest_path,
    simulation_log,
    user,
);

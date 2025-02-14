use crate::constants::BANK_BUILDING_NAME;
use crate::error::DieselError;
use crate::models::{
    AttackerType, BlockCategory, BlockType, BuildingType, DefenderType, EmpType, MineType, Prop,
};
use crate::schema::{
    artifact, attacker_type, available_attackers, available_emps, block_type, building_type,
    defender_type, emp_type, mine_type, prop,
};
use crate::schema::{map_layout, map_spaces, user};
use crate::util::function;
use anyhow::{Ok, Result};
use diesel::{dsl::exists, prelude::*, select, PgConnection};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct BuildingTypeResponse {
    id: i32,
    block_id: i32,
    name: String,
    width: i32,
    height: i32,
    capacity: i32,
    level: i32,
    cost: i32,
    hp: i32,
    map_space_id: i32,
    next_level_stats: Option<NextLevelBuildingTypeResponse>,
}
#[derive(Serialize, Deserialize)]

pub struct NextLevelBuildingTypeResponse {
    id: i32,
    block_id: i32,
    name: String,
    width: i32,
    height: i32,
    capacity: i32,
    level: i32,
    cost: i32,
    hp: i32,
}
#[derive(Serialize, Deserialize)]

pub struct AttackerTypeResponse {
    id: i32,
    max_health: i32,
    speed: i32,
    amt_of_emps: i32,
    level: i32,
    cost: i32,
    name: String,
    next_level_stats: Option<NextLevelAttackerTypeResponse>,
}
#[derive(Serialize, Deserialize)]

pub struct NextLevelAttackerTypeResponse {
    id: i32,
    max_health: i32,
    speed: i32,
    amt_of_emps: i32,
    level: i32,
    cost: i32,
    name: String,
}
#[derive(Serialize, Deserialize)]

pub struct DefenderTypeResponse {
    id: i32,
    block_id: i32,
    speed: i32,
    damage: i32,
    radius: i32,
    level: i32,
    cost: i32,
    name: String,
    map_space_id: i32,
    next_level_stats: Option<NextLevelDefenderTypeResponse>,
    max_health: i32,
}
#[derive(Serialize, Deserialize)]

pub struct NextLevelDefenderTypeResponse {
    id: i32,
    block_id: i32,
    speed: i32,
    damage: i32,
    radius: i32,
    level: i32,
    cost: i32,
    name: String,
}
#[derive(Serialize, Deserialize)]

pub struct EmpTypeResponse {
    id: i32,
    att_type: String,
    attack_radius: i32,
    attack_damage: i32,
    cost: i32,
    name: String,
    level: i32,
    next_level_stats: Option<NextLevelEmpTypeResponse>,
}
#[derive(Serialize, Deserialize)]

pub struct NextLevelEmpTypeResponse {
    id: i32,
    att_type: String,
    attack_radius: i32,
    attack_damage: i32,
    cost: i32,
    name: String,
    level: i32,
}
#[derive(Serialize, Deserialize)]

pub struct MineTypeResponse {
    id: i32,
    block_id: i32,
    radius: i32,
    damage: i32,
    level: i32,
    cost: i32,
    name: String,
    map_space_id: i32,
    next_level_stats: Option<NextLevelMineTypeResponse>,
}
#[derive(Serialize, Deserialize)]

pub struct NextLevelMineTypeResponse {
    id: i32,
    block_id: i32,
    radius: i32,
    damage: i32,
    level: i32,
    cost: i32,
    name: String,
}
#[derive(Serialize, Deserialize)]

pub struct InventoryResponse {
    buildings: Vec<BuildingTypeResponse>,
    attackers: Vec<AttackerTypeResponse>,
    defenders: Vec<DefenderTypeResponse>,
    mines: Vec<MineTypeResponse>,
    emps: Vec<EmpTypeResponse>,
}

pub fn get_inventory(player_id: i32, conn: &mut PgConnection) -> Result<InventoryResponse> {
    let buildings = get_building_types(player_id, conn)?;
    let attackers = get_attacker_types(player_id, conn)?;
    let defenders = get_defender_types(player_id, conn)?;
    let mines = get_mine_types(player_id, conn)?;
    let emps = get_emp_types(player_id, conn)?;

    Ok(InventoryResponse {
        buildings,
        attackers,
        defenders,
        mines,
        emps,
    })
}

fn get_building_types(
    player_id: i32,
    conn: &mut PgConnection,
) -> Result<Vec<BuildingTypeResponse>> {
    let joined_table = map_spaces::table
        .inner_join(map_layout::table)
        .inner_join(block_type::table.on(map_spaces::block_type_id.eq(block_type::id)))
        .filter(block_type::category.eq(BlockCategory::Building))
        .inner_join(
            building_type::table.on(block_type::building_type
                .assume_not_null()
                .eq(building_type::id)),
        )
        .filter(map_layout::player.eq(player_id))
        .filter(block_type::category.eq(BlockCategory::Building))
        .select((building_type::all_columns, block_type::id, map_spaces::id));

    let buildings = joined_table
        .load::<(BuildingType, i32, i32)>(conn)
        .map_err(|err| DieselError {
            table: "building_type",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(building_type, block_id, map_space_id)| {
            let max_level: i64 = building_type::table
                .filter(building_type::name.eq(&building_type.name))
                .count()
                .get_result::<i64>(conn)
                .map_err(|err| DieselError {
                    table: "building_type",
                    function: function!(),
                    error: err,
                })
                .unwrap_or(0);
            if building_type.level >= max_level as i32 {
                // The building is at max level
                BuildingTypeResponse {
                    id: building_type.id,
                    block_id,
                    name: building_type.name,
                    width: building_type.width,
                    height: building_type.height,
                    capacity: building_type.capacity,
                    level: building_type.level,
                    cost: building_type.cost,
                    hp: building_type.hp,
                    map_space_id,
                    next_level_stats: None,
                }
            } else {
                let next_level = building_type.level + 1;

                let next_level_stats: (BuildingType, BlockType) = building_type::table
                    .inner_join(block_type::table)
                    .filter(building_type::name.eq(&building_type.name))
                    .filter(building_type::level.eq(next_level))
                    .first::<(BuildingType, BlockType)>(conn)
                    .map_err(|err| DieselError {
                        table: "building_type",
                        function: function!(),
                        error: err,
                    })
                    .unwrap_or((
                        BuildingType {
                            id: 0,
                            name: "".to_string(),
                            width: 0,
                            height: 0,
                            capacity: 0,
                            level: 0,
                            cost: 0,
                            hp: 0,
                            prop_id: 0,
                        },
                        BlockType {
                            id: 0,
                            defender_type: None,
                            mine_type: None,
                            category: BlockCategory::Building,
                            building_type: 0,
                        },
                    ));

                BuildingTypeResponse {
                    id: building_type.id,
                    block_id,
                    name: building_type.name,
                    width: building_type.width,
                    height: building_type.height,
                    capacity: building_type.capacity,
                    level: building_type.level,
                    cost: building_type.cost,
                    hp: building_type.hp,
                    map_space_id,
                    next_level_stats: Some(NextLevelBuildingTypeResponse {
                        id: next_level_stats.0.id,
                        block_id: next_level_stats.1.id,
                        name: next_level_stats.0.name,
                        width: next_level_stats.0.width,
                        height: next_level_stats.0.height,
                        capacity: next_level_stats.0.capacity,
                        level: next_level_stats.0.level,
                        cost: next_level_stats.0.cost,
                        hp: next_level_stats.0.hp,
                    }),
                }
            }
        })
        .collect();

    Ok(buildings)
}

fn get_attacker_types(
    player_id: i32,
    conn: &mut PgConnection,
) -> Result<Vec<AttackerTypeResponse>> {
    let joined_table = available_attackers::table
        .inner_join(attacker_type::table)
        .filter(available_attackers::user_id.eq(player_id))
        .select(attacker_type::all_columns);

    let attackers = joined_table
        .load::<AttackerType>(conn)
        .map_err(|err| DieselError {
            table: "attacker_type",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|attacker_type| {
            let max_level: i64 = attacker_type::table
                .filter(attacker_type::name.eq(&attacker_type.name))
                .count()
                .get_result::<i64>(conn)
                .map_err(|err| DieselError {
                    table: "attacker_type",
                    function: function!(),
                    error: err,
                })
                .unwrap_or(0);
            if attacker_type.level >= max_level as i32 {
                // The attacker is at max level
                AttackerTypeResponse {
                    id: attacker_type.id,
                    max_health: attacker_type.max_health,
                    speed: attacker_type.speed,
                    amt_of_emps: attacker_type.amt_of_emps,
                    level: attacker_type.level,
                    cost: attacker_type.cost,
                    name: attacker_type.name,
                    next_level_stats: None,
                }
            } else {
                let next_level = attacker_type.level + 1;
                let next_level_stats = attacker_type::table
                    .filter(attacker_type::name.eq(&attacker_type.name))
                    .filter(attacker_type::level.eq(next_level))
                    .first::<AttackerType>(conn)
                    .map_err(|err| DieselError {
                        table: "attacker_type",
                        function: function!(),
                        error: err,
                    })
                    .unwrap_or(AttackerType {
                        id: 0,
                        max_health: 0,
                        speed: 0,
                        amt_of_emps: 0,
                        level: 0,
                        cost: 0,
                        name: "".to_string(),
                        prop_id: 0,
                    });
                AttackerTypeResponse {
                    id: attacker_type.id,
                    max_health: attacker_type.max_health,
                    speed: attacker_type.speed,
                    amt_of_emps: attacker_type.amt_of_emps,
                    level: attacker_type.level,
                    cost: attacker_type.cost,
                    name: attacker_type.name,
                    next_level_stats: Some(NextLevelAttackerTypeResponse {
                        id: next_level_stats.id,
                        max_health: next_level_stats.max_health,
                        speed: next_level_stats.speed,
                        amt_of_emps: next_level_stats.amt_of_emps,
                        level: next_level_stats.level,
                        cost: next_level_stats.cost,
                        name: next_level_stats.name,
                    }),
                }
            }
        })
        .collect();

    Ok(attackers)
}

fn get_defender_types(
    player_id: i32,
    conn: &mut PgConnection,
) -> Result<Vec<DefenderTypeResponse>> {
    let joined_table = map_spaces::table
        .inner_join(map_layout::table)
        .inner_join(
            block_type::table
                .inner_join(
                    defender_type::table.on(block_type::defender_type
                        .assume_not_null()
                        .eq(defender_type::id)
                        .and(block_type::category.eq(BlockCategory::Defender))),
                )
                .on(block_type::id.eq(map_spaces::block_type_id)),
        )
        .inner_join(prop::table.on(defender_type::prop_id.eq(prop::id)))
        .filter(map_layout::player.eq(player_id))
        .filter(block_type::category.eq(BlockCategory::Defender))
        .select((
            defender_type::all_columns,
            block_type::id,
            prop::all_columns,
            map_spaces::id,
        ));

    let defenders = joined_table
        .load::<(DefenderType, i32, Prop, i32)>(conn)
        .map_err(|err| DieselError {
            table: "defender_type",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(defender_type, block_id, prop, map_space_id)| {
            let max_level: i64 = defender_type::table
                .filter(defender_type::name.eq(&defender_type.name))
                .count()
                .get_result::<i64>(conn)
                .map_err(|err| DieselError {
                    table: "defender_type",
                    function: function!(),
                    error: err,
                })
                .unwrap_or(0);
            if defender_type.level >= max_level as i32 {
                //the defender is at max level
                DefenderTypeResponse {
                    id: defender_type.id,
                    block_id,
                    speed: defender_type.speed,
                    damage: defender_type.damage,
                    radius: prop.range,
                    level: defender_type.level,
                    cost: defender_type.cost,
                    name: defender_type.name,
                    map_space_id,
                    next_level_stats: None,
                    max_health: defender_type.max_health,
                }
            } else {
                let next_level = defender_type.level + 1;

                let next_level_stats: (DefenderType, BlockType, Prop) = defender_type::table
                    .inner_join(block_type::table)
                    .inner_join(prop::table.on(defender_type::prop_id.eq(prop::id)))
                    .filter(defender_type::name.eq(&defender_type.name))
                    .filter(defender_type::level.eq(next_level))
                    .first::<(DefenderType, BlockType, Prop)>(conn)
                    .map_err(|err| DieselError {
                        table: "building_type",
                        function: function!(),
                        error: err,
                    })
                    .unwrap_or((
                        DefenderType {
                            id: 0,
                            speed: 0,
                            damage: 0,
                            level: 0,
                            cost: 0,
                            name: "".to_string(),
                            prop_id: 0,
                            max_health: 0,
                        },
                        BlockType {
                            id: 0,
                            defender_type: Some(0),
                            mine_type: None,
                            category: BlockCategory::Defender,
                            building_type: 0,
                        },
                        Prop {
                            id: 0,
                            range: 0,
                            frequency: 0,
                        },
                    ));

                DefenderTypeResponse {
                    id: defender_type.id,
                    block_id,
                    speed: defender_type.speed,
                    damage: defender_type.damage,
                    radius: prop.range,
                    level: defender_type.level,
                    cost: defender_type.cost,
                    name: defender_type.name,
                    map_space_id,
                    next_level_stats: Some(NextLevelDefenderTypeResponse {
                        id: next_level_stats.0.id,
                        block_id: next_level_stats.1.id,
                        speed: next_level_stats.0.speed,
                        damage: next_level_stats.0.damage,
                        radius: next_level_stats.2.range,
                        level: next_level_stats.0.level,
                        cost: next_level_stats.0.cost,
                        name: next_level_stats.0.name,
                    }),
                    max_health: defender_type.max_health,
                }
            }
        })
        .collect();

    Ok(defenders)
}

fn get_mine_types(player_id: i32, conn: &mut PgConnection) -> Result<Vec<MineTypeResponse>> {
    let joined_table = map_spaces::table
        .inner_join(map_layout::table)
        .inner_join(block_type::table.on(map_spaces::block_type_id.eq(block_type::id)))
        .filter(block_type::category.eq(BlockCategory::Mine))
        .inner_join(mine_type::table.on(block_type::mine_type.assume_not_null().eq(mine_type::id)))
        .inner_join(prop::table.on(mine_type::prop_id.eq(prop::id)))
        .filter(map_layout::player.eq(player_id))
        .select((
            mine_type::all_columns,
            block_type::id,
            prop::all_columns,
            map_spaces::id,
        ));

    let mines = joined_table
        .load::<(MineType, i32, Prop, i32)>(conn)
        .map_err(|err| DieselError {
            table: "mine_type",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(mine_type, block_id, prop, map_space_id)| {
            let max_level: i64 = mine_type::table
                .filter(mine_type::name.eq(&mine_type.name))
                .count()
                .get_result::<i64>(conn)
                .map_err(|err| DieselError {
                    table: "mine_type",
                    function: function!(),
                    error: err,
                })
                .unwrap_or(0);

            if mine_type.level >= max_level as i32 {
                //mine is at max level
                MineTypeResponse {
                    id: mine_type.id,
                    block_id,
                    radius: prop.range,
                    damage: mine_type.damage,
                    level: mine_type.level,
                    cost: mine_type.cost,
                    name: mine_type.name,
                    map_space_id,
                    next_level_stats: None,
                }
            } else {
                let next_level = mine_type.level + 1;

                let next_level_stats: (MineType, BlockType, Prop) = mine_type::table
                    .inner_join(block_type::table)
                    .inner_join(prop::table.on(mine_type::prop_id.eq(prop::id)))
                    .filter(mine_type::name.eq(&mine_type.name))
                    .filter(mine_type::level.eq(next_level))
                    .first::<(MineType, BlockType, Prop)>(conn)
                    .map_err(|err| DieselError {
                        table: "building_type",
                        function: function!(),
                        error: err,
                    })
                    .unwrap_or((
                        MineType {
                            id: 0,
                            damage: 0,
                            level: 0,
                            cost: 0,
                            name: "".to_string(),
                            prop_id: 0,
                        },
                        BlockType {
                            id: 0,
                            defender_type: None,
                            mine_type: Some(0),
                            category: BlockCategory::Mine,
                            building_type: 0,
                        },
                        Prop {
                            id: 0,
                            range: 0,
                            frequency: 0,
                        },
                    ));

                MineTypeResponse {
                    id: mine_type.id,
                    block_id,
                    radius: prop.range,
                    damage: mine_type.damage,
                    level: mine_type.level,
                    cost: mine_type.cost,
                    name: mine_type.name,
                    map_space_id,
                    next_level_stats: Some(NextLevelMineTypeResponse {
                        id: next_level_stats.0.id,
                        block_id: next_level_stats.1.id,
                        radius: next_level_stats.2.range,
                        damage: next_level_stats.0.damage,
                        level: next_level_stats.0.level,
                        cost: next_level_stats.0.cost,
                        name: next_level_stats.0.name,
                    }),
                }
            }
        })
        .collect();

    Ok(mines)
}

fn get_emp_types(player_id: i32, conn: &mut PgConnection) -> Result<Vec<EmpTypeResponse>> {
    let joined_table = available_emps::table
        .inner_join(emp_type::table)
        .filter(available_emps::user_id.eq(player_id))
        .select(emp_type::all_columns);

    let emps = joined_table
        .load::<EmpType>(conn)
        .map_err(|err| DieselError {
            table: "emp_type",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|emp_type| {
            let max_level: i64 = emp_type::table
                .filter(emp_type::name.eq(&emp_type.name))
                .count()
                .get_result::<i64>(conn)
                .map_err(|err| DieselError {
                    table: "emp_type",
                    function: function!(),
                    error: err,
                })
                .unwrap_or(0);
            if emp_type.level >= max_level as i32 {
                // The emp is at max level
                EmpTypeResponse {
                    id: emp_type.id,
                    att_type: emp_type.att_type,
                    attack_radius: emp_type.attack_radius,
                    attack_damage: emp_type.attack_damage,
                    cost: emp_type.cost,
                    name: emp_type.name,
                    level: emp_type.level,
                    next_level_stats: None,
                }
            } else {
                let next_level = emp_type.level + 1;
                let next_level_stats = emp_type::table
                    .filter(emp_type::name.eq(&emp_type.name))
                    .filter(emp_type::level.eq(next_level))
                    .first::<EmpType>(conn)
                    .map_err(|err| DieselError {
                        table: "emp_type",
                        function: function!(),
                        error: err,
                    })
                    .unwrap_or(EmpType {
                        id: 0,
                        att_type: "".to_string(),
                        attack_radius: 0,
                        attack_damage: 0,
                        cost: 0,
                        name: "".to_string(),
                        level: 0,
                    });
                EmpTypeResponse {
                    id: emp_type.id,
                    att_type: emp_type.att_type,
                    attack_radius: emp_type.attack_radius,
                    attack_damage: emp_type.attack_damage,
                    cost: emp_type.cost,
                    name: emp_type.name,
                    level: emp_type.level,
                    next_level_stats: Some(NextLevelEmpTypeResponse {
                        id: next_level_stats.id,
                        att_type: next_level_stats.att_type,
                        attack_radius: next_level_stats.attack_radius,
                        attack_damage: next_level_stats.attack_damage,
                        cost: next_level_stats.cost,
                        name: next_level_stats.name,
                        level: next_level_stats.level,
                    }),
                }
            }
        })
        .collect();

    Ok(emps)
}

pub(crate) fn upgrade_building(
    player_id: i32,
    conn: &mut PgConnection,
    block_id: i32,
    map_space_id: i32,
) -> Result<i32> {
    let user_artifacts = get_user_artifacts(player_id, conn)?;

    //check if the given block id is a building
    //check if the given user has the block id
    let exists = select(exists(
        map_spaces::table
            .inner_join(map_layout::table)
            .inner_join(block_type::table.on(block_type::id.eq(map_spaces::block_type_id)))
            .filter(block_type::category.eq(BlockCategory::Building))
            .filter(map_layout::player.eq(player_id))
            .filter(map_spaces::id.eq(map_space_id)),
    ))
    .get_result::<bool>(conn)?;

    if !exists {
        return Err(anyhow::anyhow!(
            "either Block is not a building or the user does not have the block"
        ));
    }

    let joined_table = map_spaces::table
        .inner_join(map_layout::table)
        .inner_join(block_type::table.on(map_spaces::block_type_id.eq(block_type::id)))
        .inner_join(
            building_type::table.on(block_type::building_type
                .assume_not_null()
                .eq(building_type::id)),
        )
        .filter(block_type::category.eq(BlockCategory::Building))
        .filter(map_layout::player.eq(player_id))
        .filter(block_type::id.eq(block_id));

    let (cost, level, name, map_space_id): (i32, i32, String, i32) = joined_table
        .select((
            building_type::cost,
            building_type::level,
            building_type::name,
            map_spaces::id,
        ))
        .first::<(i32, i32, String, i32)>(conn)
        .map_err(|err| DieselError {
            table: "building_type",
            function: function!(),
            error: err,
        })?;

    let max_level: i64 = building_type::table
        .filter(building_type::name.eq(&name))
        .count()
        .get_result::<i64>(conn)
        .map_err(|err| DieselError {
            table: "building_type",
            function: function!(),
            error: err,
        })?;

    if level >= max_level as i32 {
        return Err(anyhow::anyhow!("Building is at max level"));
    };
    if cost > user_artifacts {
        return Err(anyhow::anyhow!("Not enough artifacts"));
    };

    let joined_table = block_type::table
        .inner_join(building_type::table)
        .filter(block_type::category.eq(BlockCategory::Building));

    let next_level_block: (i32, String) = joined_table
        .filter(building_type::name.eq(name))
        .filter(building_type::level.eq(level + 1))
        .select((block_type::id, building_type::name))
        .first::<(i32, String)>(conn)
        .map_err(|err| DieselError {
            table: "building_type",
            function: function!(),
            error: err,
        })?;

    let id_of_map = get_user_map_id(player_id, conn)?;
    let bank_block_type_id = get_block_id_of_bank(conn, &player_id)?;
    let bank_map_space_id = get_bank_map_space_id(conn, &id_of_map, &bank_block_type_id)?;
    let artifacts_in_bank = get_building_artifact_count(conn, &id_of_map, &bank_map_space_id)?;
    if artifacts_in_bank < cost {
        return Err(anyhow::anyhow!("Not enough artifacts in bank"));
    }
    let _ = run_transaction(
        conn,
        block_id,
        next_level_block.0,
        player_id,
        cost,
        user_artifacts,
        bank_map_space_id,
        true,
        map_space_id,
    );

    let building_map_space_id = get_building_map_space_id(conn, &id_of_map, &next_level_block.0)?;
    Ok(building_map_space_id)
}

pub(crate) fn upgrade_defender(
    player_id: i32,
    conn: &mut PgConnection,
    block_id: i32,
) -> Result<()> {
    let user_artifacts = get_user_artifacts(player_id, conn)?;

    //check if the given block id is a defender
    //check if the given user has the block id
    let exists = select(exists(
        map_spaces::table
            .inner_join(map_layout::table)
            .inner_join(block_type::table.on(map_spaces::block_type_id.eq(block_type::id)))
            .filter(map_layout::player.eq(player_id))
            .filter(map_spaces::block_type_id.eq(block_id))
            .filter(block_type::category.eq(BlockCategory::Defender)),
    ))
    .get_result::<bool>(conn)?;

    if !exists {
        return Err(anyhow::anyhow!(
            "either Block is not a defender or the user does not have the block"
        ));
    }

    let joined_table = map_spaces::table
        .inner_join(map_layout::table)
        .inner_join(block_type::table)
        .filter(block_type::category.eq(BlockCategory::Defender))
        .inner_join(
            defender_type::table.on(block_type::defender_type
                .assume_not_null()
                .eq(defender_type::id)),
        )
        .filter(map_layout::player.eq(player_id))
        .filter(map_spaces::block_type_id.eq(block_id));

    let (cost, level, name, _): (i32, i32, String, i32) = joined_table
        .select((
            defender_type::cost,
            defender_type::level,
            defender_type::name,
            map_spaces::id,
        ))
        .first::<(i32, i32, String, i32)>(conn)
        .map_err(|err| DieselError {
            table: "defender_type",
            function: function!(),
            error: err,
        })?;

    let max_level: i64 = defender_type::table
        .filter(defender_type::name.eq(&name))
        .count()
        .get_result::<i64>(conn)
        .map_err(|err| DieselError {
            table: "defender_type",
            function: function!(),
            error: err,
        })?;

    if level >= max_level as i32 {
        return Err(anyhow::anyhow!("Defender is at max level"));
    };
    if cost > user_artifacts {
        return Err(anyhow::anyhow!("Not enough artifacts"));
    };

    let joined_table = block_type::table
        .inner_join(defender_type::table)
        .filter(block_type::category.eq(BlockCategory::Defender));

    let next_level_block_id: i32 = joined_table
        .filter(defender_type::name.eq(name))
        .filter(defender_type::level.eq(level + 1))
        .select(block_type::id)
        .first::<i32>(conn)
        .map_err(|err| DieselError {
            table: "defender_type",
            function: function!(),
            error: err,
        })?;

    let id_of_map = get_user_map_id(player_id, conn)?;
    let bank_block_type_id = get_block_id_of_bank(conn, &player_id)?;
    let bank_map_space_id = get_bank_map_space_id(conn, &id_of_map, &bank_block_type_id)?;
    let artifacts_in_bank = get_building_artifact_count(conn, &id_of_map, &bank_map_space_id)?;
    if artifacts_in_bank < cost {
        return Err(anyhow::anyhow!("Not enough artifacts in bank"));
    }

    conn.transaction(|conn| {
        let id_of_map = get_user_map_id(player_id, conn)?;

        diesel::update(user::table.filter(user::id.eq(player_id)))
            .set(user::artifacts.eq(user_artifacts - cost))
            .execute(conn)?;

        //update artifacts in bank
        diesel::update(artifact::table.filter(artifact::map_space_id.eq(bank_map_space_id)))
            .set(artifact::count.eq(artifact::count - cost))
            .execute(conn)
            .map_err(|err| DieselError {
                table: "artifact",
                function: function!(),
                error: err,
            })?;

        //update map spaces
        diesel::update(
            map_spaces::table
                .filter(map_spaces::block_type_id.eq(block_id))
                // .filter(map_spaces::id.eq(map_space_id))
                .filter(map_spaces::map_id.eq(id_of_map)),
        )
        .set(map_spaces::block_type_id.eq(next_level_block_id))
        .execute(conn)?;

        Ok(())
    })
}

pub(crate) fn upgrade_mine(player_id: i32, conn: &mut PgConnection, block_id: i32) -> Result<()> {
    let user_artifacts = get_user_artifacts(player_id, conn)?;

    //check if the given block id is a mine
    //check if the given user has the block id
    let exists = select(exists(
        map_spaces::table
            .inner_join(map_layout::table)
            .inner_join(block_type::table.on(map_spaces::block_type_id.eq(block_type::id)))
            .filter(map_layout::player.eq(player_id))
            .filter(map_spaces::block_type_id.eq(block_id))
            .filter(block_type::category.eq(BlockCategory::Mine)),
    ))
    .get_result::<bool>(conn)?;

    if !exists {
        return Err(anyhow::anyhow!(
            "either Block is not a mine or the user does not have the block"
        ));
    }

    let joined_table = map_spaces::table
        .inner_join(map_layout::table)
        .inner_join(block_type::table.on(map_spaces::block_type_id.eq(block_type::id)))
        .filter(block_type::category.eq(BlockCategory::Mine))
        .inner_join(mine_type::table.on(block_type::mine_type.assume_not_null().eq(mine_type::id)))
        .filter(map_layout::player.eq(player_id))
        .filter(map_spaces::block_type_id.eq(block_id));

    let (cost, level, name, _): (i32, i32, String, i32) = joined_table
        .select((
            mine_type::cost,
            mine_type::level,
            mine_type::name,
            map_spaces::id,
        ))
        .first::<(i32, i32, String, i32)>(conn)
        .map_err(|err| DieselError {
            table: "mine_type",
            function: function!(),
            error: err,
        })?;

    let max_level: i64 = mine_type::table
        .filter(mine_type::name.eq(&name))
        .count()
        .get_result::<i64>(conn)
        .map_err(|err| DieselError {
            table: "mine_type",
            function: function!(),
            error: err,
        })?;

    if level >= max_level as i32 {
        return Err(anyhow::anyhow!("Defender is at max level"));
    };
    if cost > user_artifacts {
        return Err(anyhow::anyhow!("Not enough artifacts"));
    };

    let joined_table = block_type::table
        .inner_join(mine_type::table)
        .filter(block_type::category.eq(BlockCategory::Mine));

    let next_level_block_id: i32 = joined_table
        .filter(mine_type::name.eq(name))
        .filter(mine_type::level.eq(level + 1))
        .select(block_type::id)
        .first::<i32>(conn)
        .map_err(|err| DieselError {
            table: "mine_type",
            function: function!(),
            error: err,
        })?;

    let id_of_map = get_user_map_id(player_id, conn)?;
    let bank_block_type_id = get_block_id_of_bank(conn, &player_id)?;
    let bank_map_space_id = get_bank_map_space_id(conn, &id_of_map, &bank_block_type_id)?;
    let artifacts_in_bank = get_building_artifact_count(conn, &id_of_map, &bank_map_space_id)?;
    if artifacts_in_bank < cost {
        return Err(anyhow::anyhow!("Not enough artifacts in bank"));
    }
    conn.transaction(|conn| {
        let id_of_map = get_user_map_id(player_id, conn)?;

        diesel::update(user::table.filter(user::id.eq(player_id)))
            .set(user::artifacts.eq(user_artifacts - cost))
            .execute(conn)?;

        //update artifacts in bank
        diesel::update(artifact::table.filter(artifact::map_space_id.eq(bank_map_space_id)))
            .set(artifact::count.eq(artifact::count - cost))
            .execute(conn)
            .map_err(|err| DieselError {
                table: "artifact",
                function: function!(),
                error: err,
            })?;

        //update map spaces

        diesel::update(
            map_spaces::table
                .filter(map_spaces::block_type_id.eq(block_id))
                // .filter(map_spaces::id.eq(map_space_id))
                .filter(map_spaces::map_id.eq(id_of_map)),
        )
        .set(map_spaces::block_type_id.eq(next_level_block_id))
        .execute(conn)?;

        Ok(())
    })
}

pub(crate) fn upgrade_attacker(
    player_id: i32,
    conn: &mut PgConnection,
    attacker_id: i32,
) -> Result<()> {
    let user_artifacts = get_user_artifacts(player_id, conn)?;

    let joined_table = available_attackers::table
        .inner_join(attacker_type::table)
        .filter(available_attackers::user_id.eq(player_id))
        .filter(attacker_type::id.eq(attacker_id));

    let exists = select(exists(joined_table)).get_result::<bool>(conn)?;

    if !exists {
        return Err(anyhow::anyhow!("User does not have the attacker"));
    }

    let (cost, level, name): (i32, i32, String) = joined_table
        .filter(attacker_type::id.eq(attacker_id))
        .select((
            attacker_type::cost,
            attacker_type::level,
            attacker_type::name,
        ))
        .first::<(i32, i32, String)>(conn)
        .map_err(|err| DieselError {
            table: "attacker_type",
            function: function!(),
            error: err,
        })?;

    let max_level: i64 = attacker_type::table
        .filter(attacker_type::name.eq(&name))
        .count()
        .get_result::<i64>(conn)
        .map_err(|err| DieselError {
            table: "attacker_type",
            function: function!(),
            error: err,
        })?;

    if level >= max_level as i32 {
        return Err(anyhow::anyhow!("Attacker is at max level"));
    };
    if cost > user_artifacts {
        return Err(anyhow::anyhow!("Not enough artifacts"));
    };
    let id_of_map = get_user_map_id(player_id, conn)?;
    let bank_block_type_id = get_block_id_of_bank(conn, &player_id)?;
    let bank_map_space_id = get_bank_map_space_id(conn, &id_of_map, &bank_block_type_id)?;
    let artifacts_in_bank = get_building_artifact_count(conn, &id_of_map, &bank_map_space_id)?;
    if artifacts_in_bank < cost {
        return Err(anyhow::anyhow!("Not enough artifacts in bank"));
    }
    let next_level_attacker_id: i32 = attacker_type::table
        .filter(attacker_type::name.eq(name))
        .filter(attacker_type::level.eq(level + 1))
        .select(attacker_type::id)
        .first::<i32>(conn)
        .map_err(|err| DieselError {
            table: "attacker_type",
            function: function!(),
            error: err,
        })?;
    conn.transaction(|conn| {
        diesel::update(
            available_attackers::table
                .filter(available_attackers::attacker_type_id.eq(attacker_id))
                .filter(available_attackers::user_id.eq(player_id)),
        )
        .set(available_attackers::attacker_type_id.eq(next_level_attacker_id))
        .execute(conn)?;

        //update artifacts in bank
        diesel::update(artifact::table.filter(artifact::map_space_id.eq(bank_map_space_id)))
            .set(artifact::count.eq(artifact::count - cost))
            .execute(conn)
            .map_err(|err| DieselError {
                table: "artifact",
                function: function!(),
                error: err,
            })?;

        diesel::update(user::table.filter(user::id.eq(player_id)))
            .set(user::artifacts.eq(user_artifacts - cost))
            .execute(conn)?;

        Ok(())
    })
}

pub(crate) fn upgrade_emp(player_id: i32, conn: &mut PgConnection, emp_id: i32) -> Result<()> {
    let user_artifacts = get_user_artifacts(player_id, conn)?;

    let joined_table = available_emps::table
        .inner_join(emp_type::table)
        .filter(available_emps::user_id.eq(player_id))
        .filter(emp_type::id.eq(emp_id));
    let exists = select(exists(joined_table)).get_result::<bool>(conn)?;

    if !exists {
        return Err(anyhow::anyhow!("User does not have the emp"));
    }

    let (cost, level, name): (i32, i32, String) = joined_table
        .filter(emp_type::id.eq(emp_id))
        .select((emp_type::cost, emp_type::level, emp_type::name))
        .first::<(i32, i32, String)>(conn)
        .map_err(|err| DieselError {
            table: "emp_type",
            function: function!(),
            error: err,
        })?;

    let max_level: i64 = emp_type::table
        .filter(emp_type::name.eq(&name))
        .count()
        .get_result::<i64>(conn)
        .map_err(|err| DieselError {
            table: "emp_type",
            function: function!(),
            error: err,
        })?;

    if level >= max_level as i32 {
        return Err(anyhow::anyhow!("Emp is at max level"));
    };
    if cost > user_artifacts {
        return Err(anyhow::anyhow!("Not enough artifacts"));
    };

    let next_level_emp_id: i32 = emp_type::table
        .filter(emp_type::name.eq(name))
        .filter(emp_type::level.eq(level + 1))
        .select(emp_type::id)
        .first::<i32>(conn)
        .map_err(|err| DieselError {
            table: "emp_type",
            function: function!(),
            error: err,
        })?;

    let id_of_map = get_user_map_id(player_id, conn)?;
    let bank_block_type_id = get_block_id_of_bank(conn, &player_id)?;
    let bank_map_space_id = get_bank_map_space_id(conn, &id_of_map, &bank_block_type_id)?;
    let artifacts_in_bank = get_building_artifact_count(conn, &id_of_map, &bank_map_space_id)?;
    if artifacts_in_bank < cost {
        return Err(anyhow::anyhow!("Not enough artifacts in bank"));
    }
    conn.transaction(|conn| {
        diesel::update(
            available_emps::table
                .filter(available_emps::emp_type_id.eq(emp_id))
                .filter(available_emps::user_id.eq(player_id)),
        )
        .set(available_emps::emp_type_id.eq(next_level_emp_id))
        .execute(conn)?;

        //update artifacts in bank
        diesel::update(artifact::table.filter(artifact::map_space_id.eq(bank_map_space_id)))
            .set(artifact::count.eq(artifact::count - cost))
            .execute(conn)
            .map_err(|err| DieselError {
                table: "artifact",
                function: function!(),
                error: err,
            })?;

        diesel::update(user::table.filter(user::id.eq(player_id)))
            .set(user::artifacts.eq(user_artifacts - cost))
            .execute(conn)?;

        Ok(())
    })
}

fn run_transaction(
    conn: &mut PgConnection,
    block_id: i32,
    next_level_block_id: i32,
    player_id: i32,
    cost: i32,
    user_artifacts: i32,
    bank_map_space_id: i32,
    update_map_spaces: bool,
    map_space_id: i32,
) -> Result<(), anyhow::Error> {
    conn.transaction(|conn| {
        let id_of_map = get_user_map_id(player_id, conn)?;

        diesel::update(user::table.filter(user::id.eq(player_id)))
            .set(user::artifacts.eq(user_artifacts - cost))
            .execute(conn)?;

        //update artifacts in bank
        diesel::update(artifact::table.filter(artifact::map_space_id.eq(bank_map_space_id)))
            .set(artifact::count.eq(artifact::count - cost))
            .execute(conn)
            .map_err(|err| DieselError {
                table: "artifact",
                function: function!(),
                error: err,
            })?;

        //update map spaces
        if update_map_spaces {
            diesel::update(
                map_spaces::table
                    .filter(map_spaces::block_type_id.eq(block_id))
                    .filter(map_spaces::id.eq(map_space_id))
                    .filter(map_spaces::map_id.eq(id_of_map)),
            )
            .set(map_spaces::block_type_id.eq(next_level_block_id))
            .execute(conn)?;
        }

        Ok(())
    })
}

pub fn get_user_map_id(player_id: i32, conn: &mut PgConnection) -> Result<i32> {
    let id_of_map = map_layout::table
        .filter(map_layout::player.eq(player_id))
        .select(map_layout::id)
        .first::<i32>(conn)
        .map_err(|err| DieselError {
            table: "map_layout",
            function: function!(),
            error: err,
        })?;

    Ok(id_of_map)
}

pub fn get_user_artifacts(player_id: i32, conn: &mut PgConnection) -> Result<i32> {
    let artifacts = user::table
        .filter(user::id.eq(player_id))
        .select(user::artifacts)
        .first::<i32>(conn)
        .map_err(|err| DieselError {
            table: "user",
            function: function!(),
            error: err,
        })?;
    Ok(artifacts)
}

pub fn get_building_artifact_count(
    conn: &mut PgConnection,
    filtered_layout_id: &i32,
    given_map_space_id: &i32,
) -> Result<i32> {
    let building_artifact_count = map_spaces::table
        .inner_join(artifact::table)
        .filter(map_spaces::map_id.eq(filtered_layout_id)) //Eg:1
        .filter(map_spaces::id.eq(given_map_space_id))
        .select(artifact::count)
        .first::<i32>(conn)
        .unwrap_or(-1);
    Ok(building_artifact_count)
}

pub fn get_block_id_of_bank(conn: &mut PgConnection, player: &i32) -> Result<i32> {
    let bank_block_type_id = map_spaces::table
        .inner_join(map_layout::table)
        .filter(map_layout::player.eq(player))
        .inner_join(block_type::table.on(map_spaces::block_type_id.eq(block_type::id)))
        .filter(block_type::category.eq(BlockCategory::Building))
        .inner_join(building_type::table.on(building_type::id.eq(block_type::building_type)))
        .filter(building_type::name.like(BANK_BUILDING_NAME))
        .select(block_type::id)
        .first::<i32>(conn)
        .map_err(|err| DieselError {
            table: "block_type",
            function: function!(),
            error: err,
        })?;
    Ok(bank_block_type_id)
}

pub fn get_bank_map_space_id(
    conn: &mut PgConnection,
    filtered_layout_id: &i32,
    bank_block_type_id: &i32,
) -> Result<i32> {
    let fetched_bank_map_space_id = map_spaces::table
        .filter(map_spaces::map_id.eq(filtered_layout_id))
        .filter(map_spaces::block_type_id.eq(bank_block_type_id))
        .select(map_spaces::id)
        .first::<i32>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?;
    Ok(fetched_bank_map_space_id)
}

pub fn get_building_map_space_id(
    conn: &mut PgConnection,
    filtered_layout_id: &i32,
    block_id: &i32,
) -> Result<i32> {
    let fetched_building_map_space_id = map_spaces::table
        .filter(map_spaces::map_id.eq(filtered_layout_id))
        .filter(map_spaces::block_type_id.eq(block_id))
        .select(map_spaces::id)
        .first::<i32>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?;
    Ok(fetched_building_map_space_id)
}

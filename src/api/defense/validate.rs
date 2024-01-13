/// Functions to check if a base layout is valid
use super::MapSpacesEntry;
use crate::{api::error::BaseInvalidError, constants::*, models::*};
use petgraph::{self, algo::tarjan_scc, prelude::*, Graph};
use std::collections::{HashMap, HashSet};

//checks overlaps of blocks and also within map size
pub fn is_valid_update_layout(
    map_spaces: &[MapSpacesEntry],
    blocks: &HashMap<i32, BlockType>,
    buildings: &[BuildingType],
) -> Result<(), BaseInvalidError> {
    let mut occupied_positions: HashSet<(i32, i32)> = HashSet::new();
    let mut road_positions: HashSet<(i32, i32)> = HashSet::new();
    let buildings: HashMap<i32, BuildingType> = buildings
        .iter()
        .map(|building| (building.id, building.clone()))
        .collect();
    for map_space in map_spaces {
        let block_type = map_space.block_type_id;

        if !blocks.contains_key(&block_type) {
            return Err(BaseInvalidError::InvalidBuildingType(block_type));
        }
        let block = blocks.get(&block_type).unwrap();

        let building_type = block.building_type;
        if !buildings.contains_key(&building_type) {
            return Err(BaseInvalidError::InvalidBlockType(building_type));
        }

        let building: &BuildingType = buildings.get(&building_type).unwrap();
        let (x, y, width, height) = (
            map_space.x_coordinate,
            map_space.y_coordinate,
            building.width,
            building.height,
        );
        /*if x == -1 && building_type != ROAD_ID {
            return Err(BaseInvalidError::InvalidRotation(
                buildings[&building_type].name.clone(),
                map_space.rotation,
            ));
        }*/

        for i in 0..width {
            for j in 0..height {
                if (0..MAP_SIZE as i32).contains(&(x + i))
                    && (0..MAP_SIZE as i32).contains(&(y + j))
                {
                    if occupied_positions.contains(&(x + i, y + j)) {
                        return Err(BaseInvalidError::OverlappingBlocks);
                    }
                    occupied_positions.insert((x + i, y + j));
                } else {
                    return Err(BaseInvalidError::BlockOutsideMap);
                }
            }
        }
        if building_type == ROAD_ID {
            road_positions.insert((map_space.x_coordinate, map_space.y_coordinate));
        }
    }
    if is_road_rounded(&road_positions) {
        return Err(BaseInvalidError::RoundRoad);
    }

    Ok(())
}

// checks every 4x4 tiles has completely Roads
pub fn is_road_rounded(road_positions: &HashSet<(i32, i32)>) -> bool {
    let directions = [(-1, 0), (-1, -1), (0, -1)];
    for i in 1..MAP_SIZE as i32 {
        for j in 1..MAP_SIZE as i32 {
            if road_positions.contains(&(i, j)) {
                let mut road_count = 1;
                for (x, y) in directions.iter() {
                    if road_positions.contains(&(i + x, j + y)) {
                        road_count += 1;
                    }
                }
                if road_count == 4 {
                    return true;
                }
            }
        }
    }
    false
}

// checks if no of buildings are within level constraints and if the city is connected
pub fn is_valid_save_layout(
    map_spaces: &[MapSpacesEntry],
    block_constraints: &mut HashMap<i32, i32>,
    blocks: &HashMap<i32, BlockType>,
    buildings: &[BuildingType],
) -> Result<(), BaseInvalidError> {
    is_valid_update_layout(map_spaces, blocks, buildings)?;

    let mut graph: Graph<(), (), Directed> = Graph::new();
    let mut road_graph: Graph<(), (), Directed> = Graph::new();
    let mut map_grid: HashMap<(i32, i32), NodeIndex> = HashMap::new();
    let mut road_grid: HashMap<(i32, i32), NodeIndex> = HashMap::new();
    let mut node_to_coords: HashMap<NodeIndex, (i32, i32)> = HashMap::new();
    let mut road_node_to_coords: HashMap<NodeIndex, (i32, i32)> = HashMap::new();

    let buildings: HashMap<i32, BuildingType> = buildings
        .iter()
        .map(|building| (building.id, building.clone()))
        .collect();

    for map_space in map_spaces {
        let MapSpacesEntry {
            block_type_id,
            x_coordinate,
            y_coordinate,
            ..
        } = *map_space;

        let block = blocks.get(&block_type_id).unwrap();

        let building_type = block.building_type;

        // check for level constraints
        if let Some(block_constraint) = block_constraints.get_mut(&block_type_id) {
            if *block_constraint > 0 {
                *block_constraint -= 1;
            } else {
                return Err(BaseInvalidError::BlockCountExceeded(block_type_id));
            }
        }

        // add roads and entrances to graph
        let new_node = graph.add_node(());
        if building_type == ROAD_ID {
            let road_node = road_graph.add_node(());
            map_grid.insert((x_coordinate, y_coordinate), new_node);
            road_grid.insert((x_coordinate, y_coordinate), road_node);
            node_to_coords.insert(new_node, (x_coordinate, y_coordinate));
            road_node_to_coords.insert(road_node, (x_coordinate, y_coordinate));
        } else {
            let entrance = (map_space.x_coordinate, map_space.y_coordinate);
            node_to_coords.insert(new_node, entrance);
            map_grid.insert(entrance, new_node);
        }
    }

    //checks if every block of each type is used
    for block_constraint in block_constraints {
        if blocks.get(block_constraint.0).unwrap().category == BlockCategory::Building
            && *block_constraint.1 != 0
        {
            return Err(BaseInvalidError::BlocksUnused(
                buildings[&blocks.get(block_constraint.0).unwrap().building_type]
                    .name
                    .clone(),
            ));
        }
    }

    for (coordinates, node_index) in &map_grid {
        let (x, y) = *coordinates;
        if map_grid.contains_key(&(x + 1, y)) {
            let node_index_right = map_grid.get(&(x + 1, y)).unwrap();
            graph.add_edge(*node_index, *node_index_right, ());
            graph.add_edge(*node_index_right, *node_index, ());
        }
        if map_grid.contains_key(&(x, y + 1)) {
            let node_index_down = map_grid.get(&(x, y + 1)).unwrap();
            graph.add_edge(*node_index, *node_index_down, ());
            graph.add_edge(*node_index_down, *node_index, ());
        }
    }

    for (coordinates, node_index) in &road_grid {
        let (x, y) = *coordinates;
        if road_grid.contains_key(&(x + 1, y)) {
            let node_index_right = road_grid.get(&(x + 1, y)).unwrap();
            road_graph.add_edge(*node_index, *node_index_right, ());
            road_graph.add_edge(*node_index_right, *node_index, ());
        }
        if road_grid.contains_key(&(x, y + 1)) {
            let node_index_down = road_grid.get(&(x, y + 1)).unwrap();
            road_graph.add_edge(*node_index, *node_index_down, ());
            road_graph.add_edge(*node_index_down, *node_index, ());
        }
    }

    let road_connected_components = tarjan_scc(&road_graph);

    if road_connected_components.len() != 1 {
        let first_component_node = road_connected_components[0][0];
        let second_component_node = road_connected_components[1][0];
        let first_node_coords = road_node_to_coords[&first_component_node];
        let second_node_coords = road_node_to_coords[&second_component_node];
        return Err(BaseInvalidError::NotConnected(format!(
            "Road from ({}, {}) to ({}, {}) is not connected",
            first_node_coords.0, first_node_coords.1, second_node_coords.0, second_node_coords.1
        )));
    }

    let connected_components = tarjan_scc(&graph);

    if connected_components.len() == 1 {
        Ok(())
    } else {
        let first_component_node = connected_components[0][0];
        let second_component_node = connected_components[1][0];
        let first_node_coords = node_to_coords[&first_component_node];
        let second_node_coords = node_to_coords[&second_component_node];
        Err(BaseInvalidError::NotConnected(format!(
            "no path from ({}, {}) to ({}, {})",
            first_node_coords.0, first_node_coords.1, second_node_coords.0, second_node_coords.1
        )))
    }
}

use crate::constants::*;
use crate::error::DieselError;
use crate::models::*;
use crate::schema::{block_type, building_type, map_spaces, shortest_path};
use crate::util::function;
use anyhow::Result;
use array2d::Array2D;
use diesel::prelude::*;
use diesel::RunQueryDsl;
use diesel::{PgConnection, QueryDsl};
use petgraph::algo::astar;
use petgraph::Graph;
use std::collections::HashMap;

const NO_BLOCK: i32 = -1;

// function to get absolute coordinates
fn get_absolute_coordinates(
    rotation: i32,
    x_coordinate: i32,
    y_coordinate: i32,
    entrance_x: i32,
    entrance_y: i32,
) -> (i32, i32) {
    match rotation {
        90 => (x_coordinate - entrance_y, y_coordinate + entrance_x),
        180 => (x_coordinate - entrance_x, y_coordinate - entrance_y),
        270 => (x_coordinate + entrance_y, y_coordinate - entrance_x),
        _ => (x_coordinate + entrance_x, y_coordinate + entrance_y),
    }
}

fn get_blocks(conn: &mut PgConnection) -> Result<HashMap<i32, BlockType>> {
    Ok(building_type::table
        .inner_join(block_type::table)
        .select((building_type::id, block_type::all_columns))
        .load::<(i32, BlockType)>(conn)
        .map_err(|err| DieselError {
            table: "buildin_type",
            function: function!(),
            error: err,
        })?
        .into_iter()
        .map(|(id, block)| (id, block))
        .collect())
}

fn get_block_id(building_id: &i32, building_map: &HashMap<i32, BlockType>) -> i32 {
    building_map[building_id].id
}

//running shortest path simulation
pub fn run_shortest_paths(
    conn: &mut PgConnection,
    input_map_layout_id: i32,
    blocks_list: &Vec<BlockType>,
) -> Result<()> {
    // reading map_spaces
    let mapspaces_list = map_spaces::table
        .filter(map_spaces::map_id.eq(input_map_layout_id))
        .load::<MapSpaces>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?;

    let buildings_block_map = get_blocks(conn)?;

    // initialising map for types of blocks
    let mut map = HashMap::new();

    // initialising maps for index to nodes and nodes to index
    let mut node_to_index = HashMap::new();
    let mut index_to_node = HashMap::new();

    // filling block types in map
    for p in blocks_list {
        map.insert(p.id, (p.width, p.height, p.entrance_x, p.entrance_y));
    }

    // initialising 2d array and petgraph Graph
    let mut graph_2d = Array2D::filled_with(NO_BLOCK, MAP_SIZE, MAP_SIZE);
    let mut graph = Graph::<usize, usize>::new();

    // Initialising nodes, filling 2d array and the node_to_index and index_to_node maps
    for i in &mapspaces_list {
        let single_node = graph.add_node(0);
        let (absolute_entrance_x, absolute_entrance_y) = get_absolute_coordinates(
            i.rotation,
            i.x_coordinate,
            i.y_coordinate,
            map[&get_block_id(&i.building_type, &buildings_block_map)].2,
            map[&get_block_id(&i.building_type, &buildings_block_map)].3,
        );
        graph_2d
            .set(
                absolute_entrance_y as usize,
                absolute_entrance_x as usize,
                get_block_id(&i.building_type, &buildings_block_map),
            )
            .unwrap();
        node_to_index.insert(
            single_node,
            (absolute_entrance_y as usize) * MAP_SIZE + (absolute_entrance_x as usize),
        );
        index_to_node.insert(
            (absolute_entrance_y as usize) * MAP_SIZE + (absolute_entrance_x as usize),
            single_node,
        );
    }

    // adding edges to graph from 2d array (2 nearby nodes)
    for i in 0..MAP_SIZE {
        for j in 0..MAP_SIZE {
            if graph_2d[(i, j)] != NO_BLOCK {
                // i,j->i+1,j
                if i + 1 < MAP_SIZE && graph_2d[(i + 1, j)] != NO_BLOCK {
                    graph.extend_with_edges(&[(
                        index_to_node[&(i * MAP_SIZE + j)],
                        index_to_node[&((i + 1) * MAP_SIZE + j)],
                        1,
                    )]);
                    graph.extend_with_edges(&[(
                        index_to_node[&((i + 1) * MAP_SIZE + j)],
                        index_to_node[&(i * MAP_SIZE + j)],
                        1,
                    )]);
                }
                //i,j->i,j+1
                if j + 1 < MAP_SIZE && graph_2d[(i, j + 1)] != NO_BLOCK {
                    graph.extend_with_edges(&[(
                        index_to_node[&(i * MAP_SIZE + j)],
                        index_to_node[&(i * MAP_SIZE + (j + 1))],
                        1,
                    )]);
                    graph.extend_with_edges(&[(
                        index_to_node[&(i * MAP_SIZE + (j + 1))],
                        index_to_node[&(i * MAP_SIZE + j)],
                        1,
                    )]);
                }
            }
        }
    }

    // Astar algorithm between EVERY PAIR of nodes
    let mut shortest_paths = vec![];
    for i in &mapspaces_list {
        for j in &mapspaces_list {
            if j.building_type != ROAD_ID {
                let (start_absolute_entrance_x, start_absolute_entrance_y) =
                    get_absolute_coordinates(
                        i.rotation,
                        i.x_coordinate,
                        i.y_coordinate,
                        map[&get_block_id(&i.building_type, &buildings_block_map)].2,
                        map[&get_block_id(&i.building_type, &buildings_block_map)].3,
                    );
                let (dest_absolute_entrance_x, dest_absolute_entrance_y) = get_absolute_coordinates(
                    j.rotation,
                    j.x_coordinate,
                    j.y_coordinate,
                    map[&get_block_id(&j.building_type, &buildings_block_map)].2,
                    map[&get_block_id(&j.building_type, &buildings_block_map)].3,
                );
                let start_node = index_to_node[&((start_absolute_entrance_y as usize) * MAP_SIZE
                    + (start_absolute_entrance_x as usize))];
                let dest_node = index_to_node[&((dest_absolute_entrance_y as usize) * MAP_SIZE
                    + (dest_absolute_entrance_x as usize))];
                let path = astar(
                    &graph,
                    start_node,
                    |finish| finish == dest_node,
                    |e| *e.weight(),
                    |_| 0,
                );

                match path {
                    Some(p) => {
                        let mut path_string = String::new();

                        // Building the path string
                        for i in p.1 {
                            path_string.push('(');
                            path_string.push_str(&(node_to_index[&i] % MAP_SIZE).to_string());
                            path_string.push(',');
                            path_string.push_str(
                                &(node_to_index[&i] as i32 / MAP_SIZE as i32).to_string(),
                            );
                            path_string.push(')');
                        }

                        let new_shortest_path_entry = NewShortestPath {
                            base_id: input_map_layout_id,
                            source_x: node_to_index[&start_node] as i32 % MAP_SIZE as i32,
                            source_y: node_to_index[&start_node] as i32 / MAP_SIZE as i32,
                            dest_x: node_to_index[&dest_node] as i32 % MAP_SIZE as i32,
                            dest_y: node_to_index[&dest_node] as i32 / MAP_SIZE as i32,
                            pathlist: path_string,
                        };

                        shortest_paths.push(new_shortest_path_entry);
                    }
                    None => println!(
                        "No path found between {} and {}",
                        node_to_index[&start_node], node_to_index[&dest_node]
                    ),
                };
            }
        }
    }

    // Writing entries to shortest_path table
    let chunks: Vec<&[NewShortestPath]> = shortest_paths.chunks(1000).collect();
    for chunk in chunks {
        diesel::insert_into(shortest_path::table)
            .values(chunk)
            .execute(conn)
            .map_err(|err| DieselError {
                table: "shortest_path",
                function: function!(),
                error: err,
            })?;
    }
    Ok(())
}

use crate::constants::*;
use crate::error::DieselError;
use crate::schema::{block_type, map_spaces};
use crate::util::function;
use crate::validator::util::{Coords, SourceDestXY};
use anyhow::Result;
use array2d::Array2D;
use diesel::prelude::*;
use diesel::RunQueryDsl;
use diesel::{PgConnection, QueryDsl};
use std::collections::{HashMap, HashSet, VecDeque};

const NO_BLOCK: i32 = -1;

//running shortest path simulation
pub fn run_shortest_paths(
    conn: &mut PgConnection,
    input_map_layout_id: i32,
) -> Result<HashMap<SourceDestXY, Coords>> {
    let roads_list: Vec<(i32, i32)> = map_spaces::table
        .inner_join(block_type::table)
        .filter(map_spaces::map_id.eq(input_map_layout_id))
        .filter(block_type::building_type.eq(ROAD_ID))
        .select((map_spaces::x_coordinate, map_spaces::y_coordinate))
        .load::<(i32, i32)>(conn)
        .map_err(|err| DieselError {
            table: "map_spaces",
            function: function!(),
            error: err,
        })?
        .to_vec();

    let mut graph_2d = Array2D::filled_with(NO_BLOCK, MAP_SIZE, MAP_SIZE);

    for road in &roads_list {
        let (road_x, road_y) = (road.0, road.1);
        graph_2d
            .set(road_x as usize, road_y as usize, ROAD_ID)
            .unwrap();
    }

    let mut adjacency_list: HashMap<(i32, i32), Vec<(i32, i32)>> = HashMap::new();

    for road in &roads_list {
        let (road_x, road_y) = (road.0, road.1);
        let mut neighbors = Vec::new();

        for &(dx, dy) in &[(1, 0), (0, 1), (-1, 0), (0, -1)] {
            let (nx, ny) = (road_x + dx, road_y + dy);
            if nx >= 0
                && ny >= 0
                && (nx as usize) < MAP_SIZE
                && (ny as usize) < MAP_SIZE
                && graph_2d[(nx as usize, ny as usize)] == ROAD_ID
            {
                neighbors.push((nx, ny));
            }
        }

        adjacency_list.insert((road_x, road_y), neighbors);
    }

    let mut shortest_paths: HashMap<SourceDestXY, Coords> = HashMap::new();

    for (start_x, start_y) in &roads_list {
        let start_node = (*start_x, *start_y);
        let mut visited: HashSet<(i32, i32)> = HashSet::new();
        let mut queue: VecDeque<((i32, i32), (i32, i32))> = VecDeque::new();

        visited.insert(start_node);
        queue.push_back((start_node, start_node));

        while let Some((current_node, parent_node)) = queue.pop_front() {
            for neighbor in &adjacency_list[&current_node] {
                if visited.insert(*neighbor) {
                    let next_hop = if start_node == parent_node {
                        *neighbor
                    } else {
                        parent_node
                    };

                    queue.push_back((*neighbor, next_hop));

                    shortest_paths.insert(
                        SourceDestXY {
                            source_x: *start_x,
                            source_y: *start_y,
                            dest_x: neighbor.0,
                            dest_y: neighbor.1,
                        },
                        Coords {
                            x: next_hop.0,
                            y: next_hop.1,
                        },
                    );
                }
            }
        }
    }

    // // Writing entries to shortest_path table
    // let chunks: Vec<&[NewShortestPath]> = shortest_paths.chunks(1000).collect();
    // for chunk in chunks {
    //     diesel::insert_into(shortest_path::table)
    //         .values(chunk)
    //         .execute(conn)
    //         .map_err(|err| DieselError {
    //             table: "shortest_path",
    //             function: function!(),
    //             error: err,
    //         })?;
    // }

    Ok(shortest_paths)
}

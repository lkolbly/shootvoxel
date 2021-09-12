use rand::prelude::*;
use std::collections::{HashMap, HashSet};
use std::io::Write;

struct DungeonSpecification {
    rooms_wide: u16,
    rooms_deep: u16,
    rooms_tall: u16,
}

impl DungeonSpecification {
    fn num_rooms(&self) -> usize {
        self.rooms_deep as usize * self.rooms_tall as usize * self.rooms_wide as usize
    }

    fn positions(&self) -> Vec<Position> {
        let mut positions = vec![];
        for x in 0..self.rooms_wide {
            for y in 0..self.rooms_tall {
                for z in 0..self.rooms_deep {
                    positions.push(Position { x, y, z });
                }
            }
        }
        positions
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Direction {
    Up,
    Down,
    North,
    South,
    East,
    West,
}

const DIRECTIONS: [Direction; 6] = [
    Direction::Up,
    Direction::Down,
    Direction::North,
    Direction::South,
    Direction::East,
    Direction::West,
];

const HALF_DIRECTIONS: [Direction; 3] = [Direction::Up, Direction::North, Direction::East];

impl Direction {
    fn opposite(&self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        }
    }

    fn index(&self) -> usize {
        match self {
            Direction::Up => 0,
            Direction::Down => 1,
            Direction::North => 2,
            Direction::South => 3,
            Direction::East => 4,
            Direction::West => 5,
        }
    }

    fn from_index(idx: usize) -> Self {
        if idx == 0 {
            Direction::Up
        } else if idx == 1 {
            Direction::Down
        } else if idx == 2 {
            Direction::North
        } else if idx == 3 {
            Direction::South
        } else if idx == 4 {
            Direction::East
        } else if idx == 5 {
            Direction::West
        } else {
            panic!("Invalid direction index {}!", idx);
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Position {
    x: u16,
    y: u16,
    z: u16,
}

impl Position {
    fn index(&self, spec: &DungeonSpecification) -> usize {
        self.x as usize * spec.rooms_deep as usize * spec.rooms_tall as usize
            + self.y as usize * spec.rooms_deep as usize
            + self.z as usize
    }

    fn in_direction(&self, direction: Direction, spec: &DungeonSpecification) -> Option<Position> {
        match direction {
            Direction::Up => {
                if self.y + 1 < spec.rooms_tall {
                    Some(Self {
                        x: self.x,
                        y: self.y + 1,
                        z: self.z,
                    })
                } else {
                    None
                }
            }
            Direction::Down => {
                if self.y > 0 {
                    Some(Self {
                        x: self.x,
                        y: self.y - 1,
                        z: self.z,
                    })
                } else {
                    None
                }
            }
            Direction::North => {
                if self.z + 1 < spec.rooms_deep {
                    Some(Self {
                        x: self.x,
                        y: self.y,
                        z: self.z + 1,
                    })
                } else {
                    None
                }
            }
            Direction::South => {
                if self.z > 0 {
                    Some(Self {
                        x: self.x,
                        y: self.y,
                        z: self.z - 1,
                    })
                } else {
                    None
                }
            }
            Direction::East => {
                if self.x + 1 < spec.rooms_wide {
                    Some(Self {
                        x: self.x + 1,
                        y: self.y,
                        z: self.z,
                    })
                } else {
                    None
                }
            }
            Direction::West => {
                if self.x > 0 {
                    Some(Self {
                        x: self.x - 1,
                        y: self.y,
                        z: self.z,
                    })
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Clone)]
struct Room {
    connectivity: [bool; 6],
}

fn build_connectivity(spec: &DungeonSpecification, rooms: &Vec<Room>) -> Vec<usize> {
    let mut areas = vec![];
    for pos in spec.positions().iter() {
        areas.push(pos.index(spec));
    }

    // Flood fill each position. Note that spec.positions() is in index order, so we
    // are guaranteed to fill areas in ascending order (and so, if we try to flood fill
    // from a cell that's been touched by a lower number, we can skip it)
    for pos in spec.positions().iter() {
        let mut stack = vec![];
        stack.push(*pos);
        let current_area = pos.index(spec);
        if current_area > areas[pos.index(spec)] {
            continue;
        }
        while stack.len() > 0 {
            let pos = stack.pop().unwrap();
            areas[pos.index(spec)] = current_area;
            for dir in DIRECTIONS.iter() {
                if rooms[pos.index(spec)].connectivity[dir.index()] {
                    let new_room = pos.in_direction(*dir, spec).unwrap();
                    if areas[new_room.index(spec)] > current_area {
                        stack.push(new_room);
                    }
                }
            }
        }
    }
    areas
}

fn is_connected(spec: &DungeonSpecification, rooms: &Vec<Room>) -> bool {
    let areas = build_connectivity(spec, rooms);
    areas.iter().all(|area| *area == areas[0])
}

struct Voxel {
    x: u16,
    y: u16,
    z: u16,
}

fn fill(p1: Position, p2: Position) -> Vec<Voxel> {
    let mut voxels = vec![];
    for x in p1.x..p2.x + 1 {
        for y in p1.y..p2.y + 1 {
            for z in p1.z..p2.z + 1 {
                voxels.push(Voxel { x, y, z });
            }
        }
    }
    voxels
}

fn wall(dir: Direction) -> Vec<Voxel> {
    match dir {
        Direction::Up | Direction::Down => unimplemented!(),
        Direction::North => fill(
            Position { x: 0, y: 0, z: 11 },
            Position {
                x: 11,
                y: 10,
                z: 11,
            },
        ),
        Direction::South => fill(
            Position { x: 0, y: 0, z: 0 },
            Position { x: 11, y: 10, z: 0 },
        ),
        Direction::East => fill(
            Position { x: 11, y: 0, z: 0 },
            Position {
                x: 11,
                y: 10,
                z: 11,
            },
        ),
        Direction::West => fill(
            Position { x: 0, y: 0, z: 0 },
            Position { x: 0, y: 10, z: 11 },
        ),
    }
}

fn translate(voxels: Vec<Voxel>, offset: Position) -> Vec<Voxel> {
    voxels
        .iter()
        .map(|voxel| Voxel {
            x: voxel.x + offset.x,
            y: voxel.y + offset.y,
            z: voxel.z + offset.z,
        })
        .collect()
}

/// Start with a 3D array of rooms. Then, randomly remove walls until the map is fully
/// connected.
fn generate_dungeon(spec: DungeonSpecification) -> Vec<Voxel> {
    let mut rooms =
        vec![
            Room {
                connectivity: [false; 6]
            };
            spec.rooms_deep as usize * spec.rooms_tall as usize * spec.rooms_wide as usize
        ];

    let mut all_walls = vec![];
    for x in 0..spec.rooms_wide {
        for y in 0..spec.rooms_tall {
            for z in 0..spec.rooms_deep {
                let pos = Position { x, y, z };
                for dir in HALF_DIRECTIONS.iter() {
                    if let Some(_) = pos.in_direction(*dir, &spec) {
                        all_walls.push((pos, dir));
                    }
                }
            }
        }
    }

    let mut rng = rand::thread_rng();
    while !is_connected(&spec, &rooms) {
        // Remove a random wall
        let maze_algo: f32 = rng.gen();
        if maze_algo < 0.1 {
            // Remove a wall which is separating two spaces
            let areas = build_connectivity(&spec, &rooms);
            let separating_walls: Vec<_> = all_walls
                .iter()
                .enumerate()
                .filter_map(|(idx, (pos, dir))| {
                    let other_pos = match pos.in_direction(**dir, &spec) {
                        Some(x) => x,
                        None => return None,
                    };
                    if areas[pos.index(&spec)] != areas[other_pos.index(&spec)] {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect();
            let idx: usize = rng.gen::<usize>() % separating_walls.len();
            let idx = separating_walls[idx];

            let (pos, dir) = all_walls.remove(idx);
            rooms[pos.index(&spec)].connectivity[dir.index()] = true;
            //println!("Removing {:?} {:?}", pos, dir);
            rooms[pos.in_direction(*dir, &spec).unwrap().index(&spec)].connectivity
                [dir.opposite().index()] = true;
        } else {
            // Remove a random wall
            let idx: usize = rng.gen::<usize>() % all_walls.len();
            let (pos, dir) = all_walls.remove(idx);
            rooms[pos.index(&spec)].connectivity[dir.index()] = true;
            //println!("{:?} {:?}", pos, dir);
            rooms[pos.in_direction(*dir, &spec).unwrap().index(&spec)].connectivity
                [dir.opposite().index()] = true;
        }
    }

    // Generate voxels based on the remaining rooms
    // Walls are 2 voxels thick, rooms are 10 wide and 10 tall.
    let mut voxels = vec![];
    for x in 0..spec.rooms_wide {
        for y in 0..spec.rooms_tall {
            for z in 0..spec.rooms_deep {
                let pos = Position { x, y, z };
                for dir in DIRECTIONS.iter() {
                    if *dir == Direction::Up || *dir == Direction::Down {
                        continue;
                    }
                    if !rooms[pos.index(&spec)].connectivity[dir.index()] {
                        let wall = wall(*dir);
                        let mut wall = translate(
                            wall,
                            Position {
                                x: x * 12,
                                y: y * 12,
                                z: z * 12,
                            },
                        );
                        voxels.append(&mut wall);
                    }
                }
            }
        }
    }
    voxels
}

fn main() {
    let spec = DungeonSpecification {
        rooms_deep: 15,
        rooms_wide: 15,
        rooms_tall: 1,
    };
    let voxels = generate_dungeon(spec);
    println!("Generated {} voxels", voxels.len());
    let mut f = std::fs::File::create("map.txt").unwrap();
    for voxel in voxels.iter() {
        // Swap Z and Y for Goxel
        f.write_all(format!("{} {} {} ffffffff\n", voxel.x, voxel.z, voxel.y).as_bytes())
            .unwrap();
    }
}

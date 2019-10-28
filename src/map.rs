use tcod::Color;
use std::cmp;
use rand::Rng;
use crate::object::Object;

pub const MAP_WIDTH: i32 = 80;
pub const MAP_HEIGHT: i32 = 45;

pub const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
pub const COLOR_DARK_GROUND: Color = Color {
    r: 50,
    g: 50,
    b: 150,
};

// Dungeon generator params
pub const ROOM_MAX_SIZE: i32 = 10;
pub const ROOM_MIN_SIZE: i32 = 6;
pub const MAX_ROOMS: i32 = 30;

#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub blocked: bool,
    pub block_sight: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false,
            block_sight: false,
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocked: true,
            block_sight: true,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect {
            x1: x,
            y1: y,
            x2: x + w,
            y2: y + h,
        }
    }

    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }

    pub fn intersects_with(&self, other: &Rect) -> bool {
        // This will return true if the rect intersects with the `other`
        (self.x1 <= other.x2)
            && (self.x2 >= other.x1)
            && (self.y1 <= other.y2)
            && (self.y2 >= other.y1)
    }
}

fn create_room(room: Rect, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

fn create_horiz_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

fn create_vert_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

pub type Map = Vec<Vec<Tile>>;

pub struct Game {
    pub map: Map,
}

impl Game {
    pub fn new(player: &mut Object) -> Self {
        Game {
            map: Game::make_map(player)
        }
    }

    fn make_map(player: &mut Object) -> Map {
        let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];
        let mut rooms = vec![];

        for _ in 0..MAX_ROOMS {
            // Get a random width/height
            let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
            let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
            // Get a random position while staying in the map
            let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
            let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

            let new_room = Rect::new(x, y, w, h);

            // Run through the other rooms and check for intersection
            let failed = rooms
                .iter()
                .any(|other_room| new_room.intersects_with(other_room));

            if !failed {
                create_room(new_room, &mut map);

                // Center coordinates of the new room
                let (new_x, new_y) = new_room.center();

                if rooms.is_empty() {
                    // Set the player here
                    player.x = new_x;
                    player.y = new_y;
                } else {
                    // Connect all rooms that aren't the first room with tunnels

                    // Get the previous room's center coords
                    let (prev_x, prev_y) = rooms[rooms.len() - 1].center();

                    // Flip a `coin` (random true/false)
                    if rand::random() {
                        // Move horiz then vert
                        create_horiz_tunnel(prev_x, new_x, prev_y, &mut map);
                        create_vert_tunnel(prev_y, new_y, prev_x, &mut map);
                    } else {
                        // Move vert then horiz
                        create_vert_tunnel(prev_y, new_y, prev_x, &mut map);
                        create_horiz_tunnel(prev_x, new_x, prev_y, &mut map);
                    }
                }

                rooms.push(new_room);
            }
        }

        map
    }
}
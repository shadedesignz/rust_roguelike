use tcod::{Color, Console, BackgroundFlag, TextAlignment};
use std::cmp;
use rand::Rng;
use crate::object::{Object, place_objects};
use tcod::map::FovAlgorithm;
use crate::map::TunnelDirection::{Horizontal, Vertical};
use crate::log::Messages;
use tcod::console::{Root, Offscreen, blit};
use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};
use tcod::colors::WHITE;

pub const PLAYER: usize = 0;

pub const MAP_WIDTH: i32 = 80;
pub const MAP_HEIGHT: i32 = 43;

pub const FOV_ALGORITHM: FovAlgorithm = FovAlgorithm::Basic;
pub const FOV_LIGHT_WALLS: bool = true;
pub const TORCH_RADIUS: i32 = 10;

pub const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
pub const COLOR_LIGHT_WALL: Color = Color {
    r: 130,
    g: 110,
    b: 50,
};
pub const COLOR_DARK_GROUND: Color = Color {
    r: 50,
    g: 50,
    b: 150,
};
pub const COLOR_LIGHT_GROUND: Color = Color {
    r: 200,
    g: 180,
    b: 50,
};

// Dungeon generator params
pub const ROOM_MAX_SIZE: i32 = 10;
pub const ROOM_MIN_SIZE: i32 = 6;
pub const MAX_ROOMS: i32 = 30;

#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub blocked: bool,
    pub block_sight: bool,
    pub explored: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false,
            block_sight: false,
            explored: false,
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocked: true,
            block_sight: true,
            explored: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
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

enum TunnelDirection {
    Horizontal,
    Vertical,
}

fn create_tunnel(a1: i32, a2: i32, b: usize, dir: TunnelDirection, map: &mut Map) {
    let low = cmp::min(a1, a2);
    let high = cmp::max(a1, a2) + 1;
    for a in low..high {
        match dir {
            Horizontal => { map[a as usize][b] = Tile::empty() },
            Vertical => { map[b][a as usize] = Tile::empty() },
        }
    }
}

fn create_horiz_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    create_tunnel(x1, x2, y as usize, Horizontal, map);
}

fn create_vert_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    create_tunnel(y1, y2, x as usize, Vertical, map);
}

pub type Map = Vec<Vec<Tile>>;

pub fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32, root: &mut Root) -> Option<usize> {
    assert!(
        options.len() <= 26,
        "Cannot have a menu with more than 26 options."
    );

    // Calculate total height for the header (after auto-wrap) and one line per option
    let header_height = root.get_height_rect(0, 0, width, SCREEN_HEIGHT, header);
    let height = options.len() as i32 + header_height;

    // Create an off-screen console that represents the menu's window
    let mut window = Offscreen::new(width, height);

    // Print the header, with auto-wrap
    window.set_default_foreground(WHITE);
    window.print_rect_ex(
        0,
        0,
        width,
        height,
        BackgroundFlag::None,
        TextAlignment::Left,
        header,
    );

    // Print all the options
    for (index, option_text) in options.iter().enumerate() {
        let menu_letter = (b'a' + index as u8) as char;
        let text = format!("({}) {}", menu_letter, option_text.as_ref());
        window.print_ex(
            0,
            header_height + index as i32,
            BackgroundFlag::None,
            TextAlignment::Left,
            text
        );
    }

    // Blit the contents of "window" to the root console
    let x = SCREEN_WIDTH / 2 - width / 2;
    let y = SCREEN_HEIGHT / 2 - height / 2;
    blit(
        &window,
        (0, 0),
        (width, height),
        root,
        (x, y),
        1.0,
        0.7
    );

    // Present the root console to the player and wait for a key-press
    root.flush();
    let key = root.wait_for_keypress(true);

    // Convert the ASCII code to an index; If it corresponds to an option, return it
    if key.printable.is_alphabetic() {
        let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
        if index < options.len() {
            Some(index)
        } else {
            None
        }
    } else {
        None
    }
}

const INVENTORY_WIDTH: i32 = 50;

pub fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root) -> Option<usize> {
    let options = if inventory.len() == 0 {
        vec!["Inventory is empty.".into()]
    } else {
        inventory.iter().map(|item| item.name.clone()).collect()
    };

    let inventory_index = menu(header, &options, INVENTORY_WIDTH, root);

    // If an item was chosen, return it
    if inventory.len() > 0 {
        inventory_index
    } else {
        None
    }
}

pub struct Game {
    pub map: Map,
    pub messages: Messages,
    pub inventory: Vec<Object>,
}

impl Game {
    pub fn new(objects: &mut Vec<Object>) -> Self {
        Game {
            map: Game::make_map(objects),
            messages: Messages::new(),
            inventory: vec![],
        }
    }

    fn make_map(objects: &mut Vec<Object>) -> Map {
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
                place_objects(new_room, &map, objects);

                // Center coordinates of the new room
                let (new_x, new_y) = new_room.center();

                if rooms.is_empty() {
                    // Set the player here
                    objects[PLAYER].set_pos(new_x, new_y);
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

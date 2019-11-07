use crate::log::Messages;
use crate::map::*;
use crate::object::{place_objects, Object};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tcod::colors::*;

#[derive(Serialize, Deserialize)]
pub struct Game {
    pub map: Map,
    pub messages: Messages,
    pub inventory: Vec<Object>,
    pub dungeon_level: u32,
}

impl Game {
    pub fn new(objects: &mut Vec<Object>) -> Self {
        Game {
            map: Game::make_map(objects),
            messages: Messages::new(),
            inventory: vec![],
            dungeon_level: 1,
        }
    }

    pub fn make_map(objects: &mut Vec<Object>) -> Map {
        let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];
        // Player is the first element, remove everything else.
        // NOTE: works only when the player is the first object!
        assert_eq!(&objects[PLAYER] as *const _, &objects[0] as *const _);
        objects.truncate(1);
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

        // Create stairs at the center of the last room
        let (last_room_x, last_room_y) = rooms[rooms.len() - 1].center();
        let mut stairs = Object::new(last_room_x, last_room_y, '<', "Stairs", WHITE, false);
        stairs.always_visible = true;
        objects.push(stairs);

        map
    }
}

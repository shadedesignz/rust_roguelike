use crate::log::Messages;
use crate::map::*;
use crate::object::{place_objects, Object};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
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

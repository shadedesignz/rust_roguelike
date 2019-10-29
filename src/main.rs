use tcod::colors::*;
use tcod::console::*;
use tcod::map::{Map as FovMap};

mod object;
mod map;

use object::Object;
use map::{Game, MAP_WIDTH, MAP_HEIGHT, COLOR_DARK_GROUND, COLOR_DARK_WALL};
use crate::map::{TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGORITHM, COLOR_LIGHT_WALL, COLOR_LIGHT_GROUND, PLAYER};
use crate::object::PlayerAction;
use crate::object::PlayerAction::*;

// Actual window size
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;

// Max FPS
const LIMIT_FPS: i32 = 20;

struct Tcod {
    root: Root,
    con: Offscreen,
    fov: FovMap,
}

fn render_all(tcod: &mut Tcod, game: &mut Game, objects: &[Object], fov_recompute: bool) {
    if fov_recompute {
        let player = &objects[PLAYER];
        tcod.fov
            .compute_fov(
                player.x,
                player.y,
                TORCH_RADIUS,
                FOV_LIGHT_WALLS,
                FOV_ALGORITHM);
    }

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let visible = tcod.fov.is_in_fov(x, y);
            let tile = &mut game.map[x as usize][y as usize];
            let wall = tile.block_sight;
            let color = match (visible, wall) {
                // Outside of FOV
                (false, true) => COLOR_DARK_WALL,
                (false, false) => COLOR_DARK_GROUND,
                // Inside of FOV
                (true, true) => COLOR_LIGHT_WALL,
                (true, false) => COLOR_LIGHT_GROUND,
            };
            let explored = &mut tile.explored;
            if visible {
                *explored = true;
            }
            if *explored {
                tcod.con.set_char_background(x, y, color, BackgroundFlag::Set);
            }
        }
    }

    for object in objects {
        if tcod.fov.is_in_fov(object.x, object.y) {
            object.draw(&mut tcod.con);
        }
    }

    blit(
        &tcod.con,
        (0, 0),
        (MAP_WIDTH, MAP_HEIGHT),
        &mut tcod.root,
        (0, 0),
        1.0,
        1.0,
    );
}

fn player_move_or_attack(dx: i32, dy: i32, game: &Game, objects: &mut [Object]) {
    // Coords the player is moving to/attacking
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;

    // Try to find an attack-able object
    let target_id = objects
        .iter()
        .position(|object| object.pos() == (x, y));

    // Attack if target found, else move
    match target_id {
        Some(target_id) => {
            println!(
                "The {} laughs at your puny efforts to attack him!",
                objects[target_id].name
            );
        },
        None => {
            Object::move_by(PLAYER, dx, dy, &game.map, objects);
        }
    }
}

fn handle_keys(tcod: &mut Tcod, game: &Game, objects: &mut Vec<Object>) -> PlayerAction {
    use tcod::input::Key;
    use tcod::input::KeyCode::*;

    let key = tcod.root.wait_for_keypress(true);
    let player_alive = objects[PLAYER].alive;
    match (key, key.text(), player_alive) {
        // Movement
        (Key { code: Up, .. }, _, true) => {
            player_move_or_attack(0, -1, game, objects);
            TookTurn
        },
        (Key { code: Down, .. }, _, true) => {
            player_move_or_attack(0, 1, game, objects);
            TookTurn
        },
        (Key { code: Left, .. }, _, true) => {
            player_move_or_attack(-1, 0, game, objects);
            TookTurn
        },
        (Key { code: Right, .. }, _, true) => {
            player_move_or_attack(1, 0, game, objects);
            TookTurn
        },

        (
            Key {
                code: Enter,
                alt: true,
                ..
            },
            _,
            _,
        ) => {
            // Alt+Enter: toggle fullscreen
            let fullscreen = tcod.root.is_fullscreen();
            tcod.root.set_fullscreen(!fullscreen);
            DidntTakeTurn
        },
        // Exit the game
        (Key { code: Escape, .. }, _, _) => Exit,

        _ => DidntTakeTurn
    }
}

fn main() {
    tcod::system::set_fps(LIMIT_FPS);

    let root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Rust Roguelike")
        .init();

    let mut tcod = Tcod {
        root,
        con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
    };

    let mut player = Object::new(0, 0, '@', "player", WHITE, true);
    player.alive = true;

    let mut objects = vec![player];

    let mut game = Game::new(&mut objects);

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            tcod.fov.set(
                x,
                y,
                !game.map[x as usize][y as usize].block_sight,
                !game.map[x as usize][y as usize].blocked,
            );
        }
    }

    let mut previous_player_position = (-1, -1);

    while !tcod.root.window_closed() {
        tcod.con.clear();

        let fov_recompute = previous_player_position != (objects[PLAYER].pos());
        render_all(&mut tcod, &mut game, &objects, fov_recompute);
        tcod.root.flush();

        previous_player_position = objects[PLAYER].pos();
        let player_action = handle_keys(&mut tcod, &game, &mut objects);
        if player_action == Exit {
            break;
        }

        // Let monsters take their turn
        if objects[PLAYER].alive && player_action != DidntTakeTurn {
            for object in &objects {
                // Non-player moves
                if (object as *const _) != (&objects[PLAYER] as *const _) {
                    println!("The {} growls!", object.name);
                }
            }
        }
    }
}

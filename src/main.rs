use tcod::colors::*;
use tcod::console::*;
use tcod::map::{Map as FovMap};

mod object;
mod map;
mod ai;
mod gui;
mod log;

use object::Object;
use map::{Game, MAP_WIDTH, MAP_HEIGHT, COLOR_DARK_GROUND, COLOR_DARK_WALL};
use crate::map::{TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGORITHM, COLOR_LIGHT_WALL, COLOR_LIGHT_GROUND, PLAYER};
use crate::object::PlayerAction;
use crate::object::PlayerAction::*;
use crate::ai::{Fighter, ai_take_turn, mut_two, DeathCallback};
use crate::gui::{PANEL_HEIGHT, render_bar, BAR_WIDTH, PANEL_Y};
use crate::log::{MSG_HEIGHT, MSG_X, MSG_WIDTH};

// Actual window size
pub const SCREEN_WIDTH: i32 = 80;
pub const SCREEN_HEIGHT: i32 = 50;

// Max FPS
const LIMIT_FPS: i32 = 20;

pub struct Tcod {
    pub root: Root,
    pub con: Offscreen,
    pub panel: Offscreen,
    pub fov: FovMap,
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

    let mut to_draw: Vec<_> = objects
        .iter()
        .filter(|o| tcod.fov.is_in_fov(o.x, o.y))
        .collect();
    // Sort so that non-blocking objects come first
    to_draw.sort_by(|o1, o2| { o1.blocks.cmp(&o2.blocks) });
    // Draw the objects in the list
    for object in &to_draw {
        object.draw(&mut tcod.con);
    }

    // Show player stats
    tcod.panel.set_default_background(BLACK);
    tcod.panel.clear();

    // Print the game messages, one line at a time
    let mut y = MSG_HEIGHT as i32;
    for &(ref msg, color) in game.messages.iter().rev() {
        let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        tcod.panel.set_default_foreground(color);
        tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
    }

    // Show player stats
    let hp = objects[PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
    render_bar(
        &mut tcod.panel,
        1,
        1,
        BAR_WIDTH,
        "HP",
        hp,
        max_hp,
        LIGHT_RED,
        DARKER_RED,
    );

    blit(
        &tcod.panel,
        (0, 0),
        (SCREEN_WIDTH, PANEL_HEIGHT),
        &mut tcod.root,
        (0, PANEL_Y),
        1.0,
        1.0,
    );

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

fn player_move_or_attack(dx: i32, dy: i32, game: &mut Game, objects: &mut [Object]) {
    // Coords the player is moving to/attacking
    let x = objects[PLAYER].x + dx;
    let y = objects[PLAYER].y + dy;

    // Try to find an attack-able object
    let target_id = objects
        .iter()
        .position(|object| object.fighter.is_some() && object.pos() == (x, y));

    // Attack if target found, else move
    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(PLAYER, target_id, objects);
            player.attack(target, game);
        },
        None => {
            Object::move_by(PLAYER, dx, dy, &game.map, objects);
        }
    }
}

fn handle_keys(tcod: &mut Tcod, game: &mut Game, objects: &mut Vec<Object>) -> PlayerAction {
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
        panel: Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
        fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
    };

    let mut player = Object::new(0, 0, '@', "player", WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter {
        max_hp: 30,
        hp: 30,
        defense: 2,
        power: 5,
        on_death: DeathCallback::Player,
    });

    let mut objects = vec![player];

    let mut game = Game::new(&mut objects);

    // Add a welcome message
    game.messages.add(
        "Welcome stranger! Prepare to perish in the Tombs of the Ancient Kings.",
        RED
    );

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
        let player_action = handle_keys(&mut tcod, &mut game, &mut objects);
        if player_action == Exit {
            break;
        }

        // Let monsters take their turn
        if objects[PLAYER].alive && player_action != DidntTakeTurn {
            for id in 0..objects.len() {
                if objects[id].ai.is_some() {
                    ai_take_turn(id, &tcod, &mut game, &mut objects);
                }
            }
        }
    }
}

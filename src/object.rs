use tcod::{Color, Console, BackgroundFlag, colors};
use crate::map::{Rect, Map, Game, PLAYER};
use rand::Rng;
use crate::ai::{Fighter, Ai, DeathCallback};
use tcod::colors::{WHITE, VIOLET, RED, GREEN, LIGHT_VIOLET};
use crate::Tcod;

const MAX_ROOM_MONSTERS: i32 = 3;
const MAX_ROOM_ITEMS: i32 = 2;
const HEAL_AMOUNT: i32 = 4;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Item {
    Heal,
}

pub enum UseResult {
    UsedUp,
    Cancelled,
}

fn cast_heal(
    _inventory_id: usize,
    _tcod: &mut Tcod,
    game: &mut Game,
    objects: &mut [Object]
) -> UseResult {
    // Heal the player
    if let Some(fighter) = objects[PLAYER].fighter {
        if fighter.hp == fighter.max_hp {
            game.messages.add("You are already at full health.", RED);
            return UseResult::Cancelled;
        }
        game.messages.add(
            "Your wounds start to feel better!", LIGHT_VIOLET
        );
        objects[PLAYER].heal(HEAL_AMOUNT);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

#[derive(Debug)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    pub char: char,
    pub color: Color,
    pub name: String,
    pub blocks: bool,
    pub alive: bool,
    pub fighter: Option<Fighter>,
    pub ai: Option<Ai>,
    pub item: Option<Item>,
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, blocks: bool) -> Self {
        Object {
            x,
            y,
            char,
            color,
            name: name.into(),
            blocks,
            alive: false,
            fighter: None,
            ai: None,
            item: None,
        }
    }

    pub fn heal(&mut self, amount: i32) {
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > fighter.max_hp {
                fighter.hp = fighter.max_hp;
            }
        }
    }

    pub fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
        let (x, y) = objects[id].pos();
        if !is_blocked(x + dx, y + dy, map, objects) {
            objects[id].set_pos(x + dx, y + dy);
        }
    }

    pub fn draw(&self, con: &mut dyn Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }

    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }


    pub fn can_attack(&self, other: &Object) -> bool {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        let in_range = ((dx.pow(2) + dy.pow(2)) as f32).sqrt() < 2.0;
        let is_not_diagonal = other.x == self.x || other.y == self.y;
        in_range && is_not_diagonal
    }

    pub fn distance_to(&self, other: &Object) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }

    pub fn take_damage(&mut self, damage: i32, game: &mut Game) {
        // Apply damage if possible
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                if fighter.hp - damage < 0 {
                    fighter.hp = 0;
                } else {
                    fighter.hp -= damage;
                }
            }
        }
        if let Some(fighter) = self.fighter {
            if fighter.hp == 0 {
                self.alive = false;
                fighter.on_death.callback(self, game);
            }
        }
    }

    pub fn attack(&mut self, target: &mut Object, game: &mut Game) {
        // Simple attack formula for damage
        let damage = self.fighter
            .map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);
        if damage > 0 {
            // Target takes damage
            game.messages.add(
                format!(
                    "{} attacks {} for {} hp",
                    self.name, target.name, damage
                ),
                WHITE,
            );
            target.take_damage(damage, game);
        } else {
            game.messages.add(
                format!(
                    "{} attacks {} but it has no effect!",
                    self.name, target.name
                ),
                WHITE,
            );
        }
    }

    pub fn pick_item_up(object_id: usize, game: &mut Game, objects: &mut  Vec<Object>) {
        if game.inventory.len() >= 26 {
            game.messages.add(
                format!(
                    "Your inventory is full, cannot pick up {}",
                    objects[object_id].name,
                ),
                RED,
            );
        } else {
            let item = objects.swap_remove(object_id);
            game.messages.add(
                format!("You picked up a {}!", item.name),
                GREEN,
            );
            game.inventory.push(item);
        }
    }

    pub fn use_item(inventory_id: usize, tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) {
        use Item::*;
        // Just call the "use_function" if it is defined
        if let Some(item) = game.inventory[inventory_id].item {
            let on_use = match item {
                Heal => cast_heal,
            };
            match on_use(inventory_id, tcod, game, objects) {
                UseResult::UsedUp => {
                    // Destroy after use, unless it was cancelled for some reason
                    game.inventory.remove(inventory_id);
                }
                UseResult::Cancelled => {
                    game.messages.add("Cancelled", WHITE);
                }
            }
        } else {
            game.messages.add(
                format!("The {} cannot be used.", game.inventory[inventory_id].name),
                WHITE,
            );
        }
    }
}

pub fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    if map[x as usize][y as usize].blocked {
        return true;
    }

    objects
        .iter()
        .any(|object| object.blocks && object.pos() == (x, y))
}

pub fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>) {
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !is_blocked(x, y, map, objects) {
            // 0.8 = 80% chance of getting an orc
            let mut monster = if rand::random::<f32>() < 0.8 {
                let mut orc = Object::new(x, y, 'o', "Orc", colors::DESATURATED_GREEN, true);
                orc.fighter = Some(Fighter {
                    max_hp: 10,
                    hp: 10,
                    defense: 0,
                    power: 3,
                    on_death: DeathCallback::Monster,
                });
                orc.ai = Some(Ai::Basic);
                orc
            } else {
                let mut troll = Object::new(x, y, 'T', "Troll", colors::DARKER_GREEN, true);
                troll.fighter = Some(Fighter {
                    max_hp: 16,
                    hp: 16,
                    defense: 1,
                    power: 4,
                    on_death: DeathCallback::Monster,
                });
                troll.ai = Some(Ai::Basic);
                troll
            };

            monster.alive = true;
            objects.push(monster);
        }
    }

    // Choose random number of items
    let num_items = rand::thread_rng().gen_range(0, MAX_ROOM_ITEMS + 1);

    for _ in 0..num_items {
        // Choose random spot for this item
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        // Only place an item if the tile is not blocked
        if !is_blocked(x, y, map, objects) {
            // Create a healing potion
            let mut object = Object::new(x, y, '!', "Healing Potion", VIOLET, false);
            object.item = Some(Item::Heal);
            objects.push(object);
        }
    }
}

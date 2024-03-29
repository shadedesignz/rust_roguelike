use crate::ai::{Ai, DeathCallback, Fighter};
use crate::map::{menu, Map, Rect, MAP_HEIGHT, MAP_WIDTH, PLAYER};
use crate::{render_all, Tcod};
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use rand::Rng;
use tcod::colors::*;
use tcod::input::Event;

use crate::equipment::{Equipment, Slot};
use crate::game::Game;
use crate::item::{from_dungeon_level, Item, Potion, Scroll, Transition};
use crate::log::Messages;
use serde::{Deserialize, Serialize};
use tcod::{input, BackgroundFlag, Console};

pub const LEVEL_UP_BASE: i32 = 200;
pub const LEVEL_UP_FACTOR: i32 = 150;

const LEVEL_SCREEN_WIDTH: i32 = 40;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

pub fn target_monster(
    tcod: &mut Tcod,
    game: &mut Game,
    objects: &[Object],
    max_range: Option<f32>,
) -> Option<usize> {
    loop {
        match target_tile(tcod, game, objects, max_range) {
            Some((x, y)) => {
                // Return the first clicked monster, otherwise continue looping
                for (id, obj) in objects.iter().enumerate() {
                    if obj.pos() == (x, y) && obj.fighter.is_some() && id != PLAYER {
                        return Some(id);
                    }
                }
            }
            None => return None,
        }
    }
}

pub fn closest_monster(tcod: &Tcod, objects: &[Object], max_range: i32) -> Option<usize> {
    let mut closest_enemy = None;
    // Start with (slightly more than) maximum range
    let mut closest_dist = (max_range + 1) as f32;

    for (id, object) in objects.iter().enumerate() {
        if id != PLAYER
            && object.fighter.is_some()
            && object.ai.is_some()
            && tcod.fov.is_in_fov(object.x, object.y)
        {
            // Calculate distance between this object and the player
            let dist = objects[PLAYER].distance_to(object);
            if dist < closest_dist {
                // It's closer, so remember it
                closest_enemy = Some(id);
                closest_dist = dist;
            }
        }
    }
    closest_enemy
}

pub fn level_up(tcod: &mut Tcod, game: &mut Game, objects: &mut [Object]) {
    let player = &mut objects[PLAYER];
    let level_up_xp = LEVEL_UP_BASE + player.level * LEVEL_UP_FACTOR;
    // See if the player's xp is enough to level up
    if player.fighter.as_ref().map_or(0, |f| f.xp) >= level_up_xp {
        // Level up
        player.level += 1;
        game.messages.add(
            format!(
                "Your battle skills grow stronger! You reached level {}",
                player.level
            ),
            YELLOW,
        );
        let fighter = player.fighter.as_mut().unwrap();
        let mut choice = None;
        while choice.is_none() {
            // Keep asking until a choice is made
            choice = menu(
                "Level up! Choose a stat to raise:\n",
                &[
                    format!("Constitution (+20 HP, from {})", fighter.base_max_hp),
                    format!("Strength (+1 attack, from {})", fighter.base_power),
                    format!("Agility (+1 defense, from {})", fighter.base_defense),
                ],
                LEVEL_SCREEN_WIDTH,
                &mut tcod.root,
            );
        }
        fighter.xp -= level_up_xp;
        match choice.unwrap() {
            0 => {
                fighter.base_max_hp += 20;
                fighter.hp += 20;
            }
            1 => {
                fighter.base_power += 1;
            }
            2 => {
                fighter.base_defense += 1;
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
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
    pub always_visible: bool,
    pub level: i32,
    pub equipment: Option<Equipment>,
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
            always_visible: false,
            level: 1,
            equipment: None,
        }
    }

    pub fn power(&self, game: &Game) -> i32 {
        let base_power = self.fighter.map_or(0, |f| f.base_power);
        let bonus: i32 = self
            .get_all_equipped(game)
            .iter()
            .map(|e| e.power_bonus)
            .sum();
        base_power + bonus
    }

    pub fn defense(&self, game: &Game) -> i32 {
        let base_defense = self.fighter.map_or(0, |f| f.base_defense);
        let bonus: i32 = self
            .get_all_equipped(game)
            .iter()
            .map(|e| e.defense_bonus)
            .sum();
        base_defense + bonus
    }

    pub fn max_hp(&self, game: &Game) -> i32 {
        let base_max_hp = self.fighter.map_or(0, |f| f.base_max_hp);
        let bonus: i32 = self
            .get_all_equipped(game)
            .iter()
            .map(|e| e.max_hp_bonus)
            .sum();
        base_max_hp + bonus
    }

    pub fn get_all_equipped(&self, game: &Game) -> Vec<Equipment> {
        if self.name == "Player" {
            game.inventory
                .iter()
                .filter(|item| item.equipment.map_or(false, |e| e.equipped))
                .map(|item| item.equipment.unwrap())
                .collect()
        } else {
            vec![]
        }
    }

    pub fn equip(&mut self, messages: &mut Messages) {
        if self.item.is_none() {
            messages.add(
                format!("Can't equip {:?} because it's not an Item.", self),
                RED,
            );
            return;
        }
        if let Some(ref mut equipment) = self.equipment {
            if !equipment.equipped {
                equipment.equipped = true;
                messages.add(
                    format!("Equipped {} on {}", self.name, equipment.slot),
                    LIGHT_GREEN,
                );
            }
        } else {
            messages.add(
                format!("Can't equip {:?} because it's not an Equipment Item.", self),
                RED,
            );
        }
    }

    pub fn dequip(&mut self, messages: &mut Messages) {
        if self.item.is_none() {
            messages.add(
                format!("Can't dequip {:?} because it's not an Item.", self),
                RED,
            );
            return;
        }
        if let Some(ref mut equipment) = self.equipment {
            if !equipment.equipped {
                equipment.equipped = false;
                messages.add(
                    format!("Dequipped {} on {}", self.name, equipment.slot),
                    LIGHT_YELLOW,
                );
            }
        } else {
            messages.add(
                format!(
                    "Can't dequip {:?} because it's not an Equipment Item.",
                    self
                ),
                RED,
            );
        }
    }

    pub fn heal(&mut self, amount: i32, game: &Game) {
        let max_hp = self.max_hp(game);
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > max_hp {
                fighter.hp = max_hp;
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

    pub fn distance_to(&self, other: &Object) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }

    pub fn take_damage(&mut self, damage: i32, game: &mut Game) -> Option<i32> {
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
                self.always_visible = true;
                fighter.on_death.callback(self, game);
                return Some(fighter.xp);
            }
        }
        None
    }

    pub fn attack(&mut self, target: &mut Object, game: &mut Game) {
        // Simple attack formula for damage
        let damage = self.power(game) - target.defense(game);
        if damage > 0 {
            // Target takes damage
            game.messages.add(
                format!("{} attacks {} for {} hp", self.name, target.name, damage),
                WHITE,
            );
            if let Some(xp) = target.take_damage(damage, game) {
                // Yield xp to the player
                self.fighter.as_mut().unwrap().xp += xp;
            }
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

    pub fn distance(&self, x: i32, y: i32) -> f32 {
        (((x - self.x).pow(2) + (y - self.y).pow(2)) as f32).sqrt()
    }
}

pub fn target_tile(
    tcod: &mut Tcod,
    game: &mut Game,
    objects: &[Object],
    max_range: Option<f32>,
) -> Option<(i32, i32)> {
    use tcod::input::KeyCode::Escape;
    loop {
        // Render the screen, erasing the inventory and shows the names of
        // objects under the mouse
        tcod.root.flush();
        let event = input::check_for_event(input::KEY_PRESS | input::MOUSE).map(|e| e.1);
        match event {
            Some(Event::Mouse(m)) => tcod.mouse = m,
            Some(Event::Key(k)) => tcod.key = k,
            None => tcod.key = Default::default(),
        }
        render_all(tcod, game, objects, false);

        let (x, y) = (tcod.mouse.cx as i32, tcod.mouse.cy as i32);

        // Accept the target if the player clicked in FOV, and in case a range
        // is specified, if it's in that range
        let in_fov = (x < MAP_WIDTH) && (y < MAP_HEIGHT) && tcod.fov.is_in_fov(x, y);
        let in_range = max_range.map_or(true, |range| objects[PLAYER].distance(x, y) <= range);
        if tcod.mouse.lbutton_pressed && in_fov && in_range {
            return Some((x, y));
        }

        if tcod.mouse.rbutton_pressed || tcod.key.code == Escape {
            return None;
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

pub fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>, level: u32) {
    // Max number of monsters per room
    let max_monsters = from_dungeon_level(
        &[
            Transition { level: 1, value: 2 },
            Transition { level: 4, value: 3 },
            Transition { level: 6, value: 5 },
        ],
        level,
    );
    let num_monsters = rand::thread_rng().gen_range(0, max_monsters + 1);

    let troll_chance = from_dungeon_level(
        &[
            Transition {
                level: 3,
                value: 15,
            },
            Transition {
                level: 5,
                value: 30,
            },
            Transition {
                level: 7,
                value: 60,
            },
        ],
        level,
    );

    // Monster random choice table
    let monster_chances = [("Orc", 80), ("Troll", troll_chance)];
    let monster_choice = WeightedIndex::new(monster_chances.iter().map(|item| item.1)).unwrap();

    for _ in 0..num_monsters {
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !is_blocked(x, y, map, objects) {
            let mut monster =
                match monster_chances[monster_choice.sample(&mut rand::thread_rng())].0 {
                    "Orc" => {
                        let mut orc = Object::new(x, y, 'o', "Orc", DESATURATED_GREEN, true);
                        orc.fighter = Some(Fighter {
                            base_max_hp: 20,
                            hp: 20,
                            base_defense: 0,
                            base_power: 4,
                            xp: 35,
                            on_death: DeathCallback::Monster,
                        });
                        orc.ai = Some(Ai::Basic);
                        orc
                    }
                    "Troll" => {
                        let mut troll = Object::new(x, y, 'T', "Troll", DARKER_GREEN, true);
                        troll.fighter = Some(Fighter {
                            base_max_hp: 30,
                            hp: 30,
                            base_defense: 2,
                            base_power: 8,
                            xp: 100,
                            on_death: DeathCallback::Monster,
                        });
                        troll.ai = Some(Ai::Basic);
                        troll
                    }
                    _ => unreachable!(),
                };

            monster.alive = true;
            objects.push(monster);
        }
    }

    let max_items = from_dungeon_level(
        &[
            Transition { level: 1, value: 1 },
            Transition { level: 4, value: 2 },
        ],
        level,
    );

    // Choose random number of items
    let num_items = rand::thread_rng().gen_range(0, max_items + 1);

    // Item random choice table
    let item_chances = [
        (Item::Heal, 35),
        (
            Item::Lightning,
            from_dungeon_level(
                &[Transition {
                    level: 4,
                    value: 25,
                }],
                level,
            ),
        ),
        (
            Item::Fireball,
            from_dungeon_level(
                &[Transition {
                    level: 6,
                    value: 25,
                }],
                level,
            ),
        ),
        (
            Item::Confuse,
            from_dungeon_level(
                &[Transition {
                    level: 2,
                    value: 10,
                }],
                level,
            ),
        ),
        (
            Item::Sword,
            from_dungeon_level(&[Transition { level: 4, value: 5 }], level),
        ),
        (
            Item::Shield,
            from_dungeon_level(
                &[Transition {
                    level: 8,
                    value: 15,
                }],
                level,
            ),
        ),
    ];
    let item_choice = WeightedIndex::new(item_chances.iter().map(|item| item.1)).unwrap();

    for _ in 0..num_items {
        // Choose random spot for this item
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        // Only place an item if the tile is not blocked
        if !is_blocked(x, y, map, objects) {
            let mut item = match item_chances[item_choice.sample(&mut rand::thread_rng())].0 {
                Item::Heal => Potion::new(x, y, "Healing", Item::Heal),
                Item::Lightning => Scroll::new(x, y, "Lightning Bolt", Item::Lightning),
                Item::Fireball => Scroll::new(x, y, "Fireball", Item::Fireball),
                Item::Confuse => Scroll::new(x, y, "Confusion", Item::Confuse),
                Item::Sword => {
                    // Create a sword
                    let mut object = Object::new(x, y, '/', "Sword", SKY, false);
                    object.item = Some(Item::Sword);
                    object.equipment = Some(Equipment {
                        equipped: false,
                        slot: Slot::RightHand,
                        max_hp_bonus: 0,
                        defense_bonus: 0,
                        power_bonus: 3,
                    });
                    object
                }
                Item::Shield => {
                    // create a shield
                    let mut object = Object::new(x, y, '[', "shield", DARKER_ORANGE, false);
                    object.item = Some(Item::Shield);
                    object.equipment = Some(Equipment {
                        equipped: false,
                        slot: Slot::LeftHand,
                        max_hp_bonus: 0,
                        defense_bonus: 1,
                        power_bonus: 0,
                    });
                    object
                }
            };
            item.always_visible = true;
            objects.push(item);
        }
    }
}

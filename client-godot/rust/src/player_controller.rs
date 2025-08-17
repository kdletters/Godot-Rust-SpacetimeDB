use super::*;
use godot::classes::{Input, InputEvent, InputEventKey, Label, Time};
use godot::global::Key;
use std::sync::Arc;
use tokio::sync::OnceCell;

#[derive(GodotClass)]
#[class(init, base=Node2D)]
pub struct PlayerController {
    base: Base<Node2D>,

    #[init(node="%Label")]
    label: OnReady<Gd<Label>>,

    player_id: u32,
    pub last_movement_send_timestamp: f32,
    pub lock_input_position: Option<Vector2>,
    pub owned_circles: Vec<Gd<CircleController>>,
}

unsafe impl Send for PlayerController {}
unsafe impl Sync for PlayerController {}

pub static mut LOCAL: Option<Arc<Gd<PlayerController>>> = None;

pub fn get_local() -> Option<Arc<Gd<PlayerController>>> {
    unsafe {
        let a = &raw const LOCAL;
        if let Some(conn) = (*a).clone() {
            Some(conn)
        } else {
            None
        }
    }
}

fn find_entity(entity_id: u32) -> Option<Entity> {
    if let Some(conn) = get_connection() {
        return conn.db.entity().entity_id().find(&entity_id);
    }

    None
}

impl PlayerController {
    const SEND_UPDATES_PER_SEC: i32 = 20;
    const SEND_UPDATES_FREQUENCY: f32 = 1.0 / (Self::SEND_UPDATES_PER_SEC as f32);

    pub fn username(&self) -> String {
        get_connection()
            .unwrap()
            .db
            .player()
            .player_id()
            .find(&self.player_id)
            .unwrap()
            .name
    }

    pub fn number_of_owned_circles(&self) -> isize {
        self.owned_circles.len() as isize
    }

    pub fn is_local_player(&self) -> bool {
        if let Some(local) = get_local() {
            self.to_gd().eq(&*local)
        } else {
            false
        }
    }

    pub fn initialize(&mut self, player: Player) {
        self.player_id = player.player_id;
        godot_print!(
            "PlayerController::initialize: {}",
            LOCAL_IDENTITY.get().is_some()
        );
        if let Some(local) = LOCAL_IDENTITY.get()
            && player.identity == *local
        {
            unsafe {
                LOCAL = Some(Arc::new(self.to_gd()));
            }
        }
    }

    pub fn on_circle_spawned(&mut self, circle: Gd<CircleController>) {
        self.owned_circles.push(circle);
    }

    pub fn on_circle_deleted(&mut self, deleted_circle: Gd<CircleController>) {
        // This means we got eaten
        if let Some(i) = self
            .owned_circles
            .iter()
            .position(|x| x.eq(&deleted_circle))
        {
            self.owned_circles.remove(i);
            if self.is_local_player() && self.owned_circles.len() == 0 {
                // DeathScreen.Instance.SetVisible(true);}
            }
        }
    }

    pub fn total_mass(&self) -> u32 {
        let mass = self
            .owned_circles
            .iter()
            .map(|x| {
                let entity_id = x.bind().entity.entity_id;
                if let Some(entity) = find_entity(entity_id) {
                    entity.mass
                } else {
                    0
                }
            })
            .sum();
        mass
    }

    pub fn center_of_mass(&self) -> Option<Vector2> {
        if self.owned_circles.len() == 0 {
            return None;
        }

        let mut total_pos = Vector2::ZERO;
        let mut total_mass = 0;
        for circle in &self.owned_circles {
            if let Some(entity) = find_entity(circle.bind().entity.entity_id) {
                let position = circle.bind().base().get_position();
                total_pos += position * entity.mass as f32;
                total_mass += entity.mass;
            }
        }

        Some(total_pos / (total_mass as f32))
    }
}

#[godot_api]
impl INode2D for PlayerController {
    fn process(&mut self, _: f64) {
        if !self.is_local_player() || self.number_of_owned_circles() == 0 {
            return;
        }

        let total_mass = self.total_mass();
        self.label.set_text(&format!("Total Mass: {}", total_mass));

        // Throttled input requests
        let time = Time::singleton().get_ticks_msec() as f32;
        if time - self.last_movement_send_timestamp > Self::SEND_UPDATES_FREQUENCY {
            self.last_movement_send_timestamp = time;

            let mouse_position = if let Some(pos) = self.lock_input_position {
                pos
            } else {
                self.base().get_viewport().unwrap().get_mouse_position()
            };
            let screen_size = self.base().get_viewport_rect().size;
            let screen_size = Vector2::new(screen_size.x as f32, screen_size.y as f32);
            let center_of_screen = screen_size * 0.5;

            let direction = (mouse_position - center_of_screen) / (screen_size.y / 3.0);
            if let Some(conn) = get_connection() {
                conn.reducers.update_player_input(direction.into()).unwrap()
            }
        }
    }

    fn exit_tree(&mut self) {
        for circle in &mut self.owned_circles {
            circle.queue_free();
        }

        self.owned_circles.clear();
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if !self.is_local_player() || self.number_of_owned_circles() == 0 {
            return;
        }

        if let Ok(key) = event.try_cast::<InputEventKey>() {
            if key.get_keycode() == Key::Q && !key.is_echo() {
                if self.lock_input_position.is_some() {
                    self.lock_input_position = None;
                } else {
                    self.lock_input_position =
                        Some(self.base().get_viewport().unwrap().get_mouse_position());
                }
            }
        }
    }
}

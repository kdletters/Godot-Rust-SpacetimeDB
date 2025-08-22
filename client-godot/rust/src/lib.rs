mod camera_controller;
mod circle_controller;
mod entity_controller;
mod extensions;
mod food_batch_renderer;
mod game_manager;
mod global_state;
mod module_bindings;
mod player_controller;
mod prefab_manager;

pub use camera_controller::*;
pub use circle_controller::*;
pub use entity_controller::*;
pub use food_batch_renderer::*;
pub use game_manager::*;
pub use global_state::*;
pub use module_bindings::*;
pub use player_controller::*;
pub use prefab_manager::*;

pub use godot::classes::Engine;
pub use godot::prelude::*;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {
    fn on_level_init(level: InitLevel) {
        godot_print!("Initializing level: {:?}", level);
    }
}

pub fn get_root() -> Option<Gd<Node>> {
    Engine::singleton()
        .get_main_loop()
        .unwrap()
        .cast::<SceneTree>()
        .get_root()
        .map(|x| x.upcast::<Node>())
}

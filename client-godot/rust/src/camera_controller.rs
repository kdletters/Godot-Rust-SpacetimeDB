use super::*;
use godot::classes::{Camera2D, ENetMultiplayerPeer, ICamera2D};
use std::ops::Deref;
use std::sync::atomic::AtomicU32;

pub static WORLD_SIZE: AtomicU32 = AtomicU32::new(0);

#[derive(GodotClass)]
#[class(init, base=Camera2D)]
pub struct CameraController {
    base: Base<Camera2D>,
}

#[godot_api]
impl ICamera2D for CameraController {
    fn process(&mut self, delta: f32) {
        let arena_center_transform = Vector2::new(
            WORLD_SIZE.load(std::sync::atomic::Ordering::Relaxed) as f32 / 2.0,
            WORLD_SIZE.load(std::sync::atomic::Ordering::Relaxed) as f32 / 2.0,
        );

        if let Some(local) = get_local() {
            if !is_connected() {
                self.base_mut().set_global_position(arena_center_transform);
                return;
            }
            let center_of_mass = local.bind().center_of_mass();
            if let Some(center_of_mass) = center_of_mass {
                self.base_mut().set_global_position(center_of_mass);
            } else {
                self.base_mut().set_global_position(arena_center_transform);
            }
            let target_camera_size = self.calculate_camera_size(local.deref().clone());
            let viewport_size = self.base().get_viewport_rect().size;
            let target_camera_zoom = f32::min(viewport_size.x, viewport_size.y) / target_camera_size;
            let target_camera_zoom = Vector2::new(target_camera_zoom, target_camera_zoom);
            let zoom = self.base().get_zoom();
            self.base_mut()
                .set_zoom(Vector2::lerp(zoom, target_camera_zoom, delta * 2.0));
        } else {
            self.base_mut().set_global_position(arena_center_transform);
        }
    }
}

impl CameraController {
    fn calculate_camera_size(&self, player: Gd<PlayerController>) -> f32 {
        50.0 + f32::min(50.0, player.bind().total_mass() as f32 / 5.0)
            + isize::min(player.bind().number_of_owned_circles() - 1, 1) as f32 * 30.0
    }
}

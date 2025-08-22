use super::*;
use crate::global_state::*;
use godot::classes::{Camera2D, ICamera2D};
use std::ops::Deref;
use std::sync::atomic::AtomicU32;

pub static WORLD_SIZE: AtomicU32 = AtomicU32::new(0);

#[derive(GodotClass)]
#[class(base=Camera2D)]
pub struct CameraController {
    base: Base<Camera2D>,
    // 平滑跟随配置参数
    #[export]
    follow_speed: f32,
    #[export]
    max_distance_threshold: f32,
    #[export]
    speed_multiplier: f32,
    current_target_position: Vector2,
}

#[godot_api]
impl ICamera2D for CameraController {
    fn init(base: Base<Camera2D>) -> Self {
        Self {
            base,
            follow_speed: 5.0,              // 基础跟随速度
            max_distance_threshold: 100.0,   // 最大距离阈值
            speed_multiplier: 2.0,           // 距离速度倍数
            current_target_position: Vector2::ZERO,
        }
    }
    fn process(&mut self, delta: f32) {
        let arena_center_transform = Vector2::new(
            WORLD_SIZE.load(std::sync::atomic::Ordering::Relaxed) as f32 / 2.0,
            WORLD_SIZE.load(std::sync::atomic::Ordering::Relaxed) as f32 / 2.0,
        );

        if let Some(local) = players::get_local_player() {
            if !connection::is_connected() {
                // 在未连接状态下，也使用平滑过渡到中心位置
                let current_pos = self.base().get_global_position();
                let smooth_pos = self.smooth_follow_position(current_pos, arena_center_transform, delta);
                self.base_mut().set_global_position(smooth_pos);
                self.current_target_position = arena_center_transform;
                return;
            }
            
            let center_of_mass = local.bind().center_of_mass();
            let target_position = center_of_mass.unwrap_or(arena_center_transform);
            
            // 获取当前摄像机位置
            let current_pos = self.base().get_global_position();
            
            // 使用平滑跟随算法更新位置
            let smooth_pos = self.smooth_follow_position(current_pos, target_position, delta);
            self.base_mut().set_global_position(smooth_pos);
            
            // 记录当前目标位置
            self.current_target_position = target_position;
            
            // 缩放逻辑保持不变(已经有平滑处理)
            let target_camera_size = self.calculate_camera_size(local.deref().clone());
            let viewport_size = self.base().get_viewport_rect().size;
            let target_camera_zoom = f32::min(viewport_size.x, viewport_size.y) / target_camera_size;
            let target_camera_zoom = Vector2::new(target_camera_zoom, target_camera_zoom);
            let zoom = self.base().get_zoom();
            self.base_mut()
                .set_zoom(Vector2::lerp(zoom, target_camera_zoom, delta * 2.0));
        } else {
            // 在没有本地玩家时，也使用平滑过渡到中心位置
            let current_pos = self.base().get_global_position();
            let smooth_pos = self.smooth_follow_position(current_pos, arena_center_transform, delta);
            self.base_mut().set_global_position(smooth_pos);
            self.current_target_position = arena_center_transform;
        }
    }
}

impl CameraController {
    fn calculate_camera_size(&self, player: Gd<PlayerController>) -> f32 {
        50.0 + f32::min(50.0, player.bind().total_mass() as f32 / 5.0)
            + isize::min(player.bind().number_of_owned_circles() - 1, 1) as f32 * 30.0
    }

    /// 平滑跟随算法，根据距离自适应调整速度
    fn smooth_follow_position(&self, current_pos: Vector2, target_pos: Vector2, delta: f32) -> Vector2 {
        let distance = current_pos.distance_to(target_pos);
        
        // 自适应速度计算：距离越远，跟随速度越快
        let adaptive_speed = if distance > self.max_distance_threshold {
            self.follow_speed * self.speed_multiplier
        } else {
            self.follow_speed * (1.0 + distance / self.max_distance_threshold)
        };
        
        // 使用线性插值实现平滑过渡
        Vector2::lerp(current_pos, target_pos, (delta * adaptive_speed).clamp(0.0, 1.0))
    }
}

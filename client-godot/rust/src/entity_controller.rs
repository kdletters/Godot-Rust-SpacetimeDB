use super::*;
use crate::global_state::*;
use crate::module_bindings::{Entity, EntityTableAccess};
use godot::classes::{CanvasItem, Node2D, ShaderMaterial};
use godot::global::sqrt;
use godot::prelude::*;

pub enum EntityController {
    Circle(Gd<CircleController>),
    // Food 已移除，现在使用 FoodBatchRenderer 进行批量渲染
}

impl EntityController {
    pub fn on_entity_updated(&mut self, entity: &Entity) {
        match self {
            EntityController::Circle(circle) => {
                let node = circle.clone();
                circle.bind_mut().entity.on_entity_updated(entity, node);
            }
            // Food 变体已移除，由 FoodBatchRenderer 处理
        }
    }

    pub fn on_delete(&mut self) {
        match self {
            EntityController::Circle(circle) => {
                let node = circle.clone();
                circle.bind_mut().entity.on_delete(node);
            }
            // Food 变体已移除，由 FoodBatchRenderer 处理
        }
    }
}

pub struct EntityData {
    pub entity_id: u32,
    pub lerp_time: f32,
    pub lerp_start_position: Vector2,
    pub lerp_target_position: Vector2,
    pub target_scale: Vector2,
}

impl Default for EntityData {
    fn default() -> Self {
        Self {
            entity_id: 0,
            lerp_time: 0.0,
            lerp_start_position: Vector2::ZERO,
            lerp_target_position: Vector2::ZERO,
            target_scale: Vector2::ZERO,
        }
    }
}

impl EntityData {
    const LERP_DURATION_SEC: f32 = 0.1;

    pub fn spawn(&mut self, entity_id: u32, mut node2d: Gd<Node2D>) {
        let entity = connection::get_connection()
            .unwrap()
            .db
            .entity()
            .entity_id()
            .find(&entity_id)
            .unwrap();

        let position = entity.position.into();
        node2d.set_scale(Vector2::ONE);
        node2d.set_global_position(position);

        self.entity_id = entity_id;
        self.lerp_time = 0.0;
        self.lerp_start_position = position;
        self.lerp_target_position = position;
        self.target_scale = mass_to_scale(entity.mass);
    }

    pub fn set_color(&mut self, color: Color, node: Gd<CanvasItem>) {
        node.get_material()
            .unwrap()
            .cast::<ShaderMaterial>()
            .set_shader_parameter("tint", &color.to_variant());
    }

    pub fn on_entity_updated<T: Inherits<Node2D>>(&mut self, entity: &Entity, node2d: Gd<T>) {
        self.lerp_time = 0.0;
        self.lerp_start_position = node2d.upcast::<Node2D>().get_position();
        self.lerp_target_position = (&entity.position).into();
        self.target_scale = mass_to_scale(entity.mass);
    }

    pub fn on_delete<T: Inherits<Node>>(&mut self, node: Gd<T>) {
        node.upcast::<Node>().queue_free();
    }

    pub fn process(&mut self, delta: f32, mut node2d: Gd<Node2D>) {
        self.lerp_time = f32::min(self.lerp_time + delta, Self::LERP_DURATION_SEC);
        node2d.set_global_position(Vector2::lerp(
            self.lerp_start_position,
            self.lerp_target_position,
            self.lerp_time / Self::LERP_DURATION_SEC,
        ));

        node2d.set_scale(Vector2::lerp(
            self.target_scale,
            self.target_scale,
            delta * 8.0,
        ));
    }
}

pub fn mass_to_scale(mass: u32) -> Vector2 {
    let diameter = mass_to_diameter(mass) * 0.01;
    Vector2::new(diameter, diameter)
}

pub fn mass_to_radius(mass: u32) -> f32 {
    sqrt(mass as f64) as f32
}

pub fn mass_to_diameter(mass: u32) -> f32 {
    mass_to_radius(mass) * 2.0
}

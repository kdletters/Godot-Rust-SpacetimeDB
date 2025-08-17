use super::*;
use godot::classes::{CanvasItem, ISprite2D, Sprite2D};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Sprite2D)]
pub struct FoodController {
    base: Base<Sprite2D>,

    pub entity: EntityData,
}

const COLOR_PALETTE: &[Color] = &[
    Color::from_rgba8(119, 252, 173, 255),
    Color::from_rgba8(76, 250, 146, 255),
    Color::from_rgba8(35, 246, 120, 255),
    Color::from_rgba8(119, 251, 201, 255),
    Color::from_rgba8(76, 249, 184, 255),
    Color::from_rgba8(35, 245, 165, 255),
];

impl FoodController {
    pub fn spawn(&mut self, food: &Food) {
        self.entity
            .spawn(food.entity_id, self.base().clone().upcast::<Node2D>());
        let index = (self.entity.entity_id as usize) % COLOR_PALETTE.len();
        let canvas_item = self.base().clone().upcast::<CanvasItem>();
        self.entity.set_color(COLOR_PALETTE[index], canvas_item);
    }
}

#[godot_api]
impl ISprite2D for FoodController {
    fn process(&mut self, delta: f32) {
        let node2d = self.base().clone().upcast::<Node2D>();
        self.entity.process(delta, node2d);
    }
}
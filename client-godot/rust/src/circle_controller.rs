use super::*;
use godot::classes::{CanvasItem, ISprite2D, Label, Sprite2D};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Sprite2D)]
pub struct CircleController {
    base: Base<Sprite2D>,

    pub entity: EntityData,
    owner: Option<Gd<PlayerController>>,
}

const COLOR_PALETTE: &[Color] = &[
    //Yellow
    Color::from_rgba8(175, 159, 49, 255),
    Color::from_rgba8(175, 116, 49, 255),
    //Purple
    Color::from_rgba8(112, 47, 252, 255),
    Color::from_rgba8(51, 91, 252, 255),
    //Red
    Color::from_rgba8(176, 54, 54, 255),
    Color::from_rgba8(176, 109, 54, 255),
    Color::from_rgba8(141, 43, 99, 255),
    //Blue
    Color::from_rgba8(2, 188, 250, 255),
    Color::from_rgba8(7, 50, 251, 255),
    Color::from_rgba8(2, 28, 146, 255),
];

impl CircleController {
    pub fn spawn(&mut self, circle: Circle, owner: Gd<PlayerController>) {
        self.entity
            .spawn(circle.entity_id, self.base().clone().upcast::<Node2D>());
        let index = (self.entity.entity_id as usize) % COLOR_PALETTE.len();
        let canvas_item = self.base().clone().upcast::<CanvasItem>();
        self.entity.set_color(COLOR_PALETTE[index], canvas_item);

        self.owner = Some(owner.clone());
        self.base().get_node_as::<Label>("%NameLabel").set_text(&owner.bind().username());
    }

    pub fn on_delete(&mut self, _ctx: EventContext) {
        let node = self.base().clone().upcast::<Node>();
        self.entity.on_delete(node);

        if let Some(mut player_controller) = self.owner.clone() {
            player_controller.bind_mut().on_circle_deleted(self.to_gd());
        }
    }
}

#[godot_api]
impl ISprite2D for CircleController {
    fn process(&mut self, delta: f32) {
        let node2d = self.base().clone().upcast::<Node2D>();
        self.entity.process(delta, node2d);
    }
}

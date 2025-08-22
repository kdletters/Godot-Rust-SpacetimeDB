use super::*;
use crate::global_state::prefab_state;

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct PrefabManager {
    base: Base<Node>,

    #[export]
    circle_prefab: Option<Gd<PackedScene>>,
    #[export]
    player_prefab: Option<Gd<PackedScene>>,
}

// 预制体管理器实例现在通过全局状态管理

#[godot_api]
impl INode for PrefabManager {
    fn ready(&mut self) {
        prefab_state::set_instance(self.to_gd());
    }
}

pub fn spawn_circle(circle: Circle, mut owner: Gd<PlayerController>) -> Gd<CircleController> {
    let mut entity_controller = prefab_state::get_instance()
        .expect("PrefabManager instance not found")
        .bind()
        .circle_prefab
        .clone()
        .unwrap()
        .instantiate()
        .unwrap()
        .cast::<CircleController>();

    entity_controller
        .bind_mut()
        .base_mut()
        .set_name(&format!("Circle - {}", circle.entity_id));
    owner
        .bind_mut()
        .on_circle_spawned(entity_controller.clone());
    get_root().unwrap().add_child(&entity_controller);
    entity_controller.bind_mut().spawn(circle.clone(), owner.clone());

    entity_controller
}

// spawn_food 函数已移除，现在使用 FoodBatchRenderer 进行批量渲染

pub fn spawn_player(player: Player) -> Gd<PlayerController> {
    let mut entity_controller = prefab_state::get_instance()
        .expect("PrefabManager instance not found")
        .bind()
        .player_prefab
        .clone()
        .unwrap()
        .instantiate()
        .unwrap()
        .cast::<PlayerController>();

    entity_controller
        .bind_mut()
        .base_mut()
        .set_name(&format!("PlayerController - {}", player.name));

    entity_controller.bind_mut().initialize(player.clone());
    players::insert_player(player.player_id, entity_controller.clone());
    get_root().unwrap().add_child(&entity_controller);

    entity_controller
}

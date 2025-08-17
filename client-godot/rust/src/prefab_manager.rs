use super::*;
use tokio::sync::OnceCell;

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct PrefabManager {
    base: Base<Node>,

    #[export]
    circle_prefab: Option<Gd<PackedScene>>,
    #[export]
    food_prefab: Option<Gd<PackedScene>>,
    #[export]
    player_prefab: Option<Gd<PackedScene>>,
}

thread_local! {
    pub static PREFAB_MANAGER_INSTANCE: OnceCell<Gd<PrefabManager>> = OnceCell::const_new();
}

#[godot_api]
impl INode for PrefabManager {
    fn ready(&mut self) {
        PREFAB_MANAGER_INSTANCE
            .with(|x| x.set(self.to_gd()))
            .unwrap();
    }
}

pub fn spawn_circle(circle: Circle, mut owner: Gd<PlayerController>) -> Gd<CircleController> {
    let mut entity_controller = PREFAB_MANAGER_INSTANCE.with(|x| {
        x.get()
            .unwrap()
            .bind()
            .circle_prefab
            .clone()
            .unwrap()
            .instantiate()
            .unwrap()
            .cast::<CircleController>()
    });

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

pub fn spawn_food(food: &Food) -> Gd<FoodController> {
    let mut entity_controller = PREFAB_MANAGER_INSTANCE.with(|x| {
        x.get()
            .unwrap()
            .bind()
            .food_prefab
            .clone()
            .unwrap()
            .instantiate()
            .unwrap()
            .cast::<FoodController>()
    });

    entity_controller
        .bind_mut()
        .base_mut()
        .set_name(&format!("Food - {}", food.entity_id));

    entity_controller.bind_mut().spawn(food);
    get_root().unwrap().add_child(&entity_controller);

    entity_controller
}

pub fn spawn_player(player: Player) -> Gd<PlayerController> {
    let mut entity_controller = PREFAB_MANAGER_INSTANCE.with(|x| {
        x.get()
            .unwrap()
            .bind()
            .player_prefab
            .clone()
            .unwrap()
            .instantiate()
            .unwrap()
            .cast::<PlayerController>()
    });

    entity_controller
        .bind_mut()
        .base_mut()
        .set_name(&format!("PlayerController - {}", player.name));

    entity_controller.bind_mut().initialize(player);
    get_root().unwrap().add_child(&entity_controller);

    entity_controller
}

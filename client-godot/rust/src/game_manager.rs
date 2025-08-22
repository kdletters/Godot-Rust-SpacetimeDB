use super::*;
use crate::global_state::*;
use spacetimedb_sdk::*;

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct GameManager {
    base: Base<Node>,
}

// 全局状态现在通过 global_state 模块管理
// 不再需要 unsafe 静态变量

impl GameManager {
    const SERVER_URL: &'static str = "http://127.0.0.1:3000";
    const MODULE_NAME: &'static str = "blackholio";
}

#[godot_api]
impl INode for GameManager {
    fn process(&mut self, _delta: f64) {
        if let Some(conn) = connection::get_connection() {
            conn.frame_tick()
                .expect("Failed to process WebSocket messages");
        }
    }

    fn exit_tree(&mut self) {
        disconnect();
    }

    fn ready(&mut self) {
        Engine::singleton().set_max_fps(60);

        let builder = DbConnection::builder()
            .on_connect(handle_connect)
            .on_connect_error(handle_connect_error)
            .on_disconnect(handle_disconnect)
            .with_token(creds_store().load().expect("Failed to load credentials"))
            .with_uri(Self::SERVER_URL)
            .with_module_name(Self::MODULE_NAME);
        let conn = builder.build().unwrap();
        connection::set_connection(conn);
    }
}

fn creds_store() -> credentials::File {
    credentials::File::new("blackholio")
}

fn handle_connect(_ctx: &DbConnection, identity: Identity, token: &str) {
    godot_print!("Connected to SpacetimeDB");
    if let Err(e) = creds_store().save(token) {
        godot_error!("Failed to save credentials: {:?}", e);
    }

    identity::set_local_identity(identity);

    if let Some(conn) = connection::get_connection() {
        conn.db.circle().on_insert(circle_on_insert);
        conn.db.entity().on_update(entity_on_update);
        conn.db.entity().on_delete(entity_on_delete);
        conn.db.player().on_insert(player_on_insert);
        conn.db.player().on_delete(player_on_delete);
        conn.db.food().on_insert(food_on_insert);

        // GAME_MANAGER_INSTANCE.with(|x| x.get().unwrap().signals().on_connected().emit());

        conn.subscription_builder()
            .on_applied(handle_subscription_applied)
            .on_error(handle_subscription_error)
            .subscribe_to_all_tables();
    }
}

fn handle_connect_error(_ctx: &ErrorContext, error: Error) {
    godot_error!("Failed to connect to SpacetimeDB: {}", error);
}

fn handle_disconnect(_ctx: &ErrorContext, error: Option<Error>) {
    println!("Disconnected from SpacetimeDB");
    if let Some(error) = error {
        godot_error!("{}", error);
    }
}

fn handle_subscription_applied(ctx: &SubscriptionEventContext) {
    godot_print!("Subscription applied!");

    if let Some(conn) = connection::get_connection() {
        let world_size = conn.db.config().id().find(&0).unwrap().world_size;
        setup_arena(world_size as u32);
    };

    ctx.reducers.enter_game("3Blave".to_string()).unwrap();
}

fn handle_subscription_error(_ctx: &ErrorContext, error: Error) {
    godot_error!("Subscription error: {}", error);
}

fn disconnect() {
    if let Some(conn) = connection::get_connection() {
        conn.disconnect().unwrap();
    };

    connection::clear_connection();
}

fn setup_arena(world_size: u32) {
    WORLD_SIZE.store(world_size, std::sync::atomic::Ordering::Relaxed);
    let world_size = world_size as f32;

    let thickness = 2.0;
    create_border_cube(
        Vector2::new(world_size / 2.0, world_size + thickness / 2.0),
        Vector2::new(world_size + thickness * 2.0, thickness),
    );
    create_border_cube(
        Vector2::new(world_size / 2.0, -thickness / 2.0),
        Vector2::new(world_size + thickness * 2.0, thickness),
    );
    create_border_cube(
        Vector2::new(world_size + thickness / 2.0, world_size / 2.0),
        Vector2::new(thickness, world_size + thickness * 2.0),
    );
    create_border_cube(
        Vector2::new(-thickness / 2.0, world_size / 2.0),
        Vector2::new(thickness, world_size + thickness * 2.0),
    );
}

fn create_border_cube(pos: Vector2, size: Vector2) {
    let mut wall = load::<PackedScene>("res://prefabs/wall_prefab.tscn")
        .instantiate()
        .unwrap()
        .cast::<Node2D>();
    wall.set_name("Wall");
    wall.set_position(pos);
    wall.set_scale(size * 0.01);

    get_root().unwrap().add_child(&wall);
}

fn circle_on_insert(_ctx: &EventContext, circle: &Circle) {
    godot_print!("Circle inserted!");
    let player = get_or_create_player(circle.player_id);
    if let Some(player) = player {
        let entity = spawn_circle(circle.clone(), player);
        entities::insert_entity(circle.entity_id, EntityController::Circle(entity));
    }
}

fn entity_on_update(_ctx: &EventContext, _old_entity: &Entity, new_entity: &Entity) {
    godot_print!("Entity updated!");

    entities::update_entity(new_entity.entity_id, |entity_controller| {
        entity_controller.on_entity_updated(new_entity);
    });
}

fn entity_on_delete(_ctx: &EventContext, entity: &Entity) {
    godot_print!("Entity deleted!");
    if let Some(mut entity_controller) = entities::remove_entity(entity.entity_id) {
        entity_controller.on_delete();
    };
}

fn player_on_insert(_ctx: &EventContext, player: &Player) {
    godot_print!("Player inserted!");
    get_or_create_player(player.player_id);
}

fn player_on_delete(_ctx: &EventContext, player: &Player) {
    godot_print!("Player deleted!");

    if let Some(mut player_controller) = players::remove_player(player.player_id) {
        player_controller.bind_mut().base_mut().queue_free();
    };
}

fn food_on_insert(_ctx: &EventContext, food: &Food) {
    godot_print!("Food inserted!");
    let food_controller = spawn_food(food);
    entities::insert_entity(food.entity_id, EntityController::Food(food_controller));
}

fn get_or_create_player(player_id: u32) -> Option<Gd<PlayerController>> {
    if players::contains_player(player_id) {
        players::get_player(player_id)
    } else {
        if let Some(conn) = connection::get_connection() {
            let player = conn.db.player().player_id().find(&player_id).unwrap();
            Some(spawn_player(player))
        } else {
            None
        }
    }
}

#[godot_api]
impl GameManager {
    #[signal]
    fn on_connected();

    #[signal]
    fn on_subscription_applied();
}

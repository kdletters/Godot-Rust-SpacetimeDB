use super::*;
use crate::global_state::*;
use crate::global_state::food_batch_renderer;
use crate::camera_controller::WORLD_SIZE;
use godot::classes::Engine;
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
    
    /// 初始化食物批量渲染器
    fn setup_food_batch_renderer(&mut self) {
        // 创建食物批量渲染器实例
        let mut food_renderer = FoodBatchRenderer::new_alloc();
        
        // 设置名称以便调试
        food_renderer.set_name("FoodBatchRenderer");
        
        // 设置为全屏大小（覆盖整个游戏世界）
        food_renderer.set_anchor(godot::builtin::Side::LEFT, 0.0);
        food_renderer.set_anchor(godot::builtin::Side::TOP, 0.0);
        food_renderer.set_anchor(godot::builtin::Side::RIGHT, 1.0);
        food_renderer.set_anchor(godot::builtin::Side::BOTTOM, 1.0);
        
        // 将渲染器添加到游戏世界中
        if let Some(mut root) = get_root() {
            root.call_deferred("add_child", &[food_renderer.to_variant()]);
            godot_print!("FoodBatchRenderer added to scene tree");
        } else {
            godot_error!("Failed to get root node for FoodBatchRenderer");
        }
        
        // 将实例注册到全局状态
        food_batch_renderer::set_instance(food_renderer);
        
        godot_print!("FoodBatchRenderer setup completed");
    }
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

        // 初始化食物批量渲染器
        self.setup_food_batch_renderer();

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

    // 检查是否是食物实体，如果是则使用批量渲染器处理
    if food_batch_renderer::is_food_entity(new_entity.entity_id) {
        if let Some(mut batch_renderer) = food_batch_renderer::get_instance() {
            batch_renderer.bind_mut().update_food_entity(new_entity);
        }
        return;
    }

    // 其他实体的处理保持不变
    entities::update_entity(new_entity.entity_id, |entity_controller| {
        entity_controller.on_entity_updated(new_entity);
    });
}

fn entity_on_delete(_ctx: &EventContext, entity: &Entity) {
    godot_print!("Entity deleted!");
    
    // 检查是否是食物实体，如果是则从批量渲染器中移除
    if food_batch_renderer::is_food_entity(entity.entity_id) {
        if let Some(mut batch_renderer) = food_batch_renderer::get_instance() {
            batch_renderer.bind_mut().remove_food(entity.entity_id);
        }
        return;
    }
    
    // 其他实体的处理保持不变
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
    godot_print!("Food inserted! entity_id: {}", food.entity_id);
    
    // 不再创建独立节点，而是添加到批量渲染器
    match food_batch_renderer::get_instance() {
        Some(mut batch_renderer) => {
            godot_print!("Adding food {} to batch renderer", food.entity_id);
            batch_renderer.bind_mut().add_food(food);
            
            // 获取当前食物数量
            let food_count = batch_renderer.bind().get_food_count();
            godot_print!("Total foods in batch renderer: {}", food_count);
        },
        None => {
            godot_error!("FoodBatchRenderer instance not found! Creating new instance...");
            
            // 如果没有实例，尝试创建一个
            let mut food_renderer = FoodBatchRenderer::new_alloc();
            food_renderer.set_name("FoodBatchRenderer");
            
            // 添加到场景中
            if let Some(mut root) = get_root() {
                root.add_child(&food_renderer);
                food_batch_renderer::set_instance(food_renderer.clone());
                
                // 现在添加食物
                food_renderer.bind_mut().add_food(food);
                godot_print!("Created new FoodBatchRenderer and added food {}", food.entity_id);
            } else {
                godot_error!("Failed to get root node when creating FoodBatchRenderer");
            }
        }
    }
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

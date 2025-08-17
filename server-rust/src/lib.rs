mod math;

use math::*;

use log::{debug, info};
use spacetimedb::{
    rand::Rng, Identity, ReducerContext, ScheduleAt, SpacetimeType, Table, Timestamp,
};
use std::time::Duration;

#[spacetimedb::table(name = spawn_food_timer, scheduled(spawn_food))]
pub struct SpawnFoodTimer {
    #[primary_key]
    #[auto_inc]
    scheduled_id: u64,
    scheduled_at: spacetimedb::ScheduleAt,
}

// We're using this table as a singleton, so in this table
// there only be one element where the `id` is 0.
#[spacetimedb::table(name = config, public)]
pub struct Config {
    #[primary_key]
    pub id: u32,
    pub world_size: u64,
}

#[spacetimedb::table(name = entity, public)]
#[derive(Debug, Clone)]
pub struct Entity {
    // The `auto_inc` attribute indicates to SpacetimeDB that
    // this value should be determined by SpacetimeDB on insert.
    #[auto_inc]
    #[primary_key]
    pub entity_id: u32,
    pub position: DbVector2,
    pub mass: u32,
}

#[spacetimedb::table(name = circle, public)]
pub struct Circle {
    #[primary_key]
    pub entity_id: u32,
    #[index(btree)]
    pub player_id: u32,
    pub direction: DbVector2,
    pub speed: f32,
    pub last_split_time: Timestamp,
}

#[spacetimedb::table(name = food, public)]
pub struct Food {
    #[primary_key]
    pub entity_id: u32,
}

#[spacetimedb::table(name = player, public)]
#[spacetimedb::table(name = logged_out_player)]
#[derive(Debug, Clone)]
pub struct Player {
    #[primary_key]
    identity: Identity,
    #[unique]
    #[auto_inc]
    player_id: u32,
    name: String,
}

// Note the `init` parameter passed to the reducer macro.
// That indicates to SpacetimeDB that it should be called
#[spacetimedb::reducer(init)]
pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    log::info!("Initializing...");
    ctx.db.config().try_insert(Config {
        id: 0,
        world_size: 1000,
    })?;
    ctx.db.spawn_food_timer().try_insert(SpawnFoodTimer {
        scheduled_id: 0,
        scheduled_at: ScheduleAt::Interval(Duration::from_millis(500).into()),
    })?;
    ctx.db
        .move_all_players_timer()
        .try_insert(MoveAllPlayersTimer {
            scheduled_id: 0,
            scheduled_at: ScheduleAt::Interval(Duration::from_millis(50).into()),
        })?;
    Ok(())
}

#[spacetimedb::reducer(client_connected)]
pub fn connect(ctx: &ReducerContext) -> Result<(), String> {
    // Check if the player was previously logged out
    if let Some(player) = ctx.db.logged_out_player().identity().find(&ctx.sender) {
        ctx.db.player().insert(player.clone());
        ctx.db
            .logged_out_player()
            .identity()
            .delete(&player.identity);

        // Log connection with existing player
        let str = if player.name.is_empty() {
            ctx.sender.to_string()
        } else {
            player.name
        };
        log::info!("Player reconnected: {}", str);
    } else {
        // Create a new player with empty name
        ctx.db.player().try_insert(Player {
            identity: ctx.sender,
            player_id: 0,
            name: String::new(),
        })?;

        log::info!("New player connected with identity: {:?}", ctx.sender);
    }

    Ok(())
}

#[spacetimedb::reducer(client_disconnected)]
pub fn disconnect(ctx: &ReducerContext) -> Result<(), String> {
    let player = ctx
        .db
        .player()
        .identity()
        .find(&ctx.sender)
        .ok_or("Player not found")?;
    let player_id = player.player_id;
    ctx.db.logged_out_player().insert(player);
    ctx.db.player().identity().delete(&ctx.sender);

    // Remove any circles from the arena
    for circle in ctx.db.circle().player_id().filter(&player_id) {
        ctx.db.entity().entity_id().delete(&circle.entity_id);
        ctx.db.circle().entity_id().delete(&circle.entity_id);
    }

    Ok(())
}

#[spacetimedb::reducer]
pub fn debug(ctx: &ReducerContext) -> Result<(), String> {
    log::debug!("This reducer was called by {}.", ctx.sender);
    Ok(())
}

const FOOD_MASS_MIN: u32 = 2;
const FOOD_MASS_MAX: u32 = 4;
const TARGET_FOOD_COUNT: usize = 600;

fn mass_to_radius(mass: u32) -> f32 {
    (mass as f32).sqrt()
}

#[spacetimedb::reducer]
pub fn spawn_food(ctx: &ReducerContext, _timer: SpawnFoodTimer) -> Result<(), String> {
    if ctx.db.player().count() == 0 {
        // Are there no logged in players? Skip food spawn.
        return Ok(());
    }

    let world_size = ctx
        .db
        .config()
        .id()
        .find(0)
        .ok_or("Config not found")?
        .world_size;

    let mut rng = ctx.rng();
    let mut food_count = ctx.db.food().count();
    while food_count < TARGET_FOOD_COUNT as u64 {
        let food_mass = rng.gen_range(FOOD_MASS_MIN..FOOD_MASS_MAX);
        let food_radius = mass_to_radius(food_mass);
        let x = rng.gen_range(food_radius..world_size as f32 - food_radius);
        let y = rng.gen_range(food_radius..world_size as f32 - food_radius);
        let entity = ctx.db.entity().try_insert(Entity {
            entity_id: 0,
            position: DbVector2 { x, y },
            mass: food_mass,
        })?;
        ctx.db.food().try_insert(Food {
            entity_id: entity.entity_id,
        })?;
        food_count += 1;
        log::info!("Spawned food! {}", entity.entity_id);
    }

    Ok(())
}

const START_PLAYER_MASS: u32 = 15;

#[spacetimedb::reducer]
pub fn enter_game(ctx: &ReducerContext, name: String) -> Result<(), String> {
    log::info!("Creating player with name {}", name);

    // Get the player
    let mut player: Player = ctx
        .db
        .player()
        .identity()
        .find(ctx.sender)
        .ok_or("Player not found")?;
    let player_id = player.player_id;

    // Update player name
    player.name = name.clone();
    ctx.db.player().identity().update(player);

    // Spawn the player's initial circle
    spawn_player_initial_circle(ctx, player_id)?;

    Ok(())
}

fn spawn_player_initial_circle(ctx: &ReducerContext, player_id: u32) -> Result<Entity, String> {
    let mut rng = ctx.rng();
    let world_size = ctx
        .db
        .config()
        .id()
        .find(&0)
        .ok_or("Config not found")?
        .world_size;
    let player_start_radius = mass_to_radius(START_PLAYER_MASS);
    let x = rng.gen_range(player_start_radius..(world_size as f32 - player_start_radius));
    let y = rng.gen_range(player_start_radius..(world_size as f32 - player_start_radius));
    spawn_circle_at(
        ctx,
        player_id,
        START_PLAYER_MASS,
        DbVector2 { x, y },
        ctx.timestamp,
    )
}

fn spawn_circle_at(
    ctx: &ReducerContext,
    player_id: u32,
    mass: u32,
    position: DbVector2,
    timestamp: Timestamp,
) -> Result<Entity, String> {
    let entity = ctx.db.entity().try_insert(Entity {
        entity_id: 0,
        position,
        mass,
    })?;

    ctx.db.circle().try_insert(Circle {
        entity_id: entity.entity_id,
        player_id,
        direction: DbVector2 { x: 0.0, y: 1.0 },
        speed: 0.0,
        last_split_time: timestamp,
    })?;
    Ok(entity)
}

#[spacetimedb::reducer]
pub fn update_player_input(ctx: &ReducerContext, direction: DbVector2) -> Result<(), String> {
    let player = ctx
        .db
        .player()
        .identity()
        .find(&ctx.sender)
        .ok_or("Player not found")?;
    for mut circle in ctx.db.circle().player_id().filter(&player.player_id) {
        circle.direction = direction.normalized();
        circle.speed = direction.magnitude().clamp(0.0, 1.0);
        ctx.db.circle().entity_id().update(circle);
    }
    Ok(())
}

#[spacetimedb::table(name = move_all_players_timer, scheduled(move_all_players))]
pub struct MoveAllPlayersTimer {
    #[primary_key]
    #[auto_inc]
    scheduled_id: u64,
    scheduled_at: spacetimedb::ScheduleAt,
}

const START_PLAYER_SPEED: u32 = 10;

fn mass_to_max_move_speed(mass: u32) -> f32 {
    2.0 * START_PLAYER_SPEED as f32 / (1.0 + (mass as f32 / START_PLAYER_MASS as f32).sqrt())
}
const MINIMUM_SAFE_MASS_RATIO: f32 = 0.85;

fn is_overlapping(a: &Entity, b: &Entity) -> bool {
    let dx = a.position.x - b.position.x;
    let dy = a.position.y - b.position.y;
    let distance_sq = dx * dx + dy * dy;

    let radius_a = mass_to_radius(a.mass);
    let radius_b = mass_to_radius(b.mass);

    // If the distance between the two circle centers is less than the
    // maximum radius, then the center of the smaller circle is inside
    // the larger circle. This gives some leeway for the circles to overlap
    // before being eaten.
    let max_radius = f32::max(radius_a, radius_b);
    distance_sq <= max_radius * max_radius
}

#[spacetimedb::reducer]
pub fn move_all_players(ctx: &ReducerContext, _timer: MoveAllPlayersTimer) -> Result<(), String> {
    let world_size = ctx
        .db
        .config()
        .id()
        .find(0)
        .ok_or("Config not found")?
        .world_size;

    // Handle player input
    for circle in ctx.db.circle().iter() {
        let circle_entity = ctx.db.entity().entity_id().find(&circle.entity_id);
        if !circle_entity.is_some() {
            // This can happen if a circle is eaten by another circle
            continue;
        }
        let mut circle_entity = circle_entity.unwrap();
        let circle_radius = mass_to_radius(circle_entity.mass);
        let direction = circle.direction * circle.speed;
        let new_pos =
            circle_entity.position + direction * mass_to_max_move_speed(circle_entity.mass);
        let min = circle_radius;
        let max = world_size as f32 - circle_radius;
        circle_entity.position.x = new_pos.x.clamp(min, max);
        circle_entity.position.y = new_pos.y.clamp(min, max);

        // Check collisions
        for entity in ctx.db.entity().iter() {
            if entity.entity_id == circle_entity.entity_id {
                continue;
            }
            if is_overlapping(&circle_entity, &entity) {
                // Check to see if we're overlapping with food
                if ctx.db.food().entity_id().find(&entity.entity_id).is_some() {
                    ctx.db.entity().entity_id().delete(&entity.entity_id);
                    ctx.db.food().entity_id().delete(&entity.entity_id);
                    circle_entity.mass += entity.mass;
                }

                // Check to see if we're overlapping with another circle owned by another player
                let other_circle = ctx.db.circle().entity_id().find(&entity.entity_id);
                if let Some(other_circle) = other_circle {
                    if other_circle.player_id != circle.player_id {
                        let mass_ratio = entity.mass as f32 / circle_entity.mass as f32;
                        if mass_ratio < MINIMUM_SAFE_MASS_RATIO {
                            ctx.db.entity().entity_id().delete(&entity.entity_id);
                            ctx.db.circle().entity_id().delete(&entity.entity_id);
                            circle_entity.mass += entity.mass;
                        }
                    }
                }
            }
        }
        ctx.db.entity().entity_id().update(circle_entity);
    }

    Ok(())
}

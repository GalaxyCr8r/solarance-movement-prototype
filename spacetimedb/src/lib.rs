#[path = "../../solarance-shared/src/physics.rs"]
mod physics;

use crate::physics::{predict_movement, rotation_to_vector, MovementState, Vec2};
use spacetimedb::{log, reducer, table, Identity, ReducerContext, Table};
use spacetimedsl::Timestamp;

#[table(name = ship_stats, public)]
pub struct ShipStats {
    #[primary_key]
    pub ship_config_id: u32,
    pub max_speed: f32,     // meters per second
    pub max_turn_rate: f32, // degrees per second
}

#[table(name = player_ship, public)]
pub struct PlayerShip {
    #[primary_key]
    pub entity_id: Identity,
    pub ship_config_id: u32,
    pub movement: MovementState,
}

#[reducer(init)]
pub fn init(ctx: &ReducerContext) {
    // Seed initial ship types
    ctx.db.ship_stats().insert(ShipStats {
        ship_config_id: 1,
        max_speed: 50.0,
        max_turn_rate: 180.0,
    });
}

#[reducer(client_connected)]
pub fn on_connect(ctx: &ReducerContext) {
    // Spawn a ship for the player if they don't have one
    if ctx.db.player_ship().entity_id().find(&ctx.sender).is_none() {
        ctx.db.player_ship().insert(PlayerShip {
            entity_id: ctx.sender,
            ship_config_id: 1,
            movement: MovementState {
                pos: Vec2 { x: 0.0, y: 0.0 },
                velocity: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
                angular_velocity: 0.0,
                last_update_time: ctx.timestamp().to_micros_since_unix_epoch(),
            },
        });
    }
}

#[reducer]
pub fn set_forward_thrust(ctx: &ReducerContext, meters_per_second: f32) -> Result<(), String> {
    let mut player_ship = ctx
        .db
        .player_ship()
        .entity_id()
        .find(&ctx.sender)
        .ok_or("Ship not found")?;

    let stats = ctx
        .db
        .ship_stats()
        .ship_config_id()
        .find(&player_ship.ship_config_id)
        .ok_or("Ship stats not found")?;

    // 1. Enforce Server-Side Speed Limits
    let clamped_speed = meters_per_second.clamp(0.0, stats.max_speed);

    // 2. Synchronize current position BEFORE changing trajectory
    let (current_pos, current_rot) =
        predict_movement(&player_ship.movement, ctx.timestamp.unix_micros());

    // 3. Update the movement state
    // In Asteroids, thrust adds to the vector. In EV, it's often direct heading velocity.
    // Here we calculate the new velocity vector based on current rotation.
    let dir = rotation_to_vector(current_rot);
    let new_velocity = Vec2 {
        x: dir.x * clamped_speed,
        y: dir.y * clamped_speed,
    };

    player_ship.movement = MovementState {
        pos: current_pos,
        velocity: new_velocity,
        rotation: current_rot,
        angular_velocity: player_ship.movement.angular_velocity,
        last_update_time: ctx.timestamp.unix_micros(),
    };

    // 4. Update Database
    ctx.db.player_ship().entity_id().update(player_ship);
    Ok(())
}

#[reducer]
pub fn set_turn_velocity(ctx: &ReducerContext, degrees_per_second: f32) -> Result<(), String> {
    let mut player_ship = ctx
        .db
        .player_ship()
        .entity_id()
        .find(&ctx.sender)
        .ok_or("Ship not found")?;

    let stats = ctx
        .db
        .ship_stats()
        .ship_config_id()
        .find(&player_ship.ship_config_id)
        .ok_or("Ship stats not found")?;

    // 1. Enforce Turn Limits
    let clamped_turn = degrees_per_second.clamp(-stats.max_turn_rate, stats.max_turn_rate);

    // 2. Synchronize current position/rotation
    let (current_pos, current_rot) =
        predict_movement(&player_ship.movement, ctx.timestamp.unix_micros());

    // 3. Update trajectory
    player_ship.movement = MovementState {
        pos: current_pos,
        velocity: player_ship.movement.velocity,
        rotation: current_rot,
        angular_velocity: clamped_turn,
        last_update_time: ctx.timestamp.unix_micros(),
    };

    ctx.db.player_ship().entity_id().update(player_ship);
    Ok(())
}

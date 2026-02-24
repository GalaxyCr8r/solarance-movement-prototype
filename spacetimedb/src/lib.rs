use solarance_shared::physics::predict_movement;
use spacetimedb::{reducer, table, Identity, ReducerContext, Table};
use spacetimedsl::Timestamp;

mod physics;
use physics::*;

#[table(name = ship_config, public)]
pub struct ShipConfig {
    #[primary_key]
    pub ship_config_id: u32,
    pub max_speed: f32,     // meters per second
    pub max_turn_rate: f32, // degrees per second
}

#[table(name = space_ship, public)]
pub struct SpaceShip {
    #[primary_key]
    pub entity_id: Identity,
    pub ship_config_id: u32,
    pub movement: physics::MovementState,
}

#[reducer(init)]
pub fn init(ctx: &ReducerContext) {
    // Seed initial ship types
    ctx.db.ship_config().insert(ShipConfig {
        ship_config_id: 1,
        max_speed: 50.0,
        max_turn_rate: 45.0,
    });
}

#[reducer(client_connected)]
pub fn on_connect(ctx: &ReducerContext) {
    // Spawn a ship for the player if they don't have one
    if ctx.db.space_ship().entity_id().find(&ctx.sender).is_none() {
        ctx.db.space_ship().insert(SpaceShip {
            entity_id: ctx.sender,
            ship_config_id: 1,
            movement: MovementState {
                pos: Vec2 { x: 0.0, y: 0.0 },
                velocity: 0.0,
                rotation: 0.0,
                angular_velocity: 0.0,
                last_update_time: ctx.timestamp().to_micros_since_unix_epoch(),
            },
        });
    }
}

#[reducer]
pub fn set_forward_thrust(ctx: &ReducerContext, meters_per_second: f32) -> Result<(), String> {
    let mut space_ship = ctx
        .db
        .space_ship()
        .entity_id()
        .find(&ctx.sender)
        .ok_or("Ship not found")?;

    let stats = ctx
        .db
        .ship_config()
        .ship_config_id()
        .find(&space_ship.ship_config_id)
        .ok_or("Ship stats not found")?;

    // 1. Enforce Server-Side Speed Limits
    let clamped_speed = meters_per_second.clamp(0.0, stats.max_speed);
    if clamped_speed == space_ship.movement.velocity {
        return Ok(());
    }

    // 2. Synchronize current position BEFORE changing trajectory
    let (current_pos, current_rot) = predict_movement(
        &convert_to_movement_state(&space_ship.movement),
        ctx.timestamp.to_micros_since_unix_epoch(),
    );

    // 3. Update the movement state
    // In Asteroids, thrust adds to the vector. In EV, it's often direct heading velocity.
    // Here we calculate the new velocity vector based on current rotation.
    // let dir = rotation_to_vector(current_rot);
    // let new_velocity = Vec2 {
    //     x: dir.x * clamped_speed,
    //     y: dir.y * clamped_speed,
    // };

    space_ship.movement = MovementState {
        pos: Vec2 {
            x: current_pos.x,
            y: current_pos.y,
        },
        velocity: clamped_speed,
        rotation: current_rot,
        angular_velocity: space_ship.movement.angular_velocity,
        last_update_time: ctx.timestamp.to_micros_since_unix_epoch(),
    };

    // 4. Update Database
    ctx.db.space_ship().entity_id().update(space_ship);
    Ok(())
}

#[reducer]
pub fn set_turn_velocity(ctx: &ReducerContext, degrees_per_second: f32) -> Result<(), String> {
    let mut space_ship = ctx
        .db
        .space_ship()
        .entity_id()
        .find(&ctx.sender)
        .ok_or("Ship not found")?;

    let stats = ctx
        .db
        .ship_config()
        .ship_config_id()
        .find(&space_ship.ship_config_id)
        .ok_or("Ship stats not found")?;

    // 1. Enforce Turn Limits
    let mut clamped_turn = degrees_per_second.clamp(-stats.max_turn_rate, stats.max_turn_rate);
    if clamped_turn.abs() < 0.25 {
        clamped_turn = 0.0;
    }

    if clamped_turn == space_ship.movement.angular_velocity {
        return Ok(());
    }

    // 2. Synchronize current position/rotation
    let (current_pos, current_rot) = predict_movement(
        &convert_to_movement_state(&space_ship.movement),
        ctx.timestamp.to_micros_since_unix_epoch(),
    );

    // 3. Update trajectory
    space_ship.movement = MovementState {
        pos: Vec2 {
            x: current_pos.x,
            y: current_pos.y,
        },
        velocity: space_ship.movement.velocity,
        rotation: current_rot,
        angular_velocity: clamped_turn,
        last_update_time: ctx.timestamp.to_micros_since_unix_epoch(),
    };

    ctx.db.space_ship().entity_id().update(space_ship);
    Ok(())
}

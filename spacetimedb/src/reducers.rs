use solarance_shared::physics::predict_movement;
use spacetimedb::*;
use spacetimedsl::*;

use crate::{physics::*, tables::*};

#[reducer]
pub fn spawn_ship(ctx: &ReducerContext) -> Result<(), String> {
    // Spawn a ship for the player if they don't have one
    if ctx
        .db
        .space_ship()
        .entity_id()
        .find(&ctx.sender())
        .is_none()
    {
        // Get ship configuration to copy max_speed and max_turn_rate
        let config = ctx
            .db
            .ship_config()
            .ship_config_id()
            .find(&1)
            .expect("Default ship config not found");

        ctx.db.space_ship().try_insert(SpaceShip {
            entity_id: ctx.sender(),
            ship_config_id: 1,
            sector_id: 1,
            movement: MovementState {
                pos: Vec2 { x: 0.0, y: 0.0 },
                velocity: 0.0,
                rotation: 0.0,
                angular_velocity: 0.0,
                last_update_time: ctx.timestamp.to_micros_since_unix_epoch(),
                acceleration: 0.0,
                angular_acceleration: 0.0,
                max_speed: config.max_speed,
                max_turn_rate: config.max_turn_rate,
            },
            input_state: InputState {
                is_thrusting: false,
                is_breaking: false,
                turn_direction: 0,
            },
        })?;

        ctx.db.player_state().try_insert(PlayerState {
            player_id: ctx.sender(),
            current_system_id: 1,
            current_sector_id: 1,
        })?;

        ctx.db.visited_sectors().try_insert(VisitedSector {
            id: 0,
            player_id: ctx.sender(),
            sector_id: 1,
            visited_status: VisitedStatus::Visited,
        })?;

        ctx.db.visited_systems().try_insert(VisitedSystem {
            id: 0,
            player_id: ctx.sender(),
            system_id: 1,
            visited_status: VisitedStatus::Visited,
        })?;
    }

    Ok(())
}

#[reducer]
pub fn travel_to_sector(ctx: &ReducerContext, sector_id: u64) -> Result<(), String> {
    let mut space_ship = ctx
        .db
        .space_ship()
        .entity_id()
        .find(&ctx.sender())
        .ok_or("Ship not found")?;

    let mut player_state = ctx
        .db
        .player_state()
        .player_id()
        .find(&ctx.sender())
        .ok_or("player_state not found")?;

    let target_sector = ctx
        .db
        .sectors()
        .id()
        .find(&sector_id)
        .ok_or("Sector not found")?;

    let current_sector = ctx
        .db
        .sectors()
        .id()
        .find(&space_ship.sector_id)
        .ok_or("Current sector not found")?;

    if current_sector.system_id != target_sector.system_id {
        return Err("Cannot travel between systems".to_string());
    }

    if !ctx
        .db
        .visited_sectors()
        .player_id()
        .filter(ctx.sender())
        .any(|v| v.sector_id == sector_id)
    {
        ctx.db.visited_sectors().try_insert(VisitedSector {
            id: 0,
            player_id: ctx.sender(),
            sector_id: sector_id,
            visited_status: VisitedStatus::Visited,
        })?;
    }

    space_ship.sector_id = sector_id;
    player_state.current_sector_id = sector_id;
    ctx.db.space_ship().entity_id().update(space_ship);
    ctx.db.player_state().player_id().update(player_state);
    Ok(())
}

#[reducer]
pub fn set_forward_thrust(ctx: &ReducerContext, meters_per_second: f32) -> Result<(), String> {
    let mut space_ship = ctx
        .db
        .space_ship()
        .entity_id()
        .find(&ctx.sender())
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
    space_ship.movement = MovementState {
        pos: Vec2 {
            x: current_pos.x,
            y: current_pos.y,
        },
        velocity: clamped_speed,
        rotation: current_rot,
        angular_velocity: space_ship.movement.angular_velocity,
        last_update_time: ctx.timestamp.to_micros_since_unix_epoch(),
        acceleration: space_ship.movement.acceleration,
        angular_acceleration: space_ship.movement.angular_acceleration,
        max_speed: space_ship.movement.max_speed,
        max_turn_rate: space_ship.movement.max_turn_rate,
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
        .find(&ctx.sender())
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
        acceleration: space_ship.movement.acceleration,
        angular_acceleration: space_ship.movement.angular_acceleration,
        max_speed: space_ship.movement.max_speed,
        max_turn_rate: space_ship.movement.max_turn_rate,
    };

    ctx.db.space_ship().entity_id().update(space_ship);
    Ok(())
}

#[reducer]
pub fn set_thrust_input(
    ctx: &ReducerContext,
    is_thrusting: bool,
    is_breaking: bool,
) -> Result<(), String> {
    let mut space_ship = ctx
        .db
        .space_ship()
        .entity_id()
        .find(&ctx.sender())
        .ok_or("Ship not found")?;

    // Early return if input hasn't changed (Req 3.8)
    if is_thrusting {
        if space_ship.input_state.is_thrusting == is_thrusting
            && space_ship.input_state.is_breaking == false
        {
            return Ok(());
        }
    } else if is_breaking {
        if space_ship.input_state.is_thrusting == false
            && space_ship.input_state.is_breaking == is_breaking
        {
            return Ok(());
        }
    } else if space_ship.input_state.is_thrusting == space_ship.input_state.is_breaking {
        return Ok(());
    }

    let config = ctx
        .db
        .ship_config()
        .ship_config_id()
        .find(&space_ship.ship_config_id)
        .ok_or("Ship config not found")?;

    let now = ctx.timestamp.to_micros_since_unix_epoch();
    let dt = (now - space_ship.movement.last_update_time) as f32 / 1_000_000.0;

    // 1. Predict current position and rotation
    let (predicted_pos, predicted_rot) =
        predict_movement(&convert_to_movement_state(&space_ship.movement), now);

    // 2. Calculate predicted velocities: v = v₀ + a*dt, clamped
    let predicted_velocity = (space_ship.movement.velocity + space_ship.movement.acceleration * dt)
        .clamp(0.0, config.max_speed);
    let predicted_angular_velocity = (space_ship.movement.angular_velocity
        + space_ship.movement.angular_acceleration * dt)
        .clamp(-config.max_turn_rate, config.max_turn_rate);

    // 3. Calculate new acceleration based on thrust input
    let new_acceleration = if is_thrusting {
        config.max_acceleration
    } else if is_breaking {
        -config.max_acceleration
    } else {
        0.0 // Ship coasts at current velocity
    };

    // 4. Update input state and movement
    space_ship.input_state.is_thrusting = is_thrusting;
    space_ship.input_state.is_breaking = if !is_thrusting { is_breaking } else { false };
    space_ship.movement = MovementState {
        pos: Vec2 {
            x: predicted_pos.x,
            y: predicted_pos.y,
        },
        velocity: predicted_velocity,
        rotation: predicted_rot,
        angular_velocity: predicted_angular_velocity,
        acceleration: new_acceleration,
        angular_acceleration: space_ship.movement.angular_acceleration,
        last_update_time: now,
        max_speed: config.max_speed,
        max_turn_rate: config.max_turn_rate,
    };

    ctx.db.space_ship().entity_id().update(space_ship);
    Ok(())
}

#[reducer]
pub fn set_turn_input(ctx: &ReducerContext, turn_direction: i8) -> Result<(), String> {
    // Validate turn_direction
    if turn_direction != -1 && turn_direction != 0 && turn_direction != 1 {
        return Err(format!(
            "Invalid turn_direction: {}. Must be -1, 0, or 1",
            turn_direction
        ));
    }

    let mut space_ship = ctx
        .db
        .space_ship()
        .entity_id()
        .find(&ctx.sender())
        .ok_or("Ship not found")?;

    // Early return if input hasn't changed (Req 3.8)
    if space_ship.input_state.turn_direction == turn_direction {
        return Ok(());
    }

    let config = ctx
        .db
        .ship_config()
        .ship_config_id()
        .find(&space_ship.ship_config_id)
        .ok_or("Ship config not found")?;

    let now = ctx.timestamp.to_micros_since_unix_epoch();
    let dt = (now - space_ship.movement.last_update_time) as f32 / 1_000_000.0;

    // 1. Predict current position and rotation
    let (predicted_pos, predicted_rot) =
        predict_movement(&convert_to_movement_state(&space_ship.movement), now);

    // 2. Calculate predicted velocities: v = v₀ + a*dt, clamped
    let predicted_velocity = (space_ship.movement.velocity + space_ship.movement.acceleration * dt)
        .clamp(0.0, config.max_speed);
    let predicted_angular_velocity = (space_ship.movement.angular_velocity
        + space_ship.movement.angular_acceleration * dt)
        .clamp(-config.max_turn_rate, config.max_turn_rate);

    // 3. Calculate new angular acceleration based on turn direction
    let new_angular_acceleration = turn_direction as f32 * config.max_angular_acceleration;

    // 4. Update input state and movement
    space_ship.input_state.turn_direction = turn_direction;
    space_ship.movement = MovementState {
        pos: Vec2 {
            x: predicted_pos.x,
            y: predicted_pos.y,
        },
        velocity: predicted_velocity,
        rotation: predicted_rot,
        angular_velocity: predicted_angular_velocity,
        acceleration: space_ship.movement.acceleration,
        angular_acceleration: new_angular_acceleration,
        last_update_time: now,
        max_speed: config.max_speed,
        max_turn_rate: config.max_turn_rate,
    };

    ctx.db.space_ship().entity_id().update(space_ship);
    Ok(())
}

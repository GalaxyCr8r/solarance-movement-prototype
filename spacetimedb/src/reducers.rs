use solarance_shared::physics::predict_movement;
use spacetimedb::*;
use spacetimedsl::*;

use crate::{physics::*, sectors::observe_all_public_sectors, tables::*};

#[reducer]
pub fn spawn_ship(ctx: &ReducerContext) -> Result<(), String> {
    let dsl = spacetimedsl::dsl(ctx);

    // Spawn a ship for the player if they don't have one
    if dsl
        .get_space_ship_by_id(SpaceShipId::new(ctx.sender()))
        .is_err()
    {
        // Get ship configuration to copy max_speed and max_turn_rate
        let config = dsl
            .get_ship_config_by_id(ShipConfigId::new(1))
            .expect("Default ship config not found");

        dsl.create_space_ship(CreateSpaceShip {
            id: ctx.sender(),
            ship_config_id: config.get_id().clone(),
            health: *config.get_max_health() as f32,
            sector_id: SectorId::new(1),
            movement: MovementState {
                pos: Vec2 { x: 0.0, y: 0.0 },
                velocity: 0.0,
                rotation: 0.0,
                angular_velocity: 0.0,
                last_update_time: ctx.timestamp.to_micros_since_unix_epoch(),
                acceleration: 0.0,
                angular_acceleration: 0.0,
                max_speed: *config.get_max_speed(),
                max_turn_rate: *config.get_max_turn_rate(),
            },
            input_state: InputState {
                is_thrusting: false,
                is_breaking: false,
                turn_direction: 0,
            },
        })?;

        dsl.create_player_state(CreatePlayerState {
            id: ctx.sender(),
            current_system_id: 1,
            current_sector_id: 1,
        })?;

        dsl.create_visited_sector(CreateVisitedSector {
            player_id: ctx.sender(),
            sector_id: 1,
            visited_status: VisitedStatus::Visited,
        })?;

        observe_all_public_sectors(ctx)?;

        dsl.create_visited_system(CreateVisitedSystem {
            player_id: ctx.sender(),
            system_id: 1,
            visited_status: VisitedStatus::Visited,
        })?;
    }

    Ok(())
}

#[reducer]
pub fn travel_to_sector(ctx: &ReducerContext, sector_id: u64) -> Result<(), String> {
    let dsl = spacetimedsl::dsl(ctx);

    let mut space_ship = dsl
        .get_space_ship_by_id(SpaceShipId::new(ctx.sender()))
        .map_err(|_| "Ship not found")?;

    let mut player_state = dsl
        .get_player_state_by_id(PlayerStateId::new(ctx.sender()))
        .map_err(|_| "player_state not found")?;

    let target_sector = dsl
        .get_sector_by_id(SectorId::new(sector_id))
        .map_err(|_| "Sector not found")?;

    let current_sector = dsl
        .get_sector_by_id(space_ship.get_sector_id().clone())
        .map_err(|_| "Current sector not found")?;

    if current_sector.get_system_id() != target_sector.get_system_id() {
        return Err("Cannot travel between systems".to_string());
    }

    if !dsl
        .get_visited_sectors_by_player_id(&ctx.sender())
        .any(|v| *v.get_sector_id() == sector_id)
    {
        dsl.create_visited_sector(CreateVisitedSector {
            player_id: ctx.sender(),
            sector_id: sector_id,
            visited_status: VisitedStatus::Visited,
        });
    }

    space_ship.sector_id = sector_id;
    player_state.current_sector_id = sector_id;
    dsl.update_space_ship_by_id(space_ship);
    dsl.update_player_state_by_id(player_state);
    Ok(())
}

#[reducer]
pub fn set_forward_thrust(ctx: &ReducerContext, meters_per_second: f32) -> Result<(), String> {
    let dsl = spacetimedsl::dsl(ctx);

    let mut space_ship = dsl
        .get_space_ship_by_id(SpaceShipId::new(ctx.sender()))
        .map_err(|_| "Ship not found")?;

    let stats = dsl
        .get_ship_config_by_id(ShipConfigId::new(space_ship.ship_config_id))
        .map_err(|_| "Ship stats not found")?;

    // 1. Enforce Server-Side Speed Limits
    let clamped_speed = meters_per_second.clamp(0.0, *stats.get_max_speed());
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
    dsl.update_space_ship_by_id(space_ship);
    Ok(())
}

#[reducer]
pub fn set_turn_velocity(ctx: &ReducerContext, degrees_per_second: f32) -> Result<(), String> {
    let dsl = spacetimedsl::dsl(ctx);

    let mut space_ship = dsl
        .get_space_ship_by_id(SpaceShipId::new(ctx.sender()))
        .map_err(|_| "Ship not found")?;

    let stats = dsl
        .get_ship_config_by_id(ShipConfigId::new(space_ship.ship_config_id))
        .map_err(|_| "Ship stats not found")?;

    // 1. Enforce Turn Limits
    let mut clamped_turn =
        degrees_per_second.clamp(-*stats.get_max_turn_rate(), *stats.get_max_turn_rate());
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

    dsl.update_space_ship_by_id(space_ship);
    Ok(())
}

#[reducer]
pub fn set_thrust_input(
    ctx: &ReducerContext,
    is_thrusting: bool,
    is_breaking: bool,
) -> Result<(), String> {
    let dsl = spacetimedsl::dsl(ctx);

    let mut space_ship = dsl
        .get_space_ship_by_id(SpaceShipId::new(ctx.sender()))
        .map_err(|_| "Ship not found")?;

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

    let config = dsl
        .get_ship_config_by_id(ShipConfigId::new(space_ship.ship_config_id))
        .map_err(|_| "Ship config not found")?;

    let now = ctx.timestamp.to_micros_since_unix_epoch();
    let dt = (now - space_ship.movement.last_update_time) as f32 / 1_000_000.0;

    // 1. Predict current position and rotation
    let (predicted_pos, predicted_rot) =
        predict_movement(&convert_to_movement_state(&space_ship.movement), now);

    // 2. Calculate predicted velocities: v = v₀ + a*dt, clamped
    let predicted_velocity = (space_ship.movement.velocity + space_ship.movement.acceleration * dt)
        .clamp(0.0, *config.get_max_speed());
    let predicted_angular_velocity = (space_ship.movement.angular_velocity
        + space_ship.movement.angular_acceleration * dt)
        .clamp(-*config.get_max_turn_rate(), *config.get_max_turn_rate());

    // 3. Calculate new acceleration based on thrust input
    let new_acceleration = if is_thrusting {
        *config.get_max_acceleration()
    } else if is_breaking {
        -*config.get_max_acceleration()
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
        max_speed: *config.get_max_speed(),
        max_turn_rate: *config.get_max_turn_rate(),
    };

    dsl.update_space_ship_by_id(space_ship);
    Ok(())
}

#[reducer]
pub fn set_turn_input(ctx: &ReducerContext, turn_direction: i8) -> Result<(), String> {
    let dsl = spacetimedsl::dsl(ctx);

    // Validate turn_direction
    if turn_direction != -1 && turn_direction != 0 && turn_direction != 1 {
        return Err(format!(
            "Invalid turn_direction: {}. Must be -1, 0, or 1",
            turn_direction
        ));
    }

    let mut space_ship = dsl
        .get_space_ship_by_id(SpaceShipId::new(ctx.sender()))
        .map_err(|_| "Ship not found")?;

    // Early return if input hasn't changed (Req 3.8)
    if space_ship.input_state.turn_direction == turn_direction {
        return Ok(());
    }

    let config = dsl
        .get_ship_config_by_id(ShipConfigId::new(space_ship.ship_config_id))
        .map_err(|_| "Ship config not found")?;

    let now = ctx.timestamp.to_micros_since_unix_epoch();
    let dt = (now - space_ship.movement.last_update_time) as f32 / 1_000_000.0;

    // 1. Predict current position and rotation
    let (predicted_pos, predicted_rot) =
        predict_movement(&convert_to_movement_state(&space_ship.movement), now);

    // 2. Calculate predicted velocities: v = v₀ + a*dt, clamped
    let predicted_velocity = (space_ship.movement.velocity + space_ship.movement.acceleration * dt)
        .clamp(0.0, *config.get_max_speed());
    let predicted_angular_velocity = (space_ship.movement.angular_velocity
        + space_ship.movement.angular_acceleration * dt)
        .clamp(-*config.get_max_turn_rate(), *config.get_max_turn_rate());

    // 3. Calculate new angular acceleration based on turn direction
    let new_angular_acceleration = turn_direction as f32 * *config.get_max_angular_acceleration();

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
        max_speed: *config.get_max_speed(),
        max_turn_rate: *config.get_max_turn_rate(),
    };

    dsl.update_space_ship_by_id(space_ship);
    Ok(())
}

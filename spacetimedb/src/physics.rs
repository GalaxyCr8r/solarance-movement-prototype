use spacetimedb::SpacetimeType;

#[derive(SpacetimeType, Clone, Copy, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(SpacetimeType, Clone, Copy, Debug, PartialEq)]
pub struct InputState {
    pub is_thrusting: bool,
    pub is_breaking: bool,
    pub turn_direction: i8,
}

#[derive(SpacetimeType, Clone, Copy, Debug, PartialEq)]
pub struct MovementState {
    pub pos: Vec2,
    /// Pixels per second
    pub velocity: f32,
    /// Degrees
    pub rotation: f32,
    /// Degrees per second
    pub angular_velocity: f32,
    /// Microseconds
    pub last_update_time: i64,
    /// Pixels per second squared
    pub acceleration: f32,
    /// Degrees per second squared
    pub angular_acceleration: f32,
    /// Pixels per second (velocity cap)
    pub max_speed: f32,
    /// Degrees per second (angular velocity cap)
    pub max_turn_rate: f32,
    /// When true and angular_acceleration is zero, bleeds angular_velocity toward zero
    /// at max_turn_rate / 2 degrees per second squared.
    pub dampen_angular_rotation: bool,
}

pub fn convert_to_movement_state(
    state: &MovementState,
) -> solarance_shared::physics::MovementState {
    solarance_shared::physics::MovementState {
        pos: solarance_shared::physics::Vec2 {
            x: state.pos.x,
            y: state.pos.y,
        },
        velocity: state.velocity,
        rotation: state.rotation,
        angular_velocity: state.angular_velocity,
        last_update_time: state.last_update_time,
        acceleration: state.acceleration,
        angular_acceleration: state.angular_acceleration,
        max_speed: state.max_speed,
        max_turn_rate: state.max_turn_rate,
        dampen_angular_rotation: state.dampen_angular_rotation,
    }
}

/// Computes the angular velocity at `now`, accounting for both angular acceleration
/// and dampening. Mirrors the logic in `solarance_shared` so the server advances
/// state identically to clients before writing a new trajectory.
pub fn predict_angular_velocity(state: &MovementState, dt: f32) -> f32 {
    if state.angular_acceleration.abs() > f32::EPSILON {
        (state.angular_velocity + state.angular_acceleration * dt)
            .clamp(-state.max_turn_rate, state.max_turn_rate)
    } else if state.dampen_angular_rotation && state.angular_velocity.abs() > f32::EPSILON {
        let decel_rate = state.max_turn_rate / 2.0;
        let t_stop = state.angular_velocity.abs() / decel_rate;
        if t_stop <= dt {
            0.0
        } else {
            state.angular_velocity - state.angular_velocity.signum() * decel_rate * dt
        }
    } else {
        state.angular_velocity
    }
}

pub fn convert_from_movement_state(
    state: &solarance_shared::physics::MovementState,
) -> MovementState {
    MovementState {
        pos: Vec2 {
            x: state.pos.x,
            y: state.pos.y,
        },
        velocity: state.velocity,
        rotation: state.rotation,
        angular_velocity: state.angular_velocity,
        last_update_time: state.last_update_time,
        acceleration: state.acceleration,
        angular_acceleration: state.angular_acceleration,
        max_speed: state.max_speed,
        max_turn_rate: state.max_turn_rate,
        dampen_angular_rotation: state.dampen_angular_rotation,
    }
}

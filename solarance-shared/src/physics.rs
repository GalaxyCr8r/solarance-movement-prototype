use spacetimedb::SpacetimeType;

#[derive(SpacetimeType, Clone, Copy, Debug)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(SpacetimeType, Clone, Copy, Debug)]
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
}

/// shared logic to calculate the current position and rotation based on elapsed time.
pub fn predict_movement(state: &MovementState, current_time: i64) -> (Vec2, f32) {
    if state.last_update_time == 0 || current_time <= state.last_update_time {
        return (state.pos, state.rotation);
    }

    let dt = (current_time - state.last_update_time) as f32 / 1_000_000.0;

    // Linear move: pos = pos + (vel * dt)
    let new_pos = Vec2 {
        x: state.pos.x + (state.velocity.x * dt),
        y: state.pos.y + (state.velocity.y * dt),
    };

    // Angular move: rot = rot + (ang_vel * dt)
    let mut new_rotation = state.rotation + (state.angular_velocity * dt);

    // Keep rotation in 0-360 range
    new_rotation %= 360.0;
    if new_rotation < 0.0 {
        new_rotation += 360.0;
    }

    (new_pos, new_rotation)
}

pub fn rotation_to_vector(degrees: f32) -> Vec2 {
    let radians = degrees.to_radians();
    // Assuming 0 degrees is "Up" (North)
    Vec2 {
        x: radians.sin(),
        y: radians.cos(),
    }
}

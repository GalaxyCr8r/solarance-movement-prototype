#[derive(Clone, Copy, Debug)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug)]
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
}

/// Shared logic to calculate the current position and rotation based on elapsed time.
///
/// When the ship is turning (`angular_velocity != 0`) while moving, the position
/// is computed by integrating along the arc the ship traces, rather than projecting
/// in a straight line from the initial heading. This produces smooth, curved
/// trajectories for dead reckoning on both client and server.
pub fn predict_movement(state: &MovementState, current_time: i64) -> (Vec2, f32) {
    if state.last_update_time == 0 || current_time <= state.last_update_time {
        return (state.pos, state.rotation);
    }

    let dt = (current_time - state.last_update_time) as f32 / 1_000_000.0;

    let new_velocity = state.velocity + state.acceleration * dt;
    let new_angular_velocity = state.angular_velocity + state.angular_acceleration * dt;

    let new_rotation = calculate_new_rotation(state, dt, new_angular_velocity);
    let new_pos = calculate_new_position(state, dt, new_velocity);

    (new_pos, new_rotation)
}

fn calculate_new_rotation(state: &MovementState, dt: f32, unclamped_angular_velocity: f32) -> f32 {
    let mut new_rotation = if state.angular_acceleration.abs() < f32::EPSILON {
        // No angular acceleration: constant angular velocity
        state.rotation + (state.angular_velocity * dt)
    } else {
        // Angular acceleration with potential clamping
        calculate_accelerated_rotation(state, dt, unclamped_angular_velocity)
    };

    // Keep rotation in 0-360 range
    new_rotation %= 360.0;
    if new_rotation < 0.0 {
        new_rotation += 360.0;
    }

    new_rotation
}

fn calculate_accelerated_rotation(
    state: &MovementState,
    dt: f32,
    unclamped_angular_velocity: f32,
) -> f32 {
    if unclamped_angular_velocity > state.max_turn_rate && state.angular_acceleration > 0.0 {
        calculate_clamped_rotation_positive(state, dt)
    } else if unclamped_angular_velocity < -state.max_turn_rate && state.angular_acceleration < 0.0
    {
        calculate_clamped_rotation_negative(state, dt)
    } else {
        // No clamping needed
        state.rotation
            + (state.angular_velocity * dt)
            + (0.5 * state.angular_acceleration * dt * dt)
    }
}

fn calculate_clamped_rotation_positive(state: &MovementState, dt: f32) -> f32 {
    let t_clamp = (state.max_turn_rate - state.angular_velocity) / state.angular_acceleration;

    if t_clamp <= 0.0 {
        // Already at or above max turn rate
        state.rotation + state.max_turn_rate * dt
    } else if t_clamp >= dt {
        // Won't reach max turn rate in this time step
        state.rotation
            + (state.angular_velocity * dt)
            + (0.5 * state.angular_acceleration * dt * dt)
    } else {
        // Reaches max turn rate partway through
        let accel_rotation =
            state.angular_velocity * t_clamp + 0.5 * state.angular_acceleration * t_clamp * t_clamp;
        let const_rotation = state.max_turn_rate * (dt - t_clamp);
        state.rotation + accel_rotation + const_rotation
    }
}

fn calculate_clamped_rotation_negative(state: &MovementState, dt: f32) -> f32 {
    let t_clamp = (-state.max_turn_rate - state.angular_velocity) / state.angular_acceleration;

    if t_clamp <= 0.0 {
        // Already at or below negative max turn rate
        state.rotation - state.max_turn_rate * dt
    } else if t_clamp >= dt {
        // Won't reach negative max turn rate in this time step
        state.rotation
            + (state.angular_velocity * dt)
            + (0.5 * state.angular_acceleration * dt * dt)
    } else {
        // Reaches negative max turn rate partway through
        let accel_rotation =
            state.angular_velocity * t_clamp + 0.5 * state.angular_acceleration * t_clamp * t_clamp;
        let const_rotation = -state.max_turn_rate * (dt - t_clamp);
        state.rotation + accel_rotation + const_rotation
    }
}

fn calculate_new_position(state: &MovementState, dt: f32, new_velocity: f32) -> Vec2 {
    if state.velocity.abs() < f32::EPSILON && state.acceleration.abs() < f32::EPSILON {
        // No linear speed and no acceleration
        state.pos
    } else if state.angular_velocity.abs() < f32::EPSILON
        && state.angular_acceleration.abs() < f32::EPSILON
    {
        // Straight-line motion
        calculate_straight_line_position(state, dt, new_velocity)
    } else {
        // Arc motion with potential acceleration
        calculate_arc_position(state, dt)
    }
}

fn calculate_straight_line_position(
    state: &MovementState,
    dt: f32,
    unclamped_velocity: f32,
) -> Vec2 {
    let theta = state.rotation.to_radians();

    if state.acceleration.abs() < f32::EPSILON {
        // No acceleration: constant velocity motion
        Vec2 {
            x: state.pos.x + theta.cos() * state.velocity * dt,
            y: state.pos.y + theta.sin() * state.velocity * dt,
        }
    } else {
        // Linear acceleration with potential velocity clamping
        let displacement = calculate_accelerated_displacement(state, dt, unclamped_velocity);
        Vec2 {
            x: state.pos.x + theta.cos() * displacement,
            y: state.pos.y + theta.sin() * displacement,
        }
    }
}

fn calculate_accelerated_displacement(
    state: &MovementState,
    dt: f32,
    unclamped_velocity: f32,
) -> f32 {
    if unclamped_velocity > state.max_speed && state.acceleration > 0.0 {
        let t_clamp = (state.max_speed - state.velocity) / state.acceleration;
        if t_clamp <= 0.0 {
            state.max_speed * dt
        } else if t_clamp >= dt {
            state.velocity * dt + 0.5 * state.acceleration * dt * dt
        } else {
            let accel_disp =
                state.velocity * t_clamp + 0.5 * state.acceleration * t_clamp * t_clamp;
            let const_disp = state.max_speed * (dt - t_clamp);
            accel_disp + const_disp
        }
    } else if unclamped_velocity < 0.0 && state.acceleration < 0.0 {
        let t_clamp = -state.velocity / state.acceleration;
        if t_clamp <= 0.0 {
            0.0
        } else if t_clamp >= dt {
            state.velocity * dt + 0.5 * state.acceleration * dt * dt
        } else {
            state.velocity * t_clamp + 0.5 * state.acceleration * t_clamp * t_clamp
        }
    } else {
        state.velocity * dt + 0.5 * state.acceleration * dt * dt
    }
}

fn calculate_arc_position(state: &MovementState, dt: f32) -> Vec2 {
    if state.acceleration.abs() < f32::EPSILON && state.angular_acceleration.abs() < f32::EPSILON {
        // No acceleration: use analytical arc motion formula
        calculate_no_acceleration_arc_position(state, dt)
    } else {
        // Combined acceleration and turning: use numerical integration
        calculate_integrated_arc_position(state, dt)
    }
}

fn calculate_no_acceleration_arc_position(state: &MovementState, dt: f32) -> Vec2 {
    let omega = state.angular_velocity.to_radians();
    let theta0 = state.rotation.to_radians();
    let theta1 = theta0 + omega * dt;
    let r = state.velocity / omega;

    Vec2 {
        x: state.pos.x + r * (theta1.sin() - theta0.sin()),
        y: state.pos.y - r * (theta1.cos() - theta0.cos()),
    }
}

fn calculate_integrated_arc_position(state: &MovementState, dt: f32) -> Vec2 {
    // Maximum time to integrate with acceleration (30 seconds should be enough to reach max velocity)
    const MAX_INTEGRATION_TIME: f32 = 30.0;
    const INTEGRATION_STEPS: i32 = 20;

    // Cap the integration time and calculate what remains
    let integration_dt = dt.min(MAX_INTEGRATION_TIME);
    let step_dt = integration_dt / INTEGRATION_STEPS as f32;

    let mut x = state.pos.x;
    let mut y = state.pos.y;
    let mut v = state.velocity;
    let mut theta = state.rotation.to_radians();
    let mut omega = state.angular_velocity.to_radians();
    let mut time_integrated = 0.0;

    let a = state.acceleration;
    let alpha = state.angular_acceleration.to_radians();
    let max_omega = state.max_turn_rate.to_radians();

    for _ in 0..INTEGRATION_STEPS {
        let prev_v = v;
        let prev_omega = omega;

        // Update velocity and angular velocity based on acceleration
        v += a * step_dt;
        omega += alpha * step_dt;

        // Clamp velocity if needed
        if v > state.max_speed {
            v = state.max_speed;
        } else if v < 0.0 {
            v = 0.0;
        }

        // Clamp angular velocity if needed
        if omega > max_omega {
            omega = max_omega;
        } else if omega < -max_omega {
            omega = -max_omega;
        }

        // Check if both velocity and angular velocity are clamped (no more acceleration)
        let v_clamped =
            (v == state.max_speed || v == 0.0) && (prev_v == v || a.abs() > f32::EPSILON);
        let omega_clamped = (omega == max_omega || omega == -max_omega)
            && (prev_omega == omega || alpha.abs() > f32::EPSILON);

        if v_clamped && omega_clamped {
            // Both are clamped - switch to analytical arc formula for remaining time
            let remaining_dt = dt - time_integrated;

            if omega.abs() < f32::EPSILON {
                // Straight line motion for remaining time
                x += theta.cos() * v * remaining_dt;
                y += theta.sin() * v * remaining_dt;
            } else {
                // Analytical arc motion for remaining time
                let theta1 = theta + omega * remaining_dt;
                let r = v / omega;
                x += r * (theta1.sin() - theta.sin());
                y -= r * (theta1.cos() - theta.cos());
            }

            return Vec2 { x, y };
        }

        // Continue numerical integration
        x += theta.cos() * v * step_dt;
        y += theta.sin() * v * step_dt;
        theta += omega * step_dt;
        time_integrated += step_dt;
    }

    // If we've integrated for MAX_INTEGRATION_TIME but there's still time remaining,
    // use analytical formula for the rest (velocities should be at max by now)
    if dt > MAX_INTEGRATION_TIME {
        let remaining_dt = dt - MAX_INTEGRATION_TIME;

        if omega.abs() < f32::EPSILON {
            // Straight line motion for remaining time
            x += theta.cos() * v * remaining_dt;
            y += theta.sin() * v * remaining_dt;
        } else {
            // Analytical arc motion for remaining time
            let theta1 = theta + omega * remaining_dt;
            let r = v / omega;
            x += r * (theta1.sin() - theta.sin());
            y -= r * (theta1.cos() - theta.cos());
        }
    }

    Vec2 { x, y }
}

pub fn rotation_to_vector(degrees: f32) -> Vec2 {
    let radians = degrees.to_radians();
    // Assuming 0 degrees is "Up" (North)
    Vec2 {
        x: radians.sin(),
        y: radians.cos(),
    }
}

#[cfg(test)]
mod tests;

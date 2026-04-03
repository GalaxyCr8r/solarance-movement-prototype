#[derive(Clone, Copy, Debug)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn distance_to(&self, other: &Vec2) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
    pub fn distance_to_sq(&self, other: &Vec2) -> f32 {
        (self.x - other.x).powi(2) + (self.y - other.y).powi(2)
    }
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
    /// Dampening rate applied to forward velocity when acceleration == 0 (pixels per second squared).
    /// The velocity decays toward zero at this constant rate. Set to 0 to disable.
    pub dampen_forward_velocity: f32,
    /// Dampening rate applied to angular velocity when angular_acceleration == 0 (degrees per second squared).
    /// The angular velocity decays toward zero at this constant rate. Set to 0 to disable.
    pub dampen_angular_velocity: f32,
}

/// Shared logic to calculate the current position and rotation based on elapsed time.
///
/// Returns the new position and rotation in degrees.
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
    let (new_velocity, new_angular_velocity) = compute_new_velocities(state, dt);

    let new_rotation = calculate_new_rotation(state, dt, new_angular_velocity);
    let new_pos = calculate_new_position(state, dt, new_velocity);

    (new_pos, new_rotation)
}

/// Returns `(velocity, angular_velocity)` predicted at `current_time`, applying
/// dampening correctly when acceleration is zero. Reducers should call this
/// instead of computing `v + a*dt` manually.
pub fn predict_velocities(state: &MovementState, current_time: i64) -> (f32, f32) {
    if state.last_update_time == 0 || current_time <= state.last_update_time {
        return (state.velocity, state.angular_velocity);
    }
    let dt = (current_time - state.last_update_time) as f32 / 1_000_000.0;
    compute_new_velocities(state, dt)
}

/// Core velocity integration used by both `predict_movement` and `predict_velocities`.
/// Applies dampening when acceleration is zero and a dampen rate is set.
fn compute_new_velocities(state: &MovementState, dt: f32) -> (f32, f32) {
    // When no acceleration is set but dampening is active, the dampening is
    // applied as a constant-magnitude deceleration toward zero.
    let new_velocity = if state.acceleration.abs() < f32::EPSILON
        && state.dampen_forward_velocity.abs() > f32::EPSILON
    {
        let decel = state.dampen_forward_velocity * state.velocity.signum();
        let candidate = state.velocity - decel * dt;
        // Clamp to zero so we never overshoot
        if candidate.signum() != state.velocity.signum() {
            0.0
        } else {
            candidate
        }
    } else {
        state.velocity + state.acceleration * dt
    };

    let new_angular_velocity = if state.angular_acceleration.abs() < f32::EPSILON
        && state.dampen_angular_velocity.abs() > f32::EPSILON
    {
        let decel = state.dampen_angular_velocity * state.angular_velocity.signum();
        let candidate = state.angular_velocity - decel * dt;
        // Clamp to zero to avoid overshooting past zero
        if candidate.signum() != state.angular_velocity.signum() {
            0.0
        } else {
            candidate
        }
    } else {
        state.angular_velocity + state.angular_acceleration * dt
    };

    (new_velocity, new_angular_velocity)
}

fn calculate_new_rotation(state: &MovementState, dt: f32, unclamped_angular_velocity: f32) -> f32 {
    let mut new_rotation_degrees = if state.angular_acceleration.abs() < f32::EPSILON {
        if state.dampen_angular_velocity.abs() > f32::EPSILON
            && state.angular_velocity.abs() > f32::EPSILON
        {
            // Dampening: decelerate toward zero at a constant rate.
            // t_stop is when angular velocity reaches zero; after that the ship is stationary.
            let t_stop = state.angular_velocity.abs() / state.dampen_angular_velocity;
            if dt >= t_stop {
                // Ship stops mid-step: integrate only up to t_stop
                state.rotation + state.angular_velocity * t_stop
                    - 0.5
                        * state.dampen_angular_velocity
                        * state.angular_velocity.signum()
                        * t_stop
                        * t_stop
            } else {
                state.rotation + state.angular_velocity * dt
                    - 0.5
                        * state.dampen_angular_velocity
                        * state.angular_velocity.signum()
                        * dt
                        * dt
            }
        } else {
            // No angular acceleration, no dampening: constant angular velocity
            state.rotation + (state.angular_velocity * dt)
        }
    } else {
        // Angular acceleration with potential clamping
        calculate_accelerated_rotation(state, dt, unclamped_angular_velocity)
    };

    // Keep rotation in 0-360 range
    new_rotation_degrees %= 360.0;
    if new_rotation_degrees < 0.0 {
        new_rotation_degrees += 360.0;
    }

    new_rotation_degrees
}

fn calculate_accelerated_rotation(
    state: &MovementState,
    dt: f32,
    unclamped_angular_velocity: f32,
) -> f32 {
    let should_be_clamped = unclamped_angular_velocity.abs() > state.max_turn_rate;
    let has_the_same_signum =
        state.angular_acceleration.signum() == unclamped_angular_velocity.signum();

    if should_be_clamped && has_the_same_signum {
        calculate_clamped_rotation(state, dt, unclamped_angular_velocity.signum())
    } else {
        // No clamping needed
        state.rotation
            + (state.angular_velocity * dt)
            + (0.5 * state.angular_acceleration * dt * dt)
    }
}

fn calculate_clamped_rotation(state: &MovementState, dt: f32, sig: f32) -> f32 {
    let t_clamp = (sig * state.max_turn_rate - state.angular_velocity) / state.angular_acceleration;

    if t_clamp <= 0.0 {
        // Already at or above max turn rate
        state.rotation + sig * state.max_turn_rate * dt
    } else if t_clamp >= dt {
        // Won't reach max turn rate in this time step
        state.rotation
            + (state.angular_velocity * dt)
            + (0.5 * state.angular_acceleration * dt * dt)
    } else {
        // Reaches max turn rate partway through
        let accel_rotation =
            state.angular_velocity * t_clamp + 0.5 * state.angular_acceleration * t_clamp * t_clamp;
        let const_rotation = sig * state.max_turn_rate * (dt - t_clamp);
        state.rotation + accel_rotation + const_rotation
    }
}

fn calculate_new_position(state: &MovementState, dt: f32, new_velocity: f32) -> Vec2 {
    let no_linear_accel = state.acceleration.abs() < f32::EPSILON;
    let no_angular_accel = state.angular_acceleration.abs() < f32::EPSILON;
    let no_angular_velocity = state.angular_velocity.abs() < f32::EPSILON;
    let dampening_angular = state.dampen_angular_velocity.abs() > f32::EPSILON;
    let dampening_forward = state.dampen_forward_velocity.abs() > f32::EPSILON;

    if state.velocity.abs() < f32::EPSILON && no_linear_accel && !dampening_forward {
        // No linear speed, no acceleration, no forward dampening
        state.pos
    } else if no_angular_velocity && no_angular_accel && !dampening_angular {
        // Straight-line motion (no turning, no angular dampening)
        calculate_straight_line_position(state, dt, new_velocity)
    } else if no_angular_accel && dampening_angular {
        // Angular velocity is decaying: use numerical integration since the
        // arc radius changes continuously as omega approaches zero.
        calculate_damped_angular_position(state, dt)
    } else {
        // Arc motion with potential acceleration
        calculate_arc_position(state, dt)
    }
}

/// Computes the position change while angular velocity decays linearly to zero
/// (no angular acceleration, but dampen_angular_velocity > 0).
///
/// We split the time step at `t_stop = |omega| / dampen_rate` (when the ship
/// finishes turning), numerically integrate the decelerating arc up to that
/// point, then continue in a straight line (or constant-speed arc fallback)
/// for the remainder.
fn calculate_damped_angular_position(state: &MovementState, dt: f32) -> Vec2 {
    // Time until angular velocity reaches zero
    let t_stop = if state.angular_velocity.abs() > f32::EPSILON {
        state.angular_velocity.abs() / state.dampen_angular_velocity
    } else {
        0.0
    };

    // Numerically integrate the decelerating-spin arc using small steps
    // (or up to t_stop, whichever is smaller).
    const STEPS: i32 = 20;
    let integrate_dt = dt.min(t_stop);
    let step_dt = if t_stop > f32::EPSILON {
        integrate_dt / STEPS as f32
    } else {
        0.0
    };

    let mut x = state.pos.x;
    let mut y = state.pos.y;
    let mut v = state.velocity;
    let mut theta = state.rotation.to_radians();
    let mut omega = state.angular_velocity.to_radians();
    let dampen_omega = state.dampen_angular_velocity.to_radians();
    let sig = omega.signum();

    for _ in 0..STEPS {
        if integrate_dt <= 0.0 {
            break;
        }
        x += theta.cos() * v * step_dt;
        y += theta.sin() * v * step_dt;
        theta += omega * step_dt;
        omega -= dampen_omega * sig * step_dt;
        if omega.signum() != sig {
            break;
        }
    }

    // Remaining time after the ship has stopped turning: straight-line motion
    let remaining_dt = dt - integrate_dt;
    if remaining_dt > f32::EPSILON {
        x += theta.cos() * v * remaining_dt;
        y += theta.sin() * v * remaining_dt;
    }

    Vec2 { x, y }
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

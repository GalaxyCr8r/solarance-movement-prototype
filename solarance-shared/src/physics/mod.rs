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
    /// When true and angular_acceleration is zero, bleeds angular_velocity toward zero
    /// at max_turn_rate / 2 degrees per second squared.
    pub dampen_angular_rotation: bool,
}

/// Predicts position, rotation, forward velocity, and angular velocity at `current_time`.
///
/// Returns `(pos, rotation_degrees, velocity_px_per_s, angular_velocity_deg_per_s)`.
///
/// All four values come from a single unified simulation pass so they are always
/// mutually consistent. The simulation breaks `dt` into at most three phases at
/// velocity-clamping boundaries (angular event, linear event). Each phase uses an
/// analytical formula where the physics is clean; only phases where angular velocity
/// is actively changing fall back to bounded numerical integration. Because each
/// numerical phase is bounded by the time to the next clamping event — not by the
/// total prediction horizon — accuracy is independent of how large `dt` is.
pub fn predict_movement(state: &MovementState, current_time: i64) -> (Vec2, f32, f32, f32) {
    if state.last_update_time == 0 || current_time <= state.last_update_time {
        return (
            state.pos,
            state.rotation,
            state.velocity,
            state.angular_velocity,
        );
    }

    let dt = (current_time - state.last_update_time) as f32 / 1_000_000.0;
    simulate(state, dt)
}

fn simulate(state: &MovementState, total_dt: f32) -> (Vec2, f32, f32, f32) {
    let max_v = state.max_speed;
    let max_omega = state.max_turn_rate;
    let decel_rate = state.max_turn_rate / 2.0;
    let a = state.acceleration;
    let alpha = state.angular_acceleration;
    let dampen = state.dampen_angular_rotation;

    let mut x = state.pos.x;
    let mut y = state.pos.y;
    let mut theta = state.rotation; // degrees
    let mut v = state.velocity;
    let mut omega = state.angular_velocity; // degrees/s

    let mut remaining = total_dt;

    while remaining > f32::EPSILON {
        let t_omega = omega_event_time(omega, alpha, max_omega, decel_rate, dampen);
        let t_v = v_event_time(v, a, max_v);
        let phase_dt = remaining.min(t_omega).min(t_v);

        if phase_dt < f32::EPSILON {
            break;
        }

        if t_omega.is_finite() {
            // Angular velocity is changing — bounded numerical integration.
            // Phase duration equals time to the next angular clamping event,
            // so accuracy is independent of the overall dt.
            numerical_phase(
                &mut x, &mut y, &mut theta, &mut v, &mut omega, a, alpha, max_v, max_omega,
                decel_rate, dampen, phase_dt,
            );
        } else {
            // Angular velocity is constant — use analytical formulas.
            constant_omega_phase(&mut x, &mut y, &mut theta, &mut v, omega, a, max_v, phase_dt);
        }

        // Snap velocities exactly at event boundaries to prevent floating-point drift.
        if t_omega.is_finite() && phase_dt >= t_omega - 1e-6 {
            omega = if dampen && alpha.abs() < f32::EPSILON {
                0.0
            } else if alpha > 0.0 {
                max_omega
            } else {
                -max_omega
            };
        }
        if t_v.is_finite() && phase_dt >= t_v - 1e-6 {
            v = if a > 0.0 { max_v } else { 0.0 };
        }

        remaining -= phase_dt;
    }

    theta %= 360.0;
    if theta < 0.0 {
        theta += 360.0;
    }

    (Vec2 { x, y }, theta, v, omega)
}

/// Seconds until angular velocity reaches its clamping boundary.
/// Returns `f32::INFINITY` when omega is already at its boundary or not changing.
fn omega_event_time(omega: f32, alpha: f32, max_omega: f32, decel_rate: f32, dampen: bool) -> f32 {
    if alpha.abs() > f32::EPSILON {
        let target = if alpha > 0.0 { max_omega } else { -max_omega };
        let t = (target - omega) / alpha;
        if t > 0.0 { t } else { f32::INFINITY }
    } else if dampen && omega.abs() > f32::EPSILON {
        omega.abs() / decel_rate
    } else {
        f32::INFINITY
    }
}

/// Seconds until linear velocity reaches its clamping boundary.
/// Returns `f32::INFINITY` when velocity is already at its boundary or not changing.
fn v_event_time(v: f32, a: f32, max_v: f32) -> f32 {
    if a > f32::EPSILON {
        let t = (max_v - v) / a;
        if t > 0.0 { t } else { f32::INFINITY }
    } else if a < -f32::EPSILON {
        let t = -v / a;
        if t > 0.0 { t } else { f32::INFINITY }
    } else {
        f32::INFINITY
    }
}

/// Phase where omega is constant. Uses closed-form arc/straight-line formulas where
/// clean; falls back to numerical only when both omega and linear acceleration are
/// non-zero (arc with changing radius), which is bounded by `t_v`.
fn constant_omega_phase(
    x: &mut f32,
    y: &mut f32,
    theta: &mut f32,
    v: &mut f32,
    omega: f32,
    a: f32,
    max_v: f32,
    dt: f32,
) {
    if omega.abs() < f32::EPSILON {
        // Straight-line motion.
        if a.abs() < f32::EPSILON {
            let theta_rad = theta.to_radians();
            *x += theta_rad.cos() * *v * dt;
            *y += theta_rad.sin() * *v * dt;
        } else {
            // Kinematic formula — dt is bounded by t_v so v cannot overshoot.
            let d = *v * dt + 0.5 * a * dt * dt;
            let theta_rad = theta.to_radians();
            *x += theta_rad.cos() * d;
            *y += theta_rad.sin() * d;
            *v += a * dt; // caller snaps v exactly at the event boundary
        }
    } else if a.abs() < f32::EPSILON {
        // Pure circular arc (analytical).
        let omega_rad = omega.to_radians();
        let theta0 = theta.to_radians();
        let theta1 = theta0 + omega_rad * dt;
        let r = *v / omega_rad;
        *x += r * (theta1.sin() - theta0.sin());
        *y -= r * (theta1.cos() - theta0.cos());
        *theta += omega * dt;
    } else {
        // Arc with changing speed — numerical, but bounded by t_v (time to reach max speed).
        const STEPS: usize = 20;
        let step_dt = dt / STEPS as f32;
        for _ in 0..STEPS {
            let theta_rad = theta.to_radians();
            *x += theta_rad.cos() * *v * step_dt;
            *y += theta_rad.sin() * *v * step_dt;
            *theta += omega * step_dt;
            *v = (*v + a * step_dt).clamp(0.0, max_v);
        }
    }
}

/// Phase where omega is changing (angular acceleration or dampening).
/// Duration is bounded by `t_omega` (time to the angular clamping event), so `dt`
/// is always short and numerical integration stays accurate regardless of the overall
/// prediction interval.
///
/// Uses the trapezoidal rule for rotation (exact for linear omega change) and forward
/// Euler for position (sufficient given the short phase duration).
fn numerical_phase(
    x: &mut f32,
    y: &mut f32,
    theta: &mut f32,
    v: &mut f32,
    omega: &mut f32,
    a: f32,
    alpha: f32,
    max_v: f32,
    max_omega: f32,
    decel_rate: f32,
    dampen: bool,
    dt: f32,
) {
    const STEPS: usize = 20;
    let step_dt = dt / STEPS as f32;

    for _ in 0..STEPS {
        let omega_old = *omega;

        // Update angular velocity.
        if dampen && alpha.abs() < f32::EPSILON {
            let d_omega = -omega.signum() * decel_rate * step_dt;
            if d_omega.abs() >= omega.abs() {
                *omega = 0.0;
            } else {
                *omega += d_omega;
            }
        } else {
            *omega = (*omega + alpha * step_dt).clamp(-max_omega, max_omega);
        }

        // Update position using the heading at the start of the step.
        let theta_rad = theta.to_radians();
        *x += theta_rad.cos() * *v * step_dt;
        *y += theta_rad.sin() * *v * step_dt;

        // Trapezoidal rule for rotation: exact when omega changes linearly.
        *theta += (omega_old + *omega) / 2.0 * step_dt;

        // Update linear velocity.
        *v = (*v + a * step_dt).clamp(0.0, max_v);
    }
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

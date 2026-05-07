use super::*;

fn make_state(
    x: f32,
    y: f32,
    velocity: f32,
    rotation: f32,
    angular_velocity: f32,
    time: i64,
) -> MovementState {
    MovementState {
        pos: Vec2 { x, y },
        velocity,
        rotation,
        angular_velocity,
        last_update_time: time,
        acceleration: 0.0,
        angular_acceleration: 0.0,
        max_speed: 500.0,
        max_turn_rate: 180.0,
        dampen_angular_rotation: false,
    }
}

const BASE_TIME: i64 = 1; // non-zero so we don't hit the "uninitialized" guard
const ONE_SECOND: i64 = 1_000_000; // 1 second in microseconds

#[test]
fn straight_line_no_regression() {
    // Ship at origin heading 0° (east along +x in standard trig) with velocity 100 px/s
    let state = make_state(0.0, 0.0, 100.0, 0.0, 0.0, BASE_TIME);
    let (pos, rot, ..) = predict_movement(&state, BASE_TIME + ONE_SECOND);

    assert!(
        (pos.x - 100.0).abs() < 0.01,
        "x should be ~100, got {}",
        pos.x
    );
    assert!(pos.y.abs() < 0.01, "y should be ~0, got {}", pos.y);
    assert!(
        (rot - 0.0).abs() < 0.01,
        "rotation should be ~0, got {}",
        rot
    );
}

#[test]
fn stationary_ship_rotation_only() {
    // Ship not moving but rotating at 90 deg/s for 1 second
    let state = make_state(5.0, 10.0, 0.0, 0.0, 90.0, BASE_TIME);
    let (pos, rot, ..) = predict_movement(&state, BASE_TIME + ONE_SECOND);

    assert!(
        (pos.x - 5.0).abs() < 0.01,
        "x should stay at 5, got {}",
        pos.x
    );
    assert!(
        (pos.y - 10.0).abs() < 0.01,
        "y should stay at 10, got {}",
        pos.y
    );
    assert!(
        (rot - 90.0).abs() < 0.01,
        "rotation should be ~90, got {}",
        rot
    );
}

#[test]
fn quarter_turn_arc() {
    // Ship at origin, heading 0° (east), speed 100, turning at 90°/s.
    // After 1 second it has turned to 90° and should be at approximately
    // the analytically-computed arc position.
    let state = make_state(0.0, 0.0, 100.0, 0.0, 90.0, BASE_TIME);
    let (pos, rot, ..) = predict_movement(&state, BASE_TIME + ONE_SECOND);

    // Analytical: r = v/ω = 100 / (π/2) ≈ 63.66
    // x = r * (sin(π/2) - sin(0)) = r * 1 ≈ 63.66
    // y = -r * (cos(π/2) - cos(0)) = -r * (-1) = r ≈ 63.66
    let omega_rad = std::f32::consts::FRAC_PI_2; // 90° in radians
    let r = 100.0 / omega_rad;

    assert!((pos.x - r).abs() < 0.1, "x should be ~{}, got {}", r, pos.x);
    assert!((pos.y - r).abs() < 0.1, "y should be ~{}, got {}", r, pos.y);
    assert!(
        (rot - 90.0).abs() < 0.01,
        "rotation should be ~90, got {}",
        rot
    );
}

#[test]
fn full_circle_returns_near_origin() {
    // Ship at origin, heading 0°, speed 100, turning at 360°/s.
    // After 1 second it completes a full circle and should be back near the origin.
    let state = make_state(0.0, 0.0, 100.0, 0.0, 360.0, BASE_TIME);
    let (pos, rot, ..) = predict_movement(&state, BASE_TIME + ONE_SECOND);

    assert!(
        pos.x.abs() < 0.5,
        "x should be near 0 after full circle, got {}",
        pos.x
    );
    assert!(
        pos.y.abs() < 0.5,
        "y should be near 0 after full circle, got {}",
        pos.y
    );
    // Rotation wraps back to 0
    assert!(
        rot.abs() < 0.01 || (rot - 360.0).abs() < 0.01,
        "rotation should be ~0 or ~360, got {}",
        rot
    );
}

#[test]
fn negative_angular_velocity() {
    // Same as quarter turn but turning left (negative angular velocity)
    let state = make_state(0.0, 0.0, 100.0, 0.0, -90.0, BASE_TIME);
    let (pos, rot, ..) = predict_movement(&state, BASE_TIME + ONE_SECOND);

    let omega_rad = std::f32::consts::FRAC_PI_2;
    let r = 100.0 / omega_rad;

    // Turning left: x stays positive, y goes negative
    assert!((pos.x - r).abs() < 0.1, "x should be ~{}, got {}", r, pos.x);
    assert!(
        (pos.y + r).abs() < 0.1,
        "y should be ~-{}, got {}",
        r,
        pos.y
    );
    assert!(
        (rot - 270.0).abs() < 0.01,
        "rotation should be ~270, got {}",
        rot
    );
}

#[test]
fn no_movement_when_time_not_advanced() {
    let state = make_state(10.0, 20.0, 50.0, 45.0, 30.0, 100);
    let (pos, rot, ..) = predict_movement(&state, 100); // same time
    assert!((pos.x - 10.0).abs() < 0.001);
    assert!((pos.y - 20.0).abs() < 0.001);
    assert!((rot - 45.0).abs() < 0.001);

    let (pos2, rot2, ..) = predict_movement(&state, 50); // earlier time
    assert!((pos2.x - 10.0).abs() < 0.001);
    assert!((pos2.y - 20.0).abs() < 0.001);
    assert!((rot2 - 45.0).abs() < 0.001);
}

#[test]
fn dampening_stops_rotation_within_step() {
    // max_turn_rate = 180 → decel_rate = 90 °/s²
    // angular_velocity = 45 °/s → t_stop = 45/90 = 0.5 s
    // rotation gained = 0.5 * 45 * 0.5 = 11.25°
    // After 1 second the ship should be at 11.25° and holding.
    let mut state = make_state(0.0, 0.0, 0.0, 0.0, 45.0, BASE_TIME);
    state.dampen_angular_rotation = true;

    let (_, rot, ..) = predict_movement(&state, BASE_TIME + ONE_SECOND);
    assert!(
        (rot - 11.25).abs() < 0.01,
        "rotation should be ~11.25° after dampening stop, got {}",
        rot
    );
}

#[test]
fn dampening_partial_deceleration_within_step() {
    // max_turn_rate = 180 → decel_rate = 90 °/s²
    // angular_velocity = 180 °/s → t_stop = 180/90 = 2 s (longer than 1 s step)
    // rotation gained = 180 * 1.0 - 0.5 * 1 * 90 * 1.0² = 180 - 45 = 135°
    let mut state = make_state(0.0, 0.0, 0.0, 0.0, 180.0, BASE_TIME);
    state.dampen_angular_rotation = true;

    let (_, rot, ..) = predict_movement(&state, BASE_TIME + ONE_SECOND);
    assert!(
        (rot - 135.0).abs() < 0.01,
        "rotation should be ~135° after partial dampening, got {}",
        rot
    );
}

#[test]
fn dampening_negative_angular_velocity_clamps_at_zero() {
    // Same as dampening_stops_rotation_within_step but turning left.
    // angular_velocity = -45 °/s → stops at -11.25° → wraps to 348.75°
    let mut state = make_state(0.0, 0.0, 0.0, 0.0, -45.0, BASE_TIME);
    state.dampen_angular_rotation = true;

    let (_, rot, ..) = predict_movement(&state, BASE_TIME + ONE_SECOND);
    assert!(
        (rot - 348.75).abs() < 0.01,
        "rotation should be ~348.75° after negative dampening stop, got {}",
        rot
    );
}

#[test]
fn dampening_off_does_not_affect_rotation() {
    // With dampening off, angular_velocity of 45 °/s for 1 s → 45°
    let state = make_state(0.0, 0.0, 0.0, 0.0, 45.0, BASE_TIME);
    let (_, rot, ..) = predict_movement(&state, BASE_TIME + ONE_SECOND);
    assert!(
        (rot - 45.0).abs() < 0.01,
        "rotation without dampening should be ~45°, got {}",
        rot
    );
}

#[test]
fn dampening_arc_position_stops_curving() {
    // Moving ship with angular_velocity that will stop mid-step.
    // After angular_velocity reaches zero the ship should continue straight.
    // max_turn_rate = 180 → decel_rate = 90 °/s²
    // angular_velocity = 45 °/s → t_stop = 0.5 s
    // At t_stop, rotation = 11.25°. Ship then travels straight at 11.25° for the remaining 0.5 s.
    let mut state = make_state(0.0, 0.0, 100.0, 0.0, 45.0, BASE_TIME);
    state.dampen_angular_rotation = true;

    let (pos, rot, ..) = predict_movement(&state, BASE_TIME + ONE_SECOND);

    // After 1 second the ship must have stopped spinning
    assert!(
        (rot - 11.25).abs() < 0.5,
        "rotation should be ~11.25° after dampening arc, got {}",
        rot
    );

    // Position must be non-zero (ship was moving)
    let moved = (pos.x * pos.x + pos.y * pos.y).sqrt();
    assert!(
        moved > 10.0,
        "ship should have moved significantly, got distance {}",
        moved
    );
}

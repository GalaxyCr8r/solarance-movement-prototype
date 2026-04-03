use solarance_shared::physics;

/// Clock synchronization with server
/// Get current server time estimate
pub fn get_server_time(server_offset_micros: i64) -> i64 {
    let client_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as i64;

    client_time + server_offset_micros
}

/// Convert from generated bindings MovementState to shared physics MovementState
pub fn convert_movement_state(
    state: &crate::module_bindings::MovementState,
) -> physics::MovementState {
    physics::MovementState {
        pos: physics::Vec2 {
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
        dampen_forward_velocity: state.dampen_forward_velocity,
        dampen_angular_velocity: state.dampen_angular_velocity,
    }
}

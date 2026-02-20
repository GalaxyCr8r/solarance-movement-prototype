# Implementation Plan: Acceleration-Based Input System

## Overview

This plan implements acceleration-based ship control where players press/release thrust and turn buttons, causing ships to accelerate/decelerate naturally rather than instantly changing velocity. The implementation extends the existing dead reckoning physics system with acceleration fields and new input-based reducers.

Key changes:

- Extend MovementState with acceleration fields and velocity limits
- Add InputState tracking to SpaceShip table
- Implement new reducers (set_thrust_input, set_turn_input) that set acceleration instead of velocity
- Enhance predict_movement() to handle constant acceleration with velocity clamping
- Update client input handling to send discrete button press/release events

## Tasks

- [ ] 1. Extend MovementState with acceleration fields
  - [x] 1.1 Add acceleration fields to shared MovementState
    - Add `acceleration: f32` (pixels/sec²) to `solarance-shared/src/physics.rs::MovementState`
    - Add `angular_acceleration: f32` (degrees/sec²) to `solarance-shared/src/physics.rs::MovementState`
    - Add `max_speed: f32` (pixels/sec) to `solarance-shared/src/physics.rs::MovementState`
    - Add `max_turn_rate: f32` (degrees/sec) to `solarance-shared/src/physics.rs::MovementState`
    - _Requirements: 2.1, 2.2, 2.3, 2.4_

  - [x] 1.2 Add acceleration fields to SpacetimeDB MovementState
    - Add `acceleration: f32` to `spacetimedb/src/physics.rs::MovementState`
    - Add `angular_acceleration: f32` to `spacetimedb/src/physics.rs::MovementState`
    - Add `max_speed: f32` to `spacetimedb/src/physics.rs::MovementState`
    - Add `max_turn_rate: f32` to `spacetimedb/src/physics.rs::MovementState`
    - Update `convert_to_movement_state()` to map new fields
    - Update `convert_from_movement_state()` to map new fields
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

  - [ ]\* 1.3 Write property test for MovementState serialization round-trip
    - **Property 10: MovementState serialization round-trip**
    - **Validates: Requirements 2.5, 2.6**
    - Generate random MovementState instances with all fields
    - Test that `convert_to_movement_state()` then `convert_from_movement_state()` preserves all fields
    - Run 100+ iterations

- [x] 2. Extend ShipConfig with acceleration fields
  - [x] 2.1 Add acceleration configuration to ShipConfig table
    - Add `max_acceleration: f32` field to `spacetimedb/src/lib.rs::ShipConfig`
    - Add `max_angular_acceleration: f32` field to `spacetimedb/src/lib.rs::ShipConfig`
    - Update `init()` reducer to set reasonable default values (e.g., max_acceleration: 100.0, max_angular_acceleration: 180.0)
    - _Requirements: 4.1, 4.2, 4.3, 4.4_

- [x] 3. Add InputState to SpaceShip table
  - [x] 3.1 Create InputState struct
    - Create `InputState` struct in `spacetimedb/src/physics.rs` with fields:
      - `is_thrusting: bool`
      - `turn_direction: i8` (values: -1, 0, 1)
    - Derive `SpacetimeType, Clone, Copy, Debug`
    - _Requirements: 5.1, 5.2_

  - [x] 3.2 Add InputState field to SpaceShip table
    - Add `input_state: InputState` field to `spacetimedb/src/lib.rs::SpaceShip`
    - Initialize to `InputState { is_thrusting: false, turn_direction: 0 }` in `on_connect()` reducer
    - _Requirements: 5.3, 5.4_

- [ ] 4. Implement acceleration-based physics prediction
  - [ ] 4.1 Implement linear acceleration prediction (no clamping)
    - In `solarance-shared/src/physics.rs::predict_movement()`, add case for non-zero acceleration
    - Calculate final velocity: `v_final = v_initial + acceleration * dt`
    - Calculate position: `x = x₀ + v₀*dt + 0.5*a*dt²` (use kinematic equations)
    - Handle case where acceleration is zero (existing straight-line logic)
    - _Requirements: 1.1, 1.2_

  - [ ]\* 4.2 Write property test for linear acceleration kinematic equations
    - **Property 1: Linear acceleration kinematic equations**
    - **Validates: Requirements 1.1, 1.2**
    - Generate random MovementState with non-zero acceleration, zero angular acceleration
    - Verify v = v₀ + at and x = x₀ + v₀t + ½at² (when velocity not clamped)
    - Run 100+ iterations

  - [ ] 4.3 Implement angular acceleration prediction (no clamping)
    - In `solarance-shared/src/physics.rs::predict_movement()`, add case for non-zero angular acceleration
    - Calculate final angular velocity: `ω_final = ω_initial + angular_acceleration * dt`
    - Calculate rotation: `θ = θ₀ + ω₀*dt + 0.5*α*dt²`
    - Handle case where angular acceleration is zero (existing rotation logic)
    - _Requirements: 1.3, 1.4_

  - [ ]\* 4.4 Write property test for angular acceleration kinematic equations
    - **Property 2: Angular acceleration kinematic equations**
    - **Validates: Requirements 1.3, 1.4**
    - Generate random MovementState with non-zero angular acceleration, zero linear acceleration
    - Verify ω = ω₀ + αt and θ = θ₀ + ω₀t + ½αt²
    - Run 100+ iterations

  - [ ] 4.5 Implement velocity clamping during prediction
    - In `solarance-shared/src/physics.rs::predict_movement()`, check if predicted velocity exceeds max_speed
    - If clamped: calculate exact time `t_clamp = (max_speed - v_initial) / acceleration`
    - Apply acceleration until t_clamp, then constant velocity for remaining time
    - Position: `x = x₀ + v₀*t_clamp + 0.5*a*t_clamp² + max_speed*(dt - t_clamp)`
    - _Requirements: 1.5, 6.1, 6.3_

  - [ ]\* 4.6 Write property test for velocity clamping
    - **Property 3: Velocity clamping**
    - **Validates: Requirements 1.5, 6.3**
    - Generate random MovementState where predicted velocity would exceed max_speed
    - Verify velocity is clamped to max_speed
    - Verify position uses piecewise motion (accelerating then constant)
    - Run 100+ iterations

  - [ ]\* 4.7 Write property test for clamp time calculation accuracy
    - **Property 8: Clamp time calculation accuracy**
    - **Validates: Requirements 9.2**
    - Generate random MovementState where clamping occurs
    - Verify `v₀ + a*t_clamp = max_speed` (within numerical tolerance)
    - Run 100+ iterations

  - [ ]\* 4.8 Write property test for smooth transition at velocity limit
    - **Property 9: Smooth transition at velocity limit**
    - **Validates: Requirements 9.3**
    - Generate random MovementState where velocity reaches max_speed
    - Verify position function is continuous at clamping point (no jumps)
    - Run 100+ iterations

  - [ ] 4.9 Implement angular velocity clamping during prediction
    - In `solarance-shared/src/physics.rs::predict_movement()`, check if predicted angular velocity exceeds max_turn_rate
    - If clamped: calculate exact time `t_clamp_angular = (max_turn_rate - ω_initial) / angular_acceleration`
    - Apply angular acceleration until t_clamp_angular, then constant angular velocity for remaining time
    - Rotation: `θ = θ₀ + ω₀*t_clamp + 0.5*α*t_clamp² + max_turn_rate*(dt - t_clamp)`
    - _Requirements: 1.6, 6.2, 6.4_

  - [ ]\* 4.10 Write property test for angular velocity clamping
    - **Property 4: Angular velocity clamping**
    - **Validates: Requirements 1.6, 6.4**
    - Generate random MovementState where predicted angular velocity would exceed max_turn_rate
    - Verify angular velocity is clamped to max_turn_rate
    - Verify rotation uses piecewise motion
    - Run 100+ iterations

  - [ ] 4.11 Implement combined acceleration and turning
    - In `solarance-shared/src/physics.rs::predict_movement()`, handle case where both accelerations are non-zero
    - Use numerical integration (small fixed-step Euler or RK4) to approximate arc motion with changing speed
    - Handle velocity clamping during numerical integration
    - _Requirements: 1.7_

  - [ ]\* 4.12 Write property test for combined acceleration and turning
    - **Property 5: Combined acceleration and turning**
    - **Validates: Requirements 1.7**
    - Generate random MovementState with both non-zero accelerations
    - Verify trajectory is curved (not straight line)
    - Verify both speed and heading change over time
    - Run 100+ iterations

  - [ ]\* 4.13 Write property test for backward compatibility
    - **Property 6: Backward compatibility with zero acceleration**
    - **Validates: Requirements 7.1**
    - Generate random MovementState with acceleration = 0 and angular_acceleration = 0
    - Compare results with original predict_movement logic
    - Verify identical position and rotation
    - Run 100+ iterations

  - [ ]\* 4.14 Write property test for deterministic prediction
    - **Property 7: Deterministic prediction**
    - **Validates: Requirements 8.5**
    - Generate random MovementState and timestamp
    - Call predict_movement multiple times with same inputs
    - Verify identical results every time
    - Run 100+ iterations

- [ ] 5. Checkpoint - Ensure physics tests pass
  - Ensure all physics property tests pass, ask the user if questions arise.

- [ ] 6. Implement set_thrust_input reducer
  - [ ] 6.1 Create set_thrust_input reducer
    - Add `#[reducer] pub fn set_thrust_input(ctx: &ReducerContext, is_thrusting: bool) -> Result<(), String>` in `spacetimedb/src/lib.rs`
    - Find player's ship or return error
    - Check if `ship.input_state.is_thrusting == is_thrusting`, if so return early (no update needed)
    - Get ship configuration for max_acceleration and max_speed
    - Predict current position and velocity using `predict_movement()`
    - Calculate new acceleration:
      - If `is_thrusting`: set `acceleration = max_acceleration` (applied in direction of current rotation)
      - Else: set `acceleration = 0` (ship coasts at current velocity)
    - Update SpaceShip:
      - `input_state.is_thrusting = is_thrusting`
      - `movement.pos = predicted_pos`
      - `movement.velocity = predicted_velocity` (clamped to max_speed)
      - `movement.acceleration = new_acceleration`
      - `movement.last_update_time = current_time`
    - Update database
    - _Requirements: 3.1, 3.2, 3.6, 3.7, 3.8_

  - [ ]\* 6.2 Write unit test for set_thrust_input logic
    - Extract acceleration calculation logic into testable helper function in `solarance-shared`
    - Test that `is_thrusting=true` sets acceleration to max_acceleration
    - Test that `is_thrusting=false` sets acceleration to 0 and preserves velocity
    - Test that acceleration is applied in direction of current rotation
    - _Requirements: 3.1, 3.2_

- [ ] 7. Implement set_turn_input reducer
  - [ ] 7.1 Create set_turn_input reducer
    - Add `#[reducer] pub fn set_turn_input(ctx: &ReducerContext, turn_direction: i8) -> Result<(), String>` in `spacetimedb/src/lib.rs`
    - Validate `turn_direction` is -1, 0, or 1, else return error
    - Find player's ship or return error
    - Check if `ship.input_state.turn_direction == turn_direction`, if so return early
    - Get ship configuration for max_angular_acceleration and max_turn_rate
    - Predict current position and rotation using `predict_movement()`
    - Calculate new angular acceleration:
      - If `turn_direction == 1`: set `angular_acceleration = max_angular_acceleration`
      - If `turn_direction == -1`: set `angular_acceleration = -max_angular_acceleration`
      - If `turn_direction == 0`: set `angular_acceleration = 0`
    - Update SpaceShip:
      - `input_state.turn_direction = turn_direction`
      - `movement.pos = predicted_pos`
      - `movement.rotation = predicted_rotation`
      - `movement.angular_velocity = predicted_angular_velocity` (clamped to max_turn_rate)
      - `movement.angular_acceleration = new_angular_acceleration`
      - `movement.last_update_time = current_time`
    - Update database
    - _Requirements: 3.3, 3.4, 3.5, 3.6, 3.7, 3.8_

  - [ ]\* 7.2 Write unit test for set_turn_input logic
    - Extract angular acceleration calculation logic into testable helper function in `solarance-shared`
    - Test that `turn_direction=1` sets angular_acceleration to max_angular_acceleration
    - Test that `turn_direction=-1` sets angular_acceleration to -max_angular_acceleration
    - Test that `turn_direction=0` sets angular_acceleration to 0
    - Test that invalid turn_direction values are rejected
    - _Requirements: 3.3, 3.4, 3.5_

- [ ] 8. Update client input handling
  - [ ] 8.1 Modify handle_input to track input state changes
    - In `src/main.rs::handle_input()`, replace velocity-based logic with input state tracking
    - Track previous input state (use static variable or add to GameState)
    - Determine current input state from keyboard:
      - `is_thrusting = is_key_down(W) || is_key_down(Up)`
      - `turn_direction = if is_key_down(D) { 1 } else if is_key_down(A) { -1 } else { 0 }`
    - Compare with previous state:
      - If `is_thrusting` changed: call `ctx.reducers().set_thrust_input(is_thrusting)`
      - If `turn_direction` changed: call `ctx.reducers().set_turn_input(turn_direction)`
    - Update previous state
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7_

  - [ ]\* 8.2 Write unit test for no redundant reducer calls
    - **Property 15: No redundant reducer calls**
    - **Validates: Requirements 10.6, 10.7**
    - Simulate sequence of frames where keyboard input remains unchanged
    - Verify at most one reducer call per input state change
    - Verify no calls when state is stable
    - Run 100+ iterations with random input sequences

- [ ] 9. Update on_connect to initialize new fields
  - [ ] 9.1 Initialize MovementState with acceleration fields
    - In `spacetimedb/src/lib.rs::on_connect()`, update MovementState initialization
    - Set `acceleration: 0.0` and `angular_acceleration: 0.0`
    - Copy `max_speed` and `max_turn_rate` from ShipConfig to MovementState
    - _Requirements: 5.3_

- [ ] 10. Checkpoint - Integration testing
  - Ensure all tests pass, ask the user if questions arise.
  - Manually test in-game:
    - Press W key, verify ship accelerates smoothly
    - Release W key, verify ship coasts at current speed
    - Press A/D keys, verify ship turns with angular acceleration
    - Verify velocity caps at max_speed
    - Verify no redundant database updates when holding keys

- [ ] 11. Clean up old reducers (optional migration step)
  - [ ] 11.1 Mark old reducers as deprecated
    - Add deprecation comments to `set_forward_thrust` and `set_turn_velocity`
    - Document that new code should use `set_thrust_input` and `set_turn_input`
    - Keep old reducers functional for backward compatibility during transition
    - _Requirements: 7.2, 7.3, 7.4_

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties (minimum 100 iterations each)
- Unit tests validate specific examples and edge cases
- SpacetimeDB module cannot run `cargo test` (cdylib), so extract testable logic to `solarance-shared` crate
- Old reducers (set_forward_thrust, set_turn_velocity) remain functional during transition

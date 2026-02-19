# Implementation Plan: Acceleration-Based Input System

## Overview

This implementation plan breaks down the acceleration-based input system into incremental coding tasks. Each task builds on previous work, with property-based tests placed close to implementation to catch errors early. The plan follows this sequence: extend data models → enhance physics prediction → update server reducers → modify client input handling → integration and testing.

## Tasks

- [ ] 1. Extend MovementState data structure with acceleration and clamping fields
  - Add `acceleration: f32` field to MovementState in `solarance-shared/src/physics.rs`
  - Add `angular_acceleration: f32` field to MovementState in `solarance-shared/src/physics.rs`
  - Add `max_speed: f32` field to MovementState in `solarance-shared/src/physics.rs` (denormalized from ShipConfig so predict_movement can clamp)
  - Add `max_turn_rate: f32` field to MovementState in `solarance-shared/src/physics.rs` (denormalized from ShipConfig so predict_movement can clamp)
  - Update MovementState in `spacetimedb/src/physics.rs` with same fields
  - Update conversion functions between shared and SpacetimeDB types
  - _Requirements: 2.1, 2.2, 6.1, 6.2_

- [ ] 1b. Create InputState struct and add to SpaceShip table
  - Create `InputState` struct in `spacetimedb/src/physics.rs` with `is_thrusting: bool` and `turn_direction: i8` fields
  - Add `input_state: InputState` field to `SpaceShip` table in `spacetimedb/src/lib.rs`
  - Update `on_connect` reducer to initialize `input_state` with `is_thrusting: false, turn_direction: 0`
  - Note: InputState is server-side only — it does NOT go in `solarance-shared` since physics must not depend on it (Requirement 5.5)
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

- [ ]\* 1.1 Write unit tests for MovementState structure
  - Test that all fields are present and have correct types
  - Test conversion functions preserve all fields (including acceleration and clamping fields)
  - _Requirements: 2.1, 2.2, 2.3, 2.4_

- [ ]\* 1.2 Write property test for MovementState serialization
  - **Property 10: MovementState serialization round-trip**
  - **Validates: Requirements 2.5, 2.6**

- [ ] 2. Extend ShipConfig with acceleration parameters
  - Add `max_acceleration: f32` field to ShipConfig table in `spacetimedb/src/lib.rs`
  - Add `max_angular_acceleration: f32` field to ShipConfig table in `spacetimedb/src/lib.rs`
  - Update `init` reducer to set default acceleration values (e.g., max_acceleration: 100.0, max_angular_acceleration: 180.0)
  - _Requirements: 4.1, 4.2, 4.4_

- [ ]\* 2.1 Write unit test for ShipConfig initialization
  - Test that init reducer creates ship configs with all acceleration fields populated
  - _Requirements: 4.4_

- [ ] 3. Implement linear acceleration in predict_movement
  - Modify `predict_movement` in `solarance-shared/src/physics.rs` to handle linear acceleration
  - Implement velocity calculation: v = v₀ + at
  - Implement position calculation for straight-line motion with acceleration: x = x₀ + v₀t + ½at²
  - Handle case where acceleration is zero (use existing logic)
  - _Requirements: 1.1, 1.2_

- [ ]\* 3.1 Write unit tests for linear acceleration
  - Test ship accelerating from rest reaches expected velocity and position
  - Test ship with initial velocity and acceleration
  - Test zero acceleration produces same results as before
  - _Requirements: 1.1, 1.2_

- [ ]\* 3.2 Write property test for linear kinematic equations
  - **Property 1: Linear acceleration kinematic equations**
  - **Validates: Requirements 1.1, 1.2**

- [ ] 4. Implement velocity clamping in predict_movement
  - Use `max_speed` from MovementState (already denormalized in Task 1) — no signature change needed
  - Calculate time when velocity reaches max_speed: t_clamp = (max_speed - v₀) / a
  - Implement piecewise position calculation: accelerate until t_clamp, then constant velocity
  - Clamp final velocity to max_speed
  - Handle edge case where velocity is already at max_speed
  - _Requirements: 1.5, 6.1, 6.3, 6.5_

- [ ]\* 4.1 Write unit tests for velocity clamping
  - Test velocity clamped to max_speed when acceleration would exceed it
  - Test position calculation with clamping
  - Test edge case where velocity starts at max_speed
  - _Requirements: 1.5, 6.3_

- [ ]\* 4.2 Write property test for velocity clamping
  - **Property 3: Velocity clamping**
  - **Validates: Requirements 1.5, 6.3**

- [ ]\* 4.3 Write property test for clamp time accuracy
  - **Property 8: Clamp time calculation accuracy**
  - **Validates: Requirements 9.2**

- [ ]\* 4.4 Write property test for continuity at clamp point
  - **Property 9: Smooth transition at velocity limit**
  - **Validates: Requirements 9.3**

- [ ] 5. Implement angular acceleration in predict_movement
  - Implement angular velocity calculation: ω = ω₀ + αt
  - Implement rotation calculation with angular acceleration: θ = θ₀ + ω₀t + ½αt²
  - Handle case where angular acceleration is zero (use existing logic)
  - _Requirements: 1.3, 1.4_

- [ ]\* 5.1 Write unit tests for angular acceleration
  - Test ship rotating with angular acceleration
  - Test zero angular acceleration produces same results as before
  - _Requirements: 1.3, 1.4_

- [ ]\* 5.2 Write property test for angular kinematic equations
  - **Property 2: Angular acceleration kinematic equations**
  - **Validates: Requirements 1.3, 1.4**

- [ ] 6. Implement angular velocity clamping in predict_movement
  - Use `max_turn_rate` from MovementState (already denormalized in Task 1) — no signature change needed
  - Calculate time when angular velocity reaches max_turn_rate
  - Implement piecewise rotation calculation
  - Clamp final angular velocity to max_turn_rate
  - _Requirements: 1.6, 6.2, 6.4_

- [ ]\* 6.1 Write unit tests for angular velocity clamping
  - Test angular velocity clamped to max_turn_rate
  - Test rotation calculation with clamping
  - _Requirements: 1.6, 6.4_

- [ ]\* 6.2 Write property test for angular velocity clamping
  - **Property 4: Angular velocity clamping**
  - **Validates: Requirements 1.6, 6.4**

- [ ] 7. Implement combined acceleration and turning
  - Handle case where both acceleration and angular_acceleration are non-zero
  - Implement arc motion with changing speed (may use numerical integration)
  - Ensure smooth trajectory when both accelerations are active
  - _Requirements: 1.7_

- [ ]\* 7.1 Write unit tests for combined motion
  - Test ship accelerating while turning produces curved path
  - Test position changes correctly with both accelerations
  - _Requirements: 1.7_

- [ ]\* 7.2 Write property test for combined acceleration and turning
  - **Property 5: Combined acceleration and turning**
  - **Validates: Requirements 1.7**

- [ ] 8. Checkpoint - Ensure physics tests pass
  - Run all unit tests and property tests for physics module
  - Verify predict_movement handles all cases correctly
  - Ask the user if questions arise

- [ ]\* 8.1 Write property test for backward compatibility
  - **Property 6: Backward compatibility with zero acceleration**
  - **Validates: Requirements 7.1**

- [ ]\* 8.2 Write property test for deterministic prediction
  - **Property 7: Deterministic prediction**
  - **Validates: Requirements 8.5**

- [ ] 9. Update existing server reducers for backward compatibility
  - Update `set_forward_thrust` and `set_turn_velocity` reducers to set acceleration fields to zero (maintain backward compatibility)
  - Ensure old reducers populate `max_speed` and `max_turn_rate` in MovementState from ShipConfig when updating
  - _Requirements: 7.2_

- [ ] 10. Implement set_thrust_input reducer
  - Create `set_thrust_input(ctx: &ReducerContext, is_thrusting: bool)` reducer in `spacetimedb/src/lib.rs`
  - Check if `ship.input_state.is_thrusting == is_thrusting` — return early if unchanged (Req 3.8, 10.8)
  - Get player's ship and ship config
  - Predict current position and velocity using predict_movement
  - Set acceleration based on is_thrusting: max_acceleration in direction of current rotation if true, 0 if false (ship coasts at current velocity)
  - Update SpaceShip: set `input_state.is_thrusting`, update `movement` fields (pos, velocity, acceleration, last_update_time)
  - Update database
  - _Requirements: 3.1, 3.2, 3.6, 3.7, 3.8, 10.8_

- [ ]\* 10.1 Extract and test thrust acceleration logic in shared crate
  - Extract the pure computation ("given is_thrusting, max_acceleration, and current rotation → compute new acceleration value") into a testable function in `solarance-shared`
  - Test is_thrusting=true sets acceleration to max_acceleration in direction of rotation
  - Test is_thrusting=false sets acceleration to zero and preserves velocity (coasting)
  - Note: SpacetimeDB module (`cdylib`) cannot run `cargo test` — all testable logic must live in the shared crate
  - _Requirements: 3.1, 3.2_

- [ ]\* 10.2 Write property test for thrust input (in shared crate)
  - **Property 11: Thrust input sets acceleration in direction of rotation**
  - Implemented as a pure function test in `solarance-shared` using extracted helper
  - **Validates: Requirements 3.1, 3.2**

- [ ]\* 10.3 Integration test for state synchronization (requires running SpacetimeDB)
  - **Property 13: Reducer synchronizes state before update**
  - Must be tested against a live SpacetimeDB instance — cannot be a `cargo test`
  - **Validates: Requirements 3.6**

- [ ]\* 10.4 Integration test for idempotent updates (requires running SpacetimeDB)
  - **Property 14: Idempotent input updates**
  - Must be tested against a live SpacetimeDB instance — cannot be a `cargo test`
  - **Validates: Requirements 3.8, 10.8**

- [ ] 11. Implement set_turn_input reducer
  - Create `set_turn_input(ctx: &ReducerContext, turn_direction: i8)` reducer in `spacetimedb/src/lib.rs`
  - Validate turn_direction is -1, 0, or 1
  - Check if `ship.input_state.turn_direction == turn_direction` — return early if unchanged (Req 3.8, 10.8)
  - Get player's ship and ship config
  - Predict current position and rotation using predict_movement
  - Set angular_acceleration based on turn_direction
  - Update SpaceShip: set `input_state.turn_direction`, update `movement` fields (pos, rotation, angular_velocity, angular_acceleration, last_update_time)
  - Update database
  - _Requirements: 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 10.8_

- [ ]\* 11.1 Extract and test turn acceleration logic in shared crate
  - Extract the pure computation ("given turn_direction and max_angular_acceleration → compute new angular acceleration value") into a testable function in `solarance-shared`
  - Test turn_direction=1 sets angular_acceleration to max_angular_acceleration
  - Test turn_direction=-1 sets angular_acceleration to negative max_angular_acceleration
  - Test turn_direction=0 sets angular_acceleration to zero
  - Test invalid turn_direction returns error
  - Note: SpacetimeDB module (`cdylib`) cannot run `cargo test` — all testable logic must live in the shared crate
  - _Requirements: 3.3, 3.4, 3.5_

- [ ]\* 11.2 Write property test for turn input (in shared crate)
  - **Property 12: Turn input sets angular acceleration**
  - Implemented as a pure function test in `solarance-shared` using extracted helper
  - **Validates: Requirements 3.3, 3.4, 3.5**

- [ ] 12. Update client_connected reducer to initialize input state
  - Note: InputState initialization was partially handled in Task 1b (on_connect sets is_thrusting=false, turn_direction=0)
  - Ensure all MovementState acceleration fields are initialized to zero
  - Ensure max_speed and max_turn_rate are copied from ShipConfig into MovementState
  - _Requirements: 5.3_

- [ ]\* 12.1 Verify client connection initialization (manual or integration test)
  - Verify new ships have input_state.is_thrusting=false and input_state.turn_direction=0
  - Verify all acceleration fields are zero
  - Verify max_speed and max_turn_rate in MovementState match ShipConfig values
  - Note: Must be verified via integration test against running SpacetimeDB, not `cargo test`
  - _Requirements: 5.3_

- [ ] 13. Checkpoint - Verify server reducer behavior
  - Run shared crate unit tests and property tests (`cargo test` in `solarance-shared`)
  - Manually test reducers against running SpacetimeDB instance (cdylib cannot run `cargo test`)
  - Verify reducers update database correctly
  - Verify backward compatibility with old reducers
  - Ask the user if questions arise

- [ ] 14. Update client input handling in main.rs
  - Add static variables or game state fields to track previous input state
  - Modify `handle_input` function to detect input state changes
  - Replace velocity calculations with boolean thrust state: is_thrusting = is_key_down(W) || is_key_down(Up)
  - Replace angular velocity calculations with turn direction: turn_direction = if is_key_down(D) { 1 } else if is_key_down(A) { -1 } else { 0 }
  - Call set_thrust_input only when is_thrusting changes
  - Call set_turn_input only when turn_direction changes
  - Update previous state after sending reducer calls
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6_

- [ ]\* 14.1 Write unit tests for client input handling
  - Test thrust key press calls set_thrust_input(true)
  - Test thrust key release calls set_thrust_input(false)
  - Test turn keys call set_turn_input with correct direction
  - Test no reducer calls when input unchanged
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6_

- [ ]\* 14.2 Write property test for no redundant calls
  - **Property 15: No redundant reducer calls**
  - **Validates: Requirements 10.6, 10.7**

- [ ] 15. Update client ship rendering to use new MovementState fields
  - Verify ShipManager correctly converts new MovementState fields (including acceleration, max_speed, max_turn_rate)
  - `predict_movement` already receives max_speed/max_turn_rate via MovementState — verify conversion includes these fields
  - Update any debug rendering to show acceleration values
  - _Requirements: 8.2_

- [ ] 16. Integration testing and validation
  - Test full flow: key press → reducer call → database update → client prediction → rendering
  - Test multiple ships with different acceleration values
  - Test switching between old and new input systems
  - Verify no visual glitches or discontinuities
  - Verify network traffic is minimal (only on input changes)
  - _Requirements: 7.4, 8.5_

- [ ]\* 16.1 Write integration tests
  - Test old and new reducers can be called on same ship
  - Test client can switch between velocity-based and acceleration-based input
  - _Requirements: 7.4_

- [ ] 17. Final checkpoint - Ensure all tests pass
  - Run complete test suite (unit tests + property tests)
  - Verify all 15 correctness properties are tested
  - Verify backward compatibility with existing system
  - Ask the user if questions arise

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Property tests validate universal correctness properties with minimum 100 iterations
- Unit tests validate specific examples and edge cases
- Checkpoints ensure incremental validation at key milestones
- The implementation maintains backward compatibility throughout the transition

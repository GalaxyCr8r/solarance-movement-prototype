# Design Document: Acceleration-Based Input System

## Overview

This design extends the existing dead reckoning physics system to support acceleration-based input commands. Instead of clients directly setting velocity values, they will send discrete input events (thrust on/off, turn left/right/off) to the server. The server will calculate acceleration based on these inputs and ship configuration, then use enhanced dead reckoning to predict position and velocity over time.

The key insight is that acceleration-based input provides more natural ship control: ships accelerate when thrust is applied and coast when released, rather than instantly jumping to a target velocity. This creates more engaging gameplay while maintaining the efficiency of dead reckoning (server only updates database when input state changes, not every frame).

### Design Goals

1. **Natural ship control**: Ships accelerate and decelerate smoothly based on button presses
2. **Efficient networking**: Server updates database only when input changes, not when velocity changes
3. **Shared prediction**: Client and server use identical physics code for synchronization
4. **Backward compatibility**: New system coexists with existing velocity-based reducers during transition
5. **Numerical stability**: Accurate predictions over varying time scales without drift

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         Client                               │
│  ┌──────────────┐      ┌─────────────────────────────────┐ │
│  │ Input Handler│─────▶│  Reducer Calls                  │ │
│  │ (main.rs)    │      │  - set_thrust_input()           │ │
│  └──────────────┘      │  - set_turn_input()             │ │
│                        └─────────────────────────────────┘ │
│                                     │                        │
│                                     ▼                        │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Ship Manager (ships.rs)                            │   │
│  │  - Subscribes to SpaceShip table                    │   │
│  │  - Calls predict_movement() for rendering           │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ SpacetimeDB SDK
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    SpacetimeDB Server                        │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Reducers (spacetimedb/src/lib.rs)                  │   │
│  │  - set_thrust_input(is_thrusting)                   │   │
│  │  - set_turn_input(turn_direction)                   │   │
│  │  - Calculates acceleration from input + ship config │   │
│  │  - Predicts current state before updating           │   │
│  │  - Updates database only on input change            │   │
│  └─────────────────────────────────────────────────────┘   │
│                              │                               │
│                              ▼                               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Tables                                              │   │
│  │  - SpaceShip (with extended MovementState)          │   │
│  │  - ShipConfig (with acceleration fields)            │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ Shared Physics
                              ▼
┌─────────────────────────────────────────────────────────────┐
│           solarance-shared/src/physics.rs                    │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  predict_movement(state, current_time)              │   │
│  │  - Handles constant acceleration                    │   │
│  │  - Handles velocity clamping                        │   │
│  │  - Handles combined acceleration + turning          │   │
│  │  - Used identically by client and server            │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **Input Event**: Player presses thrust key
2. **Client**: Detects key press, calls `set_thrust_input(true)`
3. **Server**: Receives reducer call
   - Predicts current position/velocity using dead reckoning
   - Sets acceleration based on ship's max_acceleration
   - Updates database with new state (synchronized position, new acceleration, input state)
4. **Database Update**: SpacetimeDB broadcasts update to all subscribed clients
5. **Client Prediction**: Client receives update, uses `predict_movement()` to render ship at current time
6. **Continuous Prediction**: Client continues predicting forward using acceleration until next input change

## Components and Interfaces

### 1. MovementState (Extended)

**Location**: `solarance-shared/src/physics.rs` and `spacetimedb/src/physics.rs`

```rust
#[derive(Clone, Copy, Debug)]
pub struct MovementState {
    // Existing fields
    pub pos: Vec2,
    pub velocity: f32,              // pixels per second
    pub rotation: f32,              // degrees
    pub angular_velocity: f32,      // degrees per second
    pub last_update_time: i64,      // microseconds

    // New fields for acceleration
    pub acceleration: f32,          // pixels per second²
    pub angular_acceleration: f32,  // degrees per second²

    // Velocity limits (copied from ShipConfig so predict_movement can clamp without needing config)
    pub max_speed: f32,             // pixels per second (velocity cap)
    pub max_turn_rate: f32,         // degrees per second (angular velocity cap)
}
```

**Rationale**:

- `acceleration` and `angular_acceleration` store the current acceleration rates
- `max_speed` and `max_turn_rate` are denormalized from ShipConfig so the shared `predict_movement` function can perform velocity clamping without needing access to the ShipConfig table
- Input state is deliberately excluded from MovementState — it belongs on SpaceShip as a separate concern (see InputState below)
- All fields must be in both shared and SpacetimeDB versions for synchronization

### 1b. InputState (New)

**Location**: `spacetimedb/src/physics.rs` (server-side only, not needed in shared crate)

```rust
#[derive(SpacetimeType, Clone, Copy, Debug)]
pub struct InputState {
    pub is_thrusting: bool,         // thrust button pressed
    pub turn_direction: i8,         // -1 (left), 0 (none), 1 (right)
}
```

The `SpaceShip` table gains an `input_state: InputState` field:

```rust
#[table(name = space_ship, public)]
pub struct SpaceShip {
    #[primary_key]
    pub entity_id: Identity,
    pub ship_config_id: u32,
    pub movement: MovementState,
    pub input_state: InputState,    // NEW: tracks current player input
}
```

**Rationale**:

- Input state is a game-logic concern, not a physics concern (Requirement 5.5)
- The shared physics crate should not depend on input state — it derives predictions solely from MovementState fields
- InputState lives server-side because the client reads it from the SpaceShip table via SpacetimeDB subscriptions
- Separating InputState from MovementState keeps the physics prediction function pure and testable

### 2. ShipConfig (Extended)

**Location**: `spacetimedb/src/lib.rs`

```rust
#[table(name = ship_config, public)]
pub struct ShipConfig {
    #[primary_key]
    pub ship_config_id: u32,

    // Existing fields
    pub max_speed: f32,              // pixels per second (velocity cap)
    pub max_turn_rate: f32,          // degrees per second (angular velocity cap)

    // New fields
    pub max_acceleration: f32,       // pixels per second² (acceleration rate)
    pub max_angular_acceleration: f32, // degrees per second² (angular acceleration rate)
}
```

**Rationale**:

- `max_acceleration` determines how quickly ship reaches max_speed
- `max_angular_acceleration` determines how quickly ship reaches max_turn_rate
- Existing max_speed and max_turn_rate become velocity caps, not instant values
- Different ship types can have different acceleration characteristics

### 3. Physics Prediction Function (Enhanced)

**Location**: `solarance-shared/src/physics.rs`

```rust
pub fn predict_movement(state: &MovementState, current_time: i64) -> (Vec2, f32)
```

**Enhanced Algorithm**:

```
1. Calculate time delta: dt = (current_time - last_update_time) / 1_000_000.0

2. Calculate velocity with acceleration:
   - v_final = v_initial + acceleration * dt
   - Clamp v_final to [0, max_speed]
   - If clamped, calculate t_clamp = (max_speed - v_initial) / acceleration

3. Calculate angular velocity with angular acceleration:
   - ω_final = ω_initial + angular_acceleration * dt
   - Clamp ω_final to [-max_turn_rate, max_turn_rate]
   - If clamped, calculate t_clamp_angular

4. Calculate position based on motion type:

   Case A: No acceleration, no angular acceleration
     → Use existing straight-line or arc logic

   Case B: Linear acceleration only (no turning)
     → If not clamped: x = x₀ + v₀*dt + ½*a*dt²
     → If clamped: x = x₀ + v₀*t_clamp + ½*a*t_clamp² + max_speed*(dt - t_clamp)

   Case C: Angular acceleration only (no linear motion)
     → θ = θ₀ + ω₀*dt + ½*α*dt²
     → Position unchanged

   Case D: Both accelerations (complex arc with changing speed)
     → Numerical integration or piecewise analytical solution
     → Split into segments if velocity clamping occurs

5. Return (new_position, new_rotation)
```

**Rationale**:

- Kinematic equations (v = v₀ + at, x = x₀ + v₀t + ½at²) handle constant acceleration
- Velocity clamping requires calculating exact time when limit is reached
- Combined acceleration + turning is most complex case, may need numerical integration
- Must handle all edge cases (zero acceleration, clamped velocity, etc.)

### 4. Server Reducers (New)

**Location**: `spacetimedb/src/lib.rs`

```rust
#[reducer]
pub fn set_thrust_input(ctx: &ReducerContext, is_thrusting: bool) -> Result<(), String>
```

**Algorithm**:

```
1. Find player's ship or return error
2. Check if input state actually changed:
   - If ship.input_state.is_thrusting == is_thrusting: return early (no update needed)
3. Get ship configuration for max_acceleration and max_speed
4. Predict current position and velocity using predict_movement()
5. Calculate new acceleration:
   - If is_thrusting: acceleration = max_acceleration (applied in direction of current rotation)
   - Else: acceleration = 0 (ship coasts at current velocity)
6. Update SpaceShip:
   - input_state.is_thrusting = is_thrusting
   - movement.pos = predicted_pos
   - movement.velocity = predicted_velocity (clamped to max_speed)
   - movement.acceleration = new_acceleration
   - movement.last_update_time = current_time
7. Update database
```

```rust
#[reducer]
pub fn set_turn_input(ctx: &ReducerContext, turn_direction: i8) -> Result<(), String>
```

**Algorithm**:

```
1. Validate turn_direction is -1, 0, or 1
2. Find player's ship or return error
3. Check if input state actually changed:
   - If ship.input_state.turn_direction == turn_direction: return early
4. Get ship configuration for max_angular_acceleration and max_turn_rate
5. Predict current position and rotation using predict_movement()
6. Calculate new angular acceleration:
   - If turn_direction == 1: angular_acceleration = max_angular_acceleration
   - If turn_direction == -1: angular_acceleration = -max_angular_acceleration
   - If turn_direction == 0: angular_acceleration = 0
7. Update SpaceShip:
   - input_state.turn_direction = turn_direction
   - movement.pos = predicted_pos
   - movement.rotation = predicted_rotation
   - movement.angular_velocity = predicted_angular_velocity (clamped to max_turn_rate)
   - movement.angular_acceleration = new_angular_acceleration
   - movement.last_update_time = current_time
8. Update database
```

**Rationale**:

- Reducers only update database when input state changes (efficient)
- Always predict current state before applying new acceleration (prevents jumps)
- Server is authoritative for acceleration values (prevents cheating)
- Early return when input hasn't changed avoids unnecessary database writes

### 5. Client Input Handler (Modified)

**Location**: `src/main.rs`

```rust
fn handle_input(ctx: &DbConnection)
```

**New Algorithm**:

```
1. Get player's ship from database
2. Track previous input state (static or in game state)
3. Determine current input state from keyboard:
   - is_thrusting = is_key_down(W) || is_key_down(Up)
   - turn_direction = if is_key_down(D) { 1 }
                      else if is_key_down(A) { -1 }
                      else { 0 }
4. Compare with previous state:
   - If is_thrusting changed: call set_thrust_input()
   - If turn_direction changed: call set_turn_input()
5. Update previous state
```

**Rationale**:

- Only send reducer calls when input actually changes (reduces network traffic)
- Simple boolean/enum state is easier to track than continuous velocity values
- Matches natural player mental model (button pressed vs released)

## Data Models

### MovementState Fields

| Field                | Type | Unit         | Description                                |
| -------------------- | ---- | ------------ | ------------------------------------------ |
| pos                  | Vec2 | pixels       | Current position (x, y)                    |
| velocity             | f32  | pixels/sec   | Current linear speed                       |
| rotation             | f32  | degrees      | Current heading (0° = up/north)            |
| angular_velocity     | f32  | degrees/sec  | Current turn rate                          |
| last_update_time     | i64  | microseconds | Timestamp of last state update             |
| acceleration         | f32  | pixels/sec²  | Current linear acceleration rate           |
| angular_acceleration | f32  | degrees/sec² | Current angular acceleration rate          |
| max_speed            | f32  | pixels/sec   | Velocity cap (denormalized from ShipConfig)|
| max_turn_rate        | f32  | degrees/sec  | Angular velocity cap (denormalized from ShipConfig) |

### InputState Fields (on SpaceShip, separate from MovementState)

| Field          | Type | Unit | Description                                 |
| -------------- | ---- | ---- | ------------------------------------------- |
| is_thrusting   | bool | -    | Whether thrust is currently active          |
| turn_direction | i8   | -    | Turn input: -1 (left), 0 (none), 1 (right) |

### ShipConfig Fields

| Field                    | Type | Unit         | Description                            |
| ------------------------ | ---- | ------------ | -------------------------------------- |
| ship_config_id           | u32  | -            | Unique identifier                      |
| max_speed                | f32  | pixels/sec   | Maximum linear velocity                |
| max_turn_rate            | f32  | degrees/sec  | Maximum angular velocity               |
| max_acceleration         | f32  | pixels/sec²  | Acceleration rate when thrusting       |
| max_angular_acceleration | f32  | degrees/sec² | Angular acceleration rate when turning |

### Example Values

For a basic ship:

- max_speed: 200.0 pixels/sec
- max_turn_rate: 90.0 degrees/sec
- max_acceleration: 100.0 pixels/sec² (reaches max speed in 2 seconds)
- max_angular_acceleration: 180.0 degrees/sec² (reaches max turn rate in 0.5 seconds)

## Correctness Properties

_A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees._

### Physics Prediction Properties

**Property 1: Linear acceleration kinematic equations**

_For any_ MovementState with non-zero acceleration and any time delta, when predicting movement, the resulting velocity and position should satisfy the kinematic equations: v = v₀ + at and x = x₀ + v₀t + ½at² (when velocity is not clamped).

**Validates: Requirements 1.1, 1.2**

**Property 2: Angular acceleration kinematic equations**

_For any_ MovementState with non-zero angular acceleration and any time delta, when predicting movement, the resulting angular velocity and rotation should satisfy the kinematic equations: ω = ω₀ + αt and θ = θ₀ + ω₀t + ½αt² (when angular velocity is not clamped).

**Validates: Requirements 1.3, 1.4**

**Property 3: Velocity clamping**

_For any_ MovementState where predicted velocity would exceed max_speed, the predict_movement function should clamp velocity to max_speed and calculate position using piecewise motion (accelerating until max_speed is reached, then constant velocity).

**Validates: Requirements 1.5, 6.3**

**Property 4: Angular velocity clamping**

_For any_ MovementState where predicted angular velocity would exceed max_turn_rate in magnitude, the predict_movement function should clamp angular velocity to max_turn_rate and calculate rotation using piecewise motion (accelerating until max_turn_rate is reached, then constant angular velocity).

**Validates: Requirements 1.6, 6.4**

**Property 5: Combined acceleration and turning**

_For any_ MovementState with both non-zero linear acceleration and non-zero angular acceleration, the predict_movement function should produce a curved trajectory where both speed and heading change over time.

**Validates: Requirements 1.7**

**Property 6: Backward compatibility with zero acceleration**

_For any_ MovementState with acceleration = 0 and angular_acceleration = 0, the new predict_movement function should produce identical results to the original velocity-based prediction.

**Validates: Requirements 7.1**

**Property 7: Deterministic prediction**

_For any_ MovementState and timestamp, calling predict_movement multiple times with the same inputs should always return identical position and rotation values.

**Validates: Requirements 8.5**

**Property 8: Clamp time calculation accuracy**

_For any_ MovementState where velocity clamping occurs, the calculated time when velocity reaches max_speed should satisfy: v₀ + a\*t_clamp = max_speed (within numerical tolerance).

**Validates: Requirements 9.2**

**Property 9: Smooth transition at velocity limit**

_For any_ MovementState where velocity reaches max_speed during prediction, the position function should be continuous (no discontinuities or jumps) at the clamping point.

**Validates: Requirements 9.3**

### Data Model Properties

**Property 10: MovementState serialization round-trip**

_For any_ valid MovementState with all fields populated (including acceleration fields), serializing to the database format and then deserializing should produce an equivalent MovementState with all fields preserved.

**Validates: Requirements 2.5, 2.6**

### Server Reducer Properties

**Property 11: Thrust input sets acceleration in direction of rotation**

_For any_ ship, when set_thrust_input is called with is_thrusting=true, the resulting MovementState should have acceleration equal to the ship's max_acceleration applied in the direction of the ship's current rotation; when called with is_thrusting=false, acceleration should be zero and velocity should be preserved (coasting).

**Validates: Requirements 3.1, 3.2**

**Property 12: Turn input sets angular acceleration**

_For any_ ship, when set_turn_input is called with turn_direction=1, the resulting MovementState should have angular_acceleration equal to max_angular_acceleration; with turn_direction=-1, it should be negative max_angular_acceleration; with turn_direction=0, it should be zero.

**Validates: Requirements 3.3, 3.4, 3.5**

**Property 13: Reducer synchronizes state before update**

_For any_ ship with non-zero velocity or acceleration, when an input reducer is called, the resulting MovementState position and velocity should reflect the predicted values at the current server time (not the old last_update_time).

**Validates: Requirements 3.6**

**Property 14: Idempotent input updates**

_For any_ ship, calling an input reducer twice with the same input value should result in only the first call updating the database; the second call should be a no-op.

**Validates: Requirements 3.8**

### Client Input Properties

**Property 15: No redundant reducer calls**

_For any_ sequence of frames where keyboard input state remains unchanged, the client should send at most one reducer call per input state change (no redundant calls when state is stable).

**Validates: Requirements 10.6, 10.7**

## Error Handling

### Physics Prediction Errors

1. **Invalid time delta**: If current_time < last_update_time, return the current state unchanged (no backward time travel)
2. **Uninitialized state**: If last_update_time == 0, return current state unchanged (ship not yet initialized)
3. **NaN or infinite values**: If any calculation produces NaN or infinity, log error and return last known good state
4. **Extreme time deltas**: If dt > 60 seconds, log warning (possible clock desync) but still calculate

### Server Reducer Errors

1. **Ship not found**: Return `Err("Ship not found")` if player has no ship
2. **Invalid ship config**: Return `Err("Ship configuration not found")` if ship_config_id is invalid
3. **Invalid turn direction**: Return `Err("Invalid turn direction, must be -1, 0, or 1")` if turn_direction is out of range
4. **Database update failure**: SpacetimeDB handles transaction rollback automatically on panic

### Client Input Errors

1. **Connection lost**: If reducer call fails due to disconnection, queue the input for retry when reconnected
2. **Ship not spawned**: If player's ship doesn't exist yet, ignore input (ship spawns on connection)

## Testing Strategy

### Dual Testing Approach

This feature requires both unit tests and property-based tests for comprehensive coverage:

- **Unit tests**: Verify specific examples, edge cases, and integration points
- **Property tests**: Verify universal properties across all inputs through randomization

Unit tests should focus on:

- Specific examples that demonstrate correct behavior (e.g., "ship at origin with 100 px/s² acceleration for 2 seconds reaches 200 px/s")
- Edge cases (zero acceleration, zero time delta, clamping at exactly max_speed)
- Physics prediction correctness (shared crate is fully testable)

Property tests should focus on:

- Universal mathematical properties (kinematic equations hold for all valid inputs)
- Invariants (determinism, continuity, round-trip preservation)
- Comprehensive input coverage through randomization

### SpacetimeDB Module Testing Constraints

**Important**: The SpacetimeDB module (`spacetimedb/`) compiles as `cdylib` and **cannot run `cargo test`**. This means:

- **No unit tests or property tests can live in `spacetimedb/src/`**
- Reducer logic that is purely computational (e.g., acceleration calculation, input validation) should be extracted into the `solarance-shared` crate where it can be unit tested
- Reducer integration testing (database updates, transaction behavior) must use **integration tests against a running SpacetimeDB instance** or manual testing
- The `solarance-shared` crate carries the bulk of the automated test burden

### Property-Based Testing Configuration

- **Library**: Use `quickcheck` crate for Rust property-based testing
- **Iterations**: Minimum 100 iterations per property test
- **Tagging**: Each property test must reference its design document property
- **Tag format**: `// Feature: acceleration-based-input, Property N: [property text]`

### Test Organization

```
solarance-shared/
  src/
    physics.rs
    physics_tests.rs          # Unit tests for specific cases
    physics_properties.rs     # Property-based tests

src/
  main.rs
  input_tests.rs            # Unit tests for client input handling
```

**Note**: Reducer-specific tests (Properties 11-14) must be implemented as either:
- Pure function tests in `solarance-shared` (by extracting reducer logic into testable functions)
- Integration tests against a live SpacetimeDB instance

### Key Test Cases

**Unit Tests**:

1. Ship accelerates from rest to max speed in expected time
2. Ship coasts at current speed when thrust is released (velocity preserved, acceleration set to zero)
3. Ship turns with angular acceleration
4. Velocity clamping at exactly max_speed
5. Zero time delta returns unchanged state
6. Backward time delta returns unchanged state
7. Client sends reducer call only when key state changes

**Property Tests** (one per correctness property):

1. Linear kinematic equations (Property 1)
2. Angular kinematic equations (Property 2)
3. Velocity clamping (Property 3)
4. Angular velocity clamping (Property 4)
5. Combined motion (Property 5)
6. Backward compatibility (Property 6)
7. Determinism (Property 7)
8. Clamp time accuracy (Property 8)
9. Continuity at clamp point (Property 9)
10. Serialization round-trip (Property 10)
11. Thrust input with direction (Property 11) — testable via extracted helper in shared crate
12. Turn input (Property 12) — testable via extracted helper in shared crate
13. State synchronization (Property 13) — requires integration test against running SpacetimeDB
14. Idempotency (Property 14) — requires integration test against running SpacetimeDB
15. No redundant calls (Property 15)

### Test Data Generation

For property-based tests, generate random:

- Positions: (-10000.0, 10000.0) pixels
- Velocities: (0.0, 500.0) pixels/sec
- Accelerations: (-200.0, 200.0) pixels/sec²
- Rotations: (0.0, 360.0) degrees
- Angular velocities: (-180.0, 180.0) degrees/sec
- Angular accelerations: (-360.0, 360.0) degrees/sec²
- Time deltas: (0.0, 10.0) seconds
- Max speeds: (50.0, 500.0) pixels/sec
- Max turn rates: (30.0, 180.0) degrees/sec

### Migration Testing

During transition from velocity-based to acceleration-based system:

1. Test that old reducers (set_forward_thrust, set_turn_velocity) still work
2. Test that new reducers (set_thrust_input, set_turn_input) work correctly
3. Test that both can be called on the same ship without conflicts
4. Test client can switch between old and new input handling

### Performance Testing

While not part of automated tests, manual performance testing should verify:

- Prediction calculations complete in < 1ms for typical cases
- No frame drops when rendering 100+ ships with acceleration
- Database updates remain infrequent (only on input changes)
- Network traffic is minimal (no redundant reducer calls)

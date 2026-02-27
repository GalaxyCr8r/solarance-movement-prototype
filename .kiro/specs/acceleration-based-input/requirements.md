# Requirements Document: Acceleration-Based Input System

## Introduction

This document specifies the requirements for extending the dead reckoning physics system to handle acceleration-based input commands. The current system uses dead reckoning to predict ship positions based on constant velocity and angular velocity, with the server updating the database only when these velocities change. This extension will allow clients to send discrete "button press" commands (thrust on/off, turn left/right on/off) to the server, which will use dead reckoning to predict position based on constant acceleration rather than just constant velocity.

## Glossary

- **Dead_Reckoning**: A technique for predicting an entity's current position and rotation based on its last known state and elapsed time, without requiring continuous server updates
- **Movement_State**: A data structure containing position, velocity, rotation, angular velocity, acceleration, angular acceleration, and last update timestamp
- **Input_State**: The current state of player input controls (thrust button pressed/released, turn direction)
- **Reducer**: A SpacetimeDB server-side function that processes client requests and updates the database transactionally
- **Ship_Config**: Configuration data for a ship type, including maximum speed, turn rate, acceleration, and angular acceleration
- **Kinematic_Equations**: Physics equations describing motion with constant acceleration: v = v₀ + at, x = x₀ + v₀t + ½at²
- **Arc_Motion**: Curved trajectory resulting from simultaneous linear and angular motion
- **Velocity_Clamping**: Limiting velocity to not exceed configured maximum values during acceleration
- **Client**: The game client application running on the player's machine
- **Server**: The SpacetimeDB backend that maintains authoritative game state

## Requirements

### Requirement 1: Acceleration-Based Physics Prediction

**User Story:** As a game developer, I want the physics system to predict ship movement based on constant acceleration, so that ships can smoothly accelerate and decelerate rather than instantly changing velocity.

#### Acceptance Criteria

1. WHEN predicting movement with non-zero linear acceleration, THE Physics_System SHALL calculate velocity using v = v₀ + at
2. WHEN predicting movement with non-zero linear acceleration, THE Physics_System SHALL calculate position using x = x₀ + v₀t + ½at²
3. WHEN predicting movement with non-zero angular acceleration, THE Physics_System SHALL calculate angular velocity using ω = ω₀ + αt
4. WHEN predicting movement with non-zero angular acceleration, THE Physics_System SHALL calculate rotation using θ = θ₀ + ω₀t + ½αt²
5. WHEN velocity reaches max_speed during acceleration, THE Physics_System SHALL compute the exact time at which the limit was reached, apply acceleration up to that point, and then apply constant-velocity motion for the remainder of the time step (see Requirement 6 for full clamping specification)
6. WHEN angular velocity reaches max_turn_rate during acceleration, THE Physics_System SHALL compute the exact time at which the limit was reached, apply angular acceleration up to that point, and then apply constant angular-velocity motion for the remainder of the time step (see Requirement 6 for full clamping specification)
7. WHEN both linear acceleration and angular acceleration are non-zero, THE Physics_System SHALL use numerical integration (e.g., small fixed-step Euler or RK4) to approximate the combined arc motion, since no closed-form solution exists for simultaneously changing speed and heading

### Requirement 2: Extended Movement State

**User Story:** As a game developer, I want the movement state to track acceleration values, so that the system can predict motion based on acceleration rather than just velocity.

#### Acceptance Criteria

1. THE Movement_State SHALL include a linear acceleration field measured in pixels per second squared
2. THE Movement_State SHALL include an angular acceleration field measured in degrees per second squared
3. THE Movement_State SHALL maintain existing velocity and angular velocity fields for current state
4. THE Movement_State SHALL maintain existing position, rotation, and last_update_time fields
5. WHEN serializing Movement_State to the database, THE Server SHALL include all acceleration fields
6. WHEN deserializing Movement_State from the database, THE Client SHALL receive all acceleration fields

### Requirement 3: Input-Based Server Reducers

**User Story:** As a player, I want to control my ship by pressing and releasing thrust and turn buttons, so that the ship accelerates and decelerates naturally rather than instantly changing speed.

#### Acceptance Criteria

1. WHEN a client calls set_thrust_input with is_thrusting=true, THE Server SHALL set linear acceleration to the ship's max_acceleration value in the direction of the ship's current rotation
2. WHEN a client calls set_thrust_input with is_thrusting=false, THE Server SHALL set linear acceleration to zero while preserving the ship's current velocity (the ship coasts at its current speed)
3. WHEN a client calls set_turn_input with turn_direction=1, THE Server SHALL set angular acceleration to the ship's max_angular_acceleration value
4. WHEN a client calls set_turn_input with turn_direction=-1, THE Server SHALL set angular acceleration to negative max_angular_acceleration value
5. WHEN a client calls set_turn_input with turn_direction=0, THE Server SHALL set angular acceleration to zero
6. WHEN processing input commands, THE Server SHALL use dead reckoning to predict current velocity and position before applying new acceleration
7. WHEN input state changes, THE Server SHALL update the database with new acceleration values and synchronized position/velocity
8. WHEN input state does not change, THE Server SHALL not update the database

### Requirement 4: Ship Configuration Extension

**User Story:** As a game designer, I want to configure acceleration rates for different ship types, so that ships can have different handling characteristics.

#### Acceptance Criteria

1. THE Ship_Config SHALL include a max_acceleration field measured in pixels per second squared
2. THE Ship_Config SHALL include a max_angular_acceleration field measured in degrees per second squared
3. THE Ship_Config SHALL maintain existing max_speed and max_turn_rate fields as velocity limits
4. WHEN initializing ship configurations, THE Server SHALL set reasonable default values for all acceleration fields
5. WHEN a ship accelerates, THE Server SHALL use the ship's configured max_acceleration value

### Requirement 5: Input State Tracking

**User Story:** As a game developer, I want the server to track current input state, so that it can calculate the correct acceleration based on which buttons are currently pressed.

#### Acceptance Criteria

1. THE SpaceShip table SHALL include an Input_State struct with an is_thrusting field indicating whether thrust is currently active
2. THE Input_State SHALL include a turn_direction field with values -1 (left), 0 (none), or 1 (right)
3. WHEN a client connects, THE Server SHALL initialize input state to no thrust and no turn
4. WHEN input state is updated, THE Server SHALL store the new input state on the SpaceShip record and update the Movement_State acceleration fields accordingly
5. THE Physics_System SHALL derive its predictions solely from Movement_State fields (position, velocity, acceleration, rotation, angular velocity, angular acceleration, timestamp) and SHALL NOT depend on Input_State directly

### Requirement 6: Velocity Clamping During Prediction

**User Story:** As a game developer, I want velocity to be clamped to maximum values during prediction, so that ships cannot exceed their configured speed limits even when accelerating.

#### Acceptance Criteria

1. WHEN predicted velocity exceeds max_speed, THE Physics_System SHALL clamp velocity to max_speed
2. WHEN predicted angular velocity exceeds max_turn_rate in magnitude, THE Physics_System SHALL clamp angular velocity to max_turn_rate
3. WHEN velocity is clamped, THE Physics_System SHALL calculate position as if acceleration stopped when velocity reached the limit
4. WHEN angular velocity is clamped, THE Physics_System SHALL calculate rotation as if angular acceleration stopped when angular velocity reached the limit
5. WHEN velocity is at maximum and acceleration is positive, THE Physics_System SHALL treat effective acceleration as zero for that time period

### Requirement 7: Backward Compatibility

**User Story:** As a game developer, I want the new acceleration system to coexist with existing code during transition, so that I can migrate incrementally without breaking the current system.

#### Acceptance Criteria

1. WHEN acceleration fields are zero, THE Physics_System SHALL produce identical results to the current velocity-based system
2. WHEN old reducers (set_forward_thrust, set_turn_velocity) are called, THE Server SHALL continue to function correctly
3. WHEN new reducers (set_thrust_input, set_turn_input) are called, THE Server SHALL use the acceleration-based system
4. THE Server SHALL support both old and new reducers simultaneously during transition
5. WHEN migrating to the new system, THE Client SHALL transition to calling the new reducers (set_thrust_input, set_turn_input) alongside the corresponding server-side changes (new reducers and extended ShipConfig)

### Requirement 8: Shared Physics Implementation

**User Story:** As a game developer, I want physics prediction logic to be shared between client and server, so that both predict movement identically and stay synchronized.

#### Acceptance Criteria

1. THE Physics_System SHALL implement acceleration-based prediction in the solarance-shared crate
2. WHEN the client predicts movement, THE Client SHALL use the same predict_movement function as the server
3. WHEN the server predicts movement, THE Server SHALL use the same predict_movement function as the client
4. THE Physics_System SHALL handle all edge cases (zero acceleration, clamped velocity, combined motion) identically on client and server
5. WHEN movement state is synchronized, THE Client and Server SHALL produce identical position and rotation predictions for the same timestamp

### Requirement 9: Numerical Stability

**User Story:** As a game developer, I want physics calculations to remain stable and accurate over time, so that ships don't drift or behave unpredictably due to numerical errors.

#### Acceptance Criteria

1. WHEN calculating position with acceleration, THE Physics_System SHALL use numerically stable algorithms
2. WHEN velocity is clamped during prediction, THE Physics_System SHALL calculate the exact time when clamping occurred
3. WHEN combining acceleration and turning, THE Physics_System SHALL handle the transition from accelerating to constant velocity smoothly
4. WHEN time deltas are very small (< 1ms), THE Physics_System SHALL produce accurate results without numerical instability
5. WHEN time deltas are large (> 10 seconds), THE Physics_System SHALL produce accurate results without accumulating error

### Requirement 10: Client Input Handling

**User Story:** As a player, I want my keyboard input to control ship acceleration, so that I can fly my ship by pressing and releasing thrust and turn keys.

#### Acceptance Criteria

1. WHEN the player presses the thrust key, THE Client SHALL call set_thrust_input with is_thrusting=true
2. WHEN the player releases the thrust key, THE Client SHALL call set_thrust_input with is_thrusting=false
3. WHEN the player presses the turn-left key, THE Client SHALL call set_turn_input with turn_direction=-1
4. WHEN the player presses the turn-right key, THE Client SHALL call set_turn_input with turn_direction=1
5. WHEN the player releases all turn keys, THE Client SHALL call set_turn_input with turn_direction=0
6. WHEN input state changes, THE Client SHALL send only one reducer call per state change
7. WHEN input state does not change between frames, THE Client SHALL not send redundant reducer calls
8. WHEN input state does not change between reducer calls, THE Server SHALL not make redundant updates to the SpaceShip record

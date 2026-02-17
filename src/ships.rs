use macroquad::color::WHITE;
use macroquad::text::draw_text;
use solarance_shared::physics;
use spacetimedb_sdk::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::module_bindings::*;
use crate::render::draw_ship;

/// Convert from generated bindings MovementState to shared physics MovementState
fn convert_movement_state(state: &crate::module_bindings::MovementState) -> physics::MovementState {
    physics::MovementState {
        pos: physics::Vec2 {
            x: state.pos.x,
            y: state.pos.y,
        },
        velocity: state.velocity,
        rotation: state.rotation,
        angular_velocity: state.angular_velocity,
        last_update_time: state.last_update_time,
    }
}

/// Client-side ship data with dead reckoning support
#[derive(Clone, Debug)]
pub struct ClientShip {
    pub entity_id: Identity,
    pub ship_config_id: u32,
    pub movement: physics::MovementState,
}

impl ClientShip {
    /// Calculate current position and rotation based on elapsed time
    pub fn predict_current(&self, current_time_micros: i64) -> (physics::Vec2, f32) {
        physics::predict_movement(&self.movement, current_time_micros)
    }
}

/// Clock synchronization with server
#[derive(Clone, Debug)]
pub struct ClockSync {
    server_offset_micros: i64, // server_time - client_time
}

impl ClockSync {
    pub fn new() -> Self {
        Self {
            server_offset_micros: 0,
        }
    }

    /// Update offset based on server timestamp
    pub fn update_from_server(&mut self, server_time_micros: i64) {
        let client_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64;

        self.server_offset_micros = server_time_micros - client_time;
    }

    /// Get current server time estimate
    pub fn get_server_time(&self) -> i64 {
        let client_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64;

        client_time + self.server_offset_micros
    }
}

/// Thread-safe ship manager for dead reckoning
#[derive(Clone)]
pub struct ShipManager {
    ships: Arc<RwLock<HashMap<Identity, ClientShip>>>,
    clock_sync: Arc<RwLock<ClockSync>>,
}

impl ShipManager {
    pub fn new() -> Self {
        Self {
            ships: Arc::new(RwLock::new(HashMap::new())),
            clock_sync: Arc::new(RwLock::new(ClockSync::new())),
        }
    }

    /// Sync ships from SpacetimeDB tables
    pub fn sync_from_db(&self, db: &RemoteTables) {
        let mut ships = self.ships.write().unwrap();
        let mut clock_sync = self.clock_sync.write().unwrap();

        // Get current ships from database
        let db_ships: HashMap<Identity, ClientShip> = db
            .space_ship()
            .iter()
            .map(|ship| {
                // Update clock sync from server timestamp
                //clock_sync.update_from_server(ship.movement.last_update_time);

                (
                    ship.entity_id,
                    ClientShip {
                        entity_id: ship.entity_id,
                        ship_config_id: ship.ship_config_id,
                        movement: convert_movement_state(&ship.movement),
                    },
                )
            })
            .collect();

        // Remove ships that no longer exist
        ships.retain(|id, _| db_ships.contains_key(id));

        // Add new ships and update existing ones
        for (id, ship) in db_ships {
            ships.insert(id, ship);
        }
    }

    pub fn render(&self) {
        let ships = self.ships.read().unwrap();
        let clock_sync = self.clock_sync.read().unwrap();
        let current_time_micros = clock_sync.get_server_time();

        for (_eid, ship) in ships.iter() {
            let (pos, rotation) = ship.predict_current(current_time_micros);
            draw_ship(pos.x, pos.y, rotation.to_radians());
            draw_text(
                &format!("{}", (current_time_micros - ship.movement.last_update_time)),
                pos.x,
                pos.y,
                20.0,
                WHITE,
            );
        }
    }

    /// Add or update a single ship
    pub fn upsert_ship(&self, ship: SpaceShip) {
        let mut ships = self.ships.write().unwrap();
        ships.insert(
            ship.entity_id,
            ClientShip {
                entity_id: ship.entity_id,
                ship_config_id: ship.ship_config_id,
                movement: convert_movement_state(&ship.movement),
            },
        );
    }

    /// Remove a ship by entity ID
    pub fn remove_ship(&self, entity_id: &Identity) {
        let mut ships = self.ships.write().unwrap();
        ships.remove(entity_id);
    }

    /// Get a snapshot of all ships
    pub fn get_all(&self) -> Vec<ClientShip> {
        let ships = self.ships.read().unwrap();
        ships.values().cloned().collect()
    }

    /// Get a specific ship by entity ID
    pub fn get_ship(&self, entity_id: &Identity) -> Option<ClientShip> {
        let ships = self.ships.read().unwrap();
        ships.get(entity_id).cloned()
    }

    /// Get count of ships
    pub fn count(&self) -> usize {
        let ships = self.ships.read().unwrap();
        ships.len()
    }
}

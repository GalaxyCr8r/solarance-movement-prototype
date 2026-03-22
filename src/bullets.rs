use macroquad::color::WHITE;
use macroquad::text::draw_text;
use solarance_shared::physics;
use spacetimedb_sdk::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::render::*;
use crate::utilities::*;
use crate::{module_bindings::*, render};

/// Client-side bullet data with dead reckoning support
#[derive(Clone, Debug)]
pub struct ClientBullet {
    pub entity_id: u32,
    pub movement: physics::MovementState,
}

impl ClientBullet {
    /// Calculate current position and rotation based on elapsed time
    pub fn predict_current(&self, current_time_micros: i64) -> (physics::Vec2, f32) {
        physics::predict_movement(&self.movement, current_time_micros)
    }
}

/// Thread-safe bullet manager for dead reckoning
#[derive(Clone)]
pub struct BulletManager {
    bullets: Arc<RwLock<HashMap<u32, ClientBullet>>>,
    server_offset_micros: i64, // server_time - client_time
}

impl BulletManager {
    pub fn new() -> Self {
        Self {
            bullets: Arc::new(RwLock::new(HashMap::new())),
            server_offset_micros: 0,
        }
    }

    /// Sync bullets from SpacetimeDB tables
    pub fn sync_from_db(&self, db: &RemoteTables) {
        let mut bullets = self.bullets.write().unwrap();

        // Get current bullets from database
        let db_bullets: HashMap<u32, ClientBullet> = db
            .current_sector_bullets()
            .iter()
            .map(|b| {
                let movement = convert_movement_state(&b.movement);
                (
                    b.id,
                    ClientBullet {
                        entity_id: b.id,
                        movement,
                    },
                )
            })
            .collect();

        // Update server offset based on first bullet's timestamp
        // if let Some(first_bullet) = db_bullets.values().next() {
        //     let client_time = std::time::SystemTime::now()
        //         .duration_since(std::time::UNIX_EPOCH)
        //         .unwrap()
        //         .as_micros() as i64;
        //     self.server_offset_micros = first_bullet.movement.last_update_time - client_time;
        // }

        // Replace the entire map with the database version
        *bullets = db_bullets;
    }

    /// Render all bullets
    pub fn render(&self) {
        let bullets = self.bullets.read().unwrap();
        let current_time_micros = get_server_time(self.server_offset_micros);

        for (_eid, bullet) in bullets.iter() {
            let (pos, rotation) = bullet.predict_current(current_time_micros);
            render::draw_bullet(pos, rotation.to_radians());
        }
    }
}

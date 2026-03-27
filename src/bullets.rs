use solarance_shared::physics;
use spacetimedb_sdk::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::utilities::*;
use crate::{module_bindings::*, render};

/// Client-side bullet data with dead reckoning support
#[derive(Clone, Debug)]
pub struct ClientBullet {
    pub entity_id: u32,
    pub player_id: Identity,
    pub movement: physics::MovementState,
    pub lifetime: i64,
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
            .filter_map(|b| {
                let movement = convert_movement_state(&b.movement);
                if b.lifetime < get_server_time(self.server_offset_micros) {
                    return None;
                }
                Some((
                    b.id,
                    ClientBullet {
                        entity_id: b.id,
                        player_id: b.player_id,
                        movement,
                        lifetime: b.lifetime,
                    },
                ))
            })
            .collect();

        // Replace the entire map with the database version
        *bullets = db_bullets;
    }

    /// Check for bullet-ship collisions each frame.
    ///
    /// Ships are modelled as circles with a 32 px radius. For every bullet
    /// currently tracked, its dead-reckoned position is compared against every
    /// ship's dead-reckoned position. If the distance is within the threshold,
    /// `submit_hit` is called on the server and the bullet is removed from the
    /// local cache so we don't fire duplicate reports before the DB syncs.
    pub fn check_collisions(&self, ship_manager: &crate::ships::ShipManager, ctx: &DbConnection) {
        const SHIP_RADIUS: f32 = 32.0;
        const HIT_DIST_SQ: f32 = SHIP_RADIUS * SHIP_RADIUS;

        let now_micros = get_server_time(self.server_offset_micros);
        let hit_at = Timestamp::from_micros_since_unix_epoch(now_micros);

        // Snapshot ships so we don't hold a lock during the reducer call.
        let ships = ship_manager.get_all();

        let mut bullet_ids_to_remove: Vec<u32> = Vec::new();

        {
            let bullets = self.bullets.read().unwrap();

            'bullet_loop: for (bullet_id, bullet) in bullets.iter() {
                if now_micros > bullet.lifetime {
                    bullet_ids_to_remove.push(*bullet_id);
                    continue 'bullet_loop;
                }

                let (bullet_pos, _) = bullet.predict_current(now_micros);

                for ship in &ships {
                    if ship.state.player_id == bullet.player_id {
                        continue;
                    }

                    let (ship_pos, _) = ship.predict_current(now_micros);

                    let dx = bullet_pos.x - ship_pos.x;
                    let dy = bullet_pos.y - ship_pos.y;
                    let dist_sq = dx * dx + dy * dy;

                    if dist_sq <= HIT_DIST_SQ {
                        let manager = self.clone();
                        let b_id = *bullet_id;
                        let result = ctx.reducers().submit_hit_then(
                            hit_at,
                            SpaceShipId {
                                value: ship.entity_id,
                            },
                            BulletId { value: b_id },
                            move |_db, result| match result {
                                Ok(_) => {
                                    println!("Hit submitted successfully");
                                    // Call remove_bullet from inside this callback function
                                    manager.remove_bullet(b_id);
                                }
                                Err(e) => {
                                    println!("Error submitting hit: {}", e);
                                }
                            },
                        );
                        if let Err(e) = result {
                            println!("Error submitting hit: {}", e);
                        }
                        // Only report one hit per bullet (first ship wins).
                        //bullet_ids_to_remove.push(*bullet_id);
                        continue 'bullet_loop;
                    }
                }
            }
        }

        // Remove locally so duplicates aren't sent on the next frame.
        if !bullet_ids_to_remove.is_empty() {
            let mut bullets = self.bullets.write().unwrap();
            for id in bullet_ids_to_remove {
                bullets.remove(&id);
            }
        }
    }

    fn remove_bullet(&self, bullet_id: u32) {
        let mut bullets = self.bullets.write().unwrap();
        bullets.remove(&bullet_id);
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

use spacetimedb::*;
use spacetimedsl::*;

use crate::physics;

#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub enum VisitedStatus {
    /// The player has not visited this system or sector.
    Unvisited,
    /// The player has observed this system or sector, either via intel report or sensors.
    Observed,
    /// The player has visited this system or sector.
    Visited,
}

impl VisitedStatus {
    pub fn is_visible(&self) -> bool {
        match self {
            VisitedStatus::Observed => true,
            VisitedStatus::Visited => true,
            _ => false,
        }
    }
}

#[table(accessor = ship_config, public)]
pub struct ShipConfig {
    #[primary_key]
    pub ship_config_id: u32,
    pub max_health: u16,
    pub max_speed: f32,                // meters per second
    pub max_turn_rate: f32,            // degrees per second
    pub max_acceleration: f32,         // meters per second²
    pub max_angular_acceleration: f32, // degrees per second²
}

#[table(accessor = space_ship)]
pub struct SpaceShip {
    #[primary_key]
    pub entity_id: Identity,
    #[index(btree)]
    pub sector_id: u64,

    pub ship_config_id: u32,
    pub health: f32,
    pub movement: physics::MovementState,
    pub input_state: physics::InputState,
}

/// Cargo crates can be picked up by players and
// #[table(accessor = cargo_crate)]
// pub struct CargoCrate {
//     #[primary_key]
//     #[auto_inc]
//     pub id: u64,

//     #[index(btree)]
//     pub sector_id: u64,

//     pub item_def_id: u32,
//     pub item_amount: u32,
//     pub movement: physics::MovementState,
// }

#[table(accessor = systems)]
pub struct System {
    #[primary_key]
    #[auto_inc]
    pub id: u32,
    pub name: String,
}

#[table(accessor = sectors,
index(accessor = position, btree(columns=[x,y])))]
pub struct Sector {
    #[primary_key]
    pub id: u64,
    #[index(btree)]
    pub system_id: u32,
    pub x: i32,
    pub y: i32,
    /// If true, this sector contains a station/gate and is visible
    /// to anyone who has visited the system.
    pub is_public: bool,
}

/// Tracks where a player's ship is currently located.
#[table(accessor = player_state)]
pub struct PlayerState {
    #[primary_key]
    pub player_id: Identity,
    pub current_system_id: u32,
    pub current_sector_id: u64,
}

/// Private relationship table: Who has visited which system.
#[table(accessor = visited_systems)]
pub struct VisitedSystem {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub player_id: Identity,
    pub system_id: u32,
    pub visited_status: VisitedStatus,
}

/// Private relationship table: Who has visited which specific sector.
#[table(accessor = visited_sectors)]
pub struct VisitedSector {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub player_id: Identity,
    #[index(btree)]
    pub sector_id: u64,
    pub visited_status: VisitedStatus,
}

#[derive(SpacetimeType, Clone, Debug)]
pub enum EventType {
    Bullet,
    Explosion,
    Warpgate,
}

/// A short-lived event used to broadcast visual effects to players in a sector.
#[table(accessor = damage_event, public, event)]
pub struct DamageEvent {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[index(btree)]
    pub sector_id: u64,
    pub event_type: EventType,
    pub pos: physics::Vec2,
    /// The timestamp when the event occurred (in microseconds).
    pub timestamp: i64,
}

use spacetimedb::*;
use spacetimedsl::*;

use crate::tables::*;

// --- Views ---

/// View: Returns all ships in the player's current sector.
#[view(accessor = my_player_state, public)]
pub fn my_player_state(ctx: &ViewContext) -> Vec<PlayerState> {
    match ctx.db.player_state().player_id().find(ctx.sender()) {
        Some(p) => vec![p],
        None => Vec::new(), // Player hasn't joined/initialized
    }
}

/// View: Returns all ships in the player's current sector.
#[view(accessor = current_sector_ships, public)]
pub fn current_sector_ships(ctx: &ViewContext) -> Vec<SpaceShip> {
    // 1. Get the player's current system ID
    let player = match ctx.db.player_state().player_id().find(ctx.sender()) {
        Some(p) => p,
        None => return Vec::new(), // Player hasn't joined/initialized
    };

    let current_sector_id = player.current_sector_id;

    // 2. Filter sectors in this system
    ctx.db
        .space_ship()
        .sector_id()
        .filter(current_sector_id)
        .collect()
}

/// View: Returns all sectors in the player's current system that they are
/// authorized to see (either because they visited them or they are public).
#[view(accessor = current_system_visible_sectors, public)]
pub fn current_system_visible_sectors(ctx: &ViewContext) -> Vec<Sector> {
    // 1. Get the player's current system ID
    let player = match ctx.db.player_state().player_id().find(ctx.sender()) {
        Some(p) => p,
        None => return Vec::new(), // Player hasn't joined/initialized
    };

    let current_sys_id = player.current_system_id;

    // 2. Filter sectors in this system
    ctx.db
        .sectors()
        .system_id()
        .filter(current_sys_id)
        .filter(|sector| {
            // Logic: Visible if the sector is marked public...
            if sector.is_public {
                return true;
            }
            // ...OR if the player has a record in visited_sectors for this ID.
            ctx.db
                .visited_sectors()
                .player_id()
                .filter(ctx.sender())
                .any(|v| v.sector_id == sector.id && v.visited_status.is_visible())
        })
        .collect()
}

/// View: Returns the full System details for every system the player has ever visited.
#[view(accessor = my_visited_systems, public)]
pub fn my_visited_systems(ctx: &ViewContext) -> Vec<System> {
    ctx.db
        .visited_systems()
        .player_id()
        .filter(ctx.sender())
        .filter(|sys| sys.visited_status.is_visible())
        .flat_map(|v| ctx.db.systems().id().find(v.system_id))
        .collect()
}
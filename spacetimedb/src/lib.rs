use spacetimedb::*;
use spacetimedsl::*;

mod physics;

mod tables;
use tables::*;

mod views;

mod reducers;

#[reducer(init)]
pub fn init(ctx: &ReducerContext) {
    // Seed initial ship types
    ctx.db.ship_config().insert(ShipConfig {
        ship_config_id: 1,
        max_health: 100,
        max_speed: 150.0,
        max_turn_rate: 80.0,
        max_acceleration: 100.0,
        max_angular_acceleration: 180.0,
    });

    ctx.db.sectors().insert(Sector {
        id: 1,
        system_id: 1,
        is_public: true,
        x: 0,
        y: 0,
    });

    ctx.db.sectors().insert(Sector {
        id: 2,
        system_id: 1,
        is_public: true,
        x: 5,
        y: 0,
    });

    ctx.db.sectors().insert(Sector {
        id: 3,
        system_id: 1,
        is_public: false,
        x: 15,
        y: 0,
    });

    ctx.db.systems().insert(System {
        id: 1,
        name: "Sol".to_string(),
    });

    ctx.db.systems().insert(System {
        id: 2,
        name: "Alpha Centauri".to_string(),
    });

    ctx.db.systems().insert(System {
        id: 3,
        name: "Tau Ceti".to_string(),
    });

    ctx.db.sectors().insert(Sector {
        id: 20,
        system_id: 2,
        is_public: true,
        x: 0,
        y: 0,
    });

    ctx.db.sectors().insert(Sector {
        id: 30,
        system_id: 3,
        is_public: true,
        x: 0,
        y: 0,
    });
}

#[reducer(client_connected)]
pub fn on_connect(_ctx: &ReducerContext) {
    //
}

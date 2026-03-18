use spacetimedb::*;
use spacetimedsl::*;

mod physics;

mod tables;
use tables::*;

mod views;

mod reducers;

#[reducer(init)]
pub fn init(ctx: &ReducerContext) {
    let dsl = spacetimedsl::dsl(ctx);

    // Seed initial ship types
    dsl.create_ship_config(CreateShipConfig {
        id: 1,
        max_health: 100,
        max_speed: 150.0,
        max_turn_rate: 80.0,
        max_acceleration: 100.0,
        max_angular_acceleration: 180.0,
    });

    dsl.create_sector(CreateSector {
        id: 1,
        system_id: SystemId::new(1),
        is_public: true,
        x: 0,
        y: 0,
    });

    dsl.create_sector(CreateSector {
        id: 2,
        system_id: SystemId::new(1),
        is_public: true,
        x: 5,
        y: 0,
    });

    dsl.create_sector(CreateSector {
        id: 3,
        system_id: SystemId::new(1),
        is_public: false,
        x: 15,
        y: 0,
    });

    dsl.create_system(CreateSystem {
        name: "Sol".to_string(),
    });

    dsl.create_system(CreateSystem {
        name: "Alpha Centauri".to_string(),
    });

    dsl.create_system(CreateSystem {
        name: "Tau Ceti".to_string(),
    });

    dsl.create_sector(CreateSector {
        id: 20,
        system_id: SystemId::new(2),
        is_public: true,
        x: 0,
        y: 0,
    });

    dsl.create_sector(CreateSector {
        id: 30,
        system_id: SystemId::new(3),
        is_public: true,
        x: 0,
        y: 0,
    });
}

#[reducer(client_connected)]
pub fn on_connect(_ctx: &ReducerContext) {
    //
}

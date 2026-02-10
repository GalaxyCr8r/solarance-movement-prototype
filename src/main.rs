mod module_bindings;
use macroquad::prelude::{collections::storage, *};
use macroquad::window::Conf;
use module_bindings::*;
use std::env;
use std::io::{self, Read};

use spacetimedb_sdk::{DbContext, Table, Timestamp};

/// Configures the game window properties including title, dimensions, and resizability
fn window_conf() -> Conf {
    Conf {
        window_title: "Solarance:Beginnings".to_owned(),
        window_width: 1600,
        window_height: 900,
        window_resizable: false,
        fullscreen: false,
        ..Default::default()
    }
}

#[egui_macroquad::macroquad::main(window_conf)]
async fn main() -> Result<(), macroquad::Error> {
    set_pc_assets_folder("assets");
    clear_background(BLACK);
    next_frame().await;
    connect();
    Ok(())
}

fn connect() {
    // The URI of the SpacetimeDB instance hosting our chat module.
    let host: String = env::var("SPACETIMEDB_HOST").unwrap_or("http://localhost:3000".to_string());

    // The module name we chose when we published our module.
    let db_name: String =
        env::var("SPACETIMEDB_DB_NAME").unwrap_or("solarance-movement-prototype".to_string());

    // Connect to the database
    let conn = DbConnection::builder()
        .with_module_name(db_name)
        .with_uri(host)
        .on_connect(|_, _, _| {
            println!("Connected to SpacetimeDB");
        })
        .on_connect_error(|_ctx, e| {
            eprintln!("Connection error: {:?}", e);
            std::process::exit(1);
        })
        .build()
        .expect("Failed to connect");

    conn.run_threaded();
}

// fn main() {

//     // Subscribe to the person table
//     conn.subscription_builder()
//         .on_applied(|_ctx| println!("Subscripted to the player_ship table"))
//         .on_error(|_ctx, e| {
//             eprintln!("There was an error when subscribing to the player_ship table: {e}")
//         })
//         .subscribe(["SELECT * FROM player_ship"]);

//     // Register a callback for when rows are inserted into the person table
//     conn.db().player_ship().on_insert(|_ctx, player_ship| {
//         println!("New player_ship: eid{}", player_ship.entity_id);
//     });

//     println!("Press any key to exit...");

//     let _ = io::stdin().read(&mut [0u8]).unwrap();
// }

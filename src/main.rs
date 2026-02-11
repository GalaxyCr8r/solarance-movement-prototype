mod module_bindings;
use macroquad::math::Vec2;
use macroquad::prelude::{collections::storage, *};
use macroquad::window::Conf;
use module_bindings::*;
use std::env;
use std::io::{self, Read};

use spacetimedb_sdk::{DbContext, Table, Timestamp};

pub struct GameState<'a> {
    // Game-Wide States
    pub done: bool,
    pub ctx: &'a DbConnection,

    // Display States
    pub camera: Camera2D,
    pub bg_camera: Camera2D,
    // GUI States
}

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

    let mut game_state = GameState {
        done: false,
        ctx: &connect(),
        camera: Camera2D::from_display_rect(Rect {
            x: 0.0,
            y: 0.0,
            w: screen_width(),
            h: screen_height(),
        }),
        bg_camera: Camera2D::from_display_rect(Rect {
            x: 0.0,
            y: 0.0,
            w: screen_width(),
            h: screen_height(),
        }),
    };
    game_state.camera.zoom.y *= -1.0;
    game_state.bg_camera.zoom.y *= -1.0;

    loop {
        clear_background(BLACK);

        game_state.camera.target = Vec2::ZERO; //get_player_transform_vec2(&ctx, Vec2::ZERO);
        set_camera(&game_state.camera);

        draw_text("Henlo!", 0.0, 0.0, 42.0, WHITE);

        egui_macroquad::ui(|egui_ctx| {
            egui::SidePanel::left("left_panel").show(egui_ctx, |ui| {
                ui.heading("Solarance:Beginnings");
            });
        });

        egui_macroquad::draw();
        next_frame().await;

        if game_state.done {
            let _ = game_state.ctx.disconnect();
            break;
        }
    }

    Ok(())
}

fn connect() -> DbConnection {
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

    conn
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

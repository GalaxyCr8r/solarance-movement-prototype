mod module_bindings;
use macroquad::math::Vec2;
use macroquad::prelude::{collections::storage, *};
use macroquad::window::Conf;
use module_bindings::*;
use std::env;
use std::io::{self, Read};

use spacetimedb_sdk::{DbContext, Table, Timestamp};

pub mod resources;

use resources::Resources;

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

    let resources = resources::Resources::new().await?;
    storage::store(resources);

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

    let mut side_panel_rect = egui::Rect::ZERO;

    loop {
        clear_background(BLACK);

        // Focus camera on current target (usually the player's ship)
        game_state.camera.target = Vec2::ZERO; //get_player_transform_vec2(&ctx, Vec2::ZERO);

        // Offset camera to account for side panel
        game_state.camera.target.x -= side_panel_rect.right() / 2.0;
        set_camera(&game_state.camera);

        draw_ship(0.0, 0.0, 0.0);

        draw_ship(0.0, 100.0, 3.14 / 2.0);

        draw_ship(0.0, 200.0, -3.14 / 2.0);

        draw_text("Henlo!", 0.0, 0.0, 16.0, WHITE);

        draw_text(
            format!("Side Panel Rect: {}", side_panel_rect).as_str(),
            0.0,
            42.0,
            42.0,
            WHITE,
        );

        egui_macroquad::ui(|egui_ctx| {
            side_panel_rect = egui::SidePanel::left("left_panel")
                .show(egui_ctx, |ui| {
                    ui.heading("Solarance:Beginnings");
                })
                .response
                .rect;
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

fn draw_ship(x: f32, y: f32, rotation_radians: f32) {
    let resources = storage::get::<Resources>();
    let ship_texture = &resources.ship_textures.get("lc.phalanx").unwrap();
    draw_texture_ex(
        ship_texture,
        x,
        y,
        WHITE,
        DrawTextureParams {
            rotation: rotation_radians,
            ..Default::default()
        },
    );
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

use macroquad::math::Vec2;
use macroquad::prelude::{collections::storage, *};
use macroquad::window::Conf;

use spacetimedb_sdk::*;
use std::result::Result;

pub mod resources;
use resources::Resources;

mod module_bindings;
use module_bindings::*;
mod gui_side_panel;
mod render;
use render::*;
mod connection;

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

    let resources = Resources::new().await?;
    storage::store(resources);

    clear_background(BLACK);
    next_frame().await;

    let mut game_state = GameState {
        done: false,
        ctx: &connection::connect(),
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

        // Iterate through the PlayerShip ctx table and draw each ship
        for obj in game_state.ctx.db.space_ship().iter() {
            let pos = obj.movement.pos;
            draw_ship(pos.x, pos.y, obj.movement.rotation);
        }

        //draw_text("Henlo!", 0.0, 0.0, 16.0, WHITE);

        egui_macroquad::ui(|egui_ctx| {
            side_panel_rect = gui_side_panel::draw_side_panel_contents(egui_ctx);
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

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

        handle_input(game_state.ctx);

        if game_state.done {
            let _ = game_state.ctx.disconnect();
            break;
        }
    }

    Ok(())
}

/// Handles player input and calls the proper reducers if the controls have changed.
fn handle_input(ctx: &DbConnection) {
    let player_ship = {
        match ctx.db.space_ship().entity_id().find(&ctx.identity()) {
            Some(ship) => ship,
            None => return,
        }
    };

    let mut changed = false;
    let mut new_angular_velocity = player_ship.movement.angular_velocity;
    let mut new_velocity = player_ship.movement.velocity;

    if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
        new_angular_velocity += 0.42;
        changed = true;
    }
    if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
        new_angular_velocity -= 0.42;
        changed = true;
    }
    if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
        new_velocity -= 1.337;
        changed = true;
    } //else {
      //     if controller.down {
      //         controller.down = false;
      //         changed = true;
      //     }
      // }
    if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
        new_velocity += 1.337;
        changed = true;
    } //else {
      //     if controller.up {
      //         controller.up = false;
      //         changed = true;
      //     }
      // }

    if changed {
        ctx.reducers().set_forward_thrust(new_velocity);
        ctx.reducers().set_turn_velocity(new_angular_velocity);
    }
}

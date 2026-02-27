use macroquad::math::Vec2;
use macroquad::prelude::{collections::storage, *};
use macroquad::window::Conf;

use spacetimedb_sdk::*;
use std::result::Result;

pub mod resources;
use resources::Resources;

mod module_bindings;
use module_bindings::*;
mod connection;
mod gui_side_panel;
mod render;
mod ships;
use ships::*;

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

    let ship_manager = ShipManager::new();

    let mut side_panel_rect = egui::Rect::ZERO;

    loop {
        ship_manager.sync_from_db(&game_state.ctx.db);
        clear_background(BLACK);

        // Focus camera on current target (usually the player's ship)
        game_state.camera.target = Vec2::ZERO; //get_player_transform_vec2(&ctx, Vec2::ZERO);

        // Offset camera to account for side panel
        game_state.camera.target.x -= side_panel_rect.right() / 2.0;
        set_camera(&game_state.camera);

        // Render ships with synchronized server time
        ship_manager.render();

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

impl Default for InputState {
    fn default() -> Self {
        Self {
            is_thrusting: false,
            is_breaking: false,
            turn_direction: 0,
        }
    }
}

/// Handles player input and calls the proper reducers if the controls have changed.
fn handle_input(ctx: &DbConnection) {
    use std::cell::RefCell;

    // Track previous input state using a thread-local variable
    thread_local! {
        static PREVIOUS_INPUT: RefCell<InputState> = RefCell::new(InputState::default());
    }

    let _player_ship = {
        match ctx.db.space_ship().entity_id().find(&ctx.identity()) {
            Some(ship) => ship,
            None => return,
        }
    };

    // Determine current input state from keyboard
    let is_thrusting = is_key_down(KeyCode::W) || is_key_down(KeyCode::Up);
    let is_breaking = is_key_down(KeyCode::S) || is_key_down(KeyCode::Down);
    let turn_direction = if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
        1
    } else if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
        -1
    } else {
        0
    };

    let current_input = InputState {
        is_thrusting,
        is_breaking,
        turn_direction,
    };

    // Compare with previous state and call reducers only if changed
    PREVIOUS_INPUT.with(|prev| {
        let mut prev_input = prev.borrow_mut();

        if current_input.is_thrusting != prev_input.is_thrusting
            || current_input.is_breaking != prev_input.is_breaking
        {
            let _ = ctx.reducers().set_thrust_input(is_thrusting, is_breaking);
        }

        if current_input.turn_direction != prev_input.turn_direction {
            let _ = ctx.reducers().set_turn_input(turn_direction);
        }

        // Update previous state
        *prev_input = current_input;
    });
}

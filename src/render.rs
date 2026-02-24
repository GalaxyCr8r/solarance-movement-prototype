use macroquad::prelude::{collections::storage, *};

use crate::resources::Resources;

pub fn draw_ship(x: f32, y: f32, rotation_radians: f32) {
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

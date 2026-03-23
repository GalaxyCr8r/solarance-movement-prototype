use macroquad::prelude::{collections::storage, *};

use crate::resources::Resources;

pub fn draw_ship(x: f32, y: f32, rotation_radians: f32) {
    let resources = storage::get::<Resources>();
    let ship_texture = &resources.ship_textures.get("lc.phalanx").unwrap();
    draw_texture_ex(
        ship_texture,
        x - ship_texture.width() / 2.0,
        y - ship_texture.height() / 2.0,
        WHITE,
        DrawTextureParams {
            rotation: rotation_radians,
            ..Default::default()
        },
    );
}

pub(crate) fn draw_bullet(pos: solarance_shared::physics::Vec2, rotation: f32) {
    let resources = storage::get::<Resources>();
    let bullet_texture = &resources.bullet_textures.get("bullet01").unwrap();
    draw_texture_ex(
        bullet_texture,
        pos.x - bullet_texture.width() / 2.0,
        pos.y - bullet_texture.height() / 2.0,
        WHITE,
        DrawTextureParams {
            rotation: rotation,
            ..Default::default()
        },
    );
}

use macroquad::prelude::*;
use std::collections::HashMap;

pub struct Resources {
    pub ship_textures: HashMap<&'static str, Texture2D>,
}

impl Resources {
    pub async fn new() -> Result<Resources, macroquad::Error> {
        let mut resources = Resources {
            ship_textures: HashMap::new(),
        };

        // Load asset textures
        info!("Loading textures...");
        resources
            .ship_textures
            .insert("lc.phalanx", load_linear_sprite("phalanx.png").await?);
        Ok(resources)
    }
}

async fn load_linear_sprite(path: &str) -> Result<Texture2D, macroquad::Error> {
    let texture = load_texture(path).await?;
    texture.set_filter(FilterMode::Nearest);
    Ok(texture)
}

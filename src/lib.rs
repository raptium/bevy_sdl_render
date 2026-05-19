use bevy::prelude::*;

mod render;
mod texture;

pub struct SdlRenderPlugin;

impl Plugin for SdlRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(texture::TexturePlugin)
            .set_runner(render::render);
    }
}

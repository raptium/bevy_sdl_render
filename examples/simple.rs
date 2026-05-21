use bevy::prelude::*;
use bevy_sdl_render::SdlRenderPlugin;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, SdlRenderPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn(Sprite::from_image(
        asset_server.load_builder().load("gabe-idle-run.png"),
    ));
}

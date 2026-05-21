use bevy::{color::palettes::css, prelude::*};
use bevy_sdl_render::SdlRenderPlugin;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, SdlRenderPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: Color::from(css::BLUE).into(),
            ..default()
        },
    ));
    commands.spawn(Sprite::from_image(
        asset_server.load_builder().load("gabe-idle-run.png"),
    ));
}

use bevy::{color::palettes::css, prelude::*};
use bevy_sdl_render::SdlRenderPlugin;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, SdlRenderPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, player_move)
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

fn player_move(
    mut sprite: Single<&mut Transform, With<Sprite>>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
) {
    let mut dir = Vec2::ZERO;
    for gamepad in gamepads {
        dir += gamepad.left_stick();
    }
    sprite.translation += Vec3::from((dir * 50. * time.delta_secs(), 0.));
}

// From https://github.com/MrSheerluck/bevy-pong
use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowResized, WindowResolution},
};
use bevy_sdl_render::{FromColor, SdlRenderPlugin};

#[derive(Component, Clone, Default)]
struct Paddle {
    speed: f32,
    side: Side,
}

#[derive(Component, Clone, Copy, Default)]
enum Side {
    #[default]
    Left,
    Right,
}

#[derive(Component)]
struct Ball {
    velocity: Vec3,
}

#[derive(Resource, Default)]
struct Score {
    left: u32,
    right: u32,
}

const WINDOW_SIZE: (u32, u32) = (640, 480);

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(WINDOW_SIZE.0, WINDOW_SIZE.1),
                    title: "Pong".into(),
                    ..default()
                }),
                ..default()
            }),
            SdlRenderPlugin,
        ))
        .insert_resource(ClearColor(Color::srgb(0.0, 0.0, 0.0)))
        .insert_resource(Score::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                move_paddle,
                move_ball,
                bounce_ball,
                check_paddle_collision,
                score_goal,
                handle_window_resize,
            ),
        )
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.spawn(Camera2d);
    let paddle_image = images.add(Image::from_color(Color::srgb(1., 0., 0.)));
    commands.spawn_scene(paddle(Side::Left, paddle_image.clone()));
    commands.spawn_scene(paddle(Side::Right, paddle_image));

    // Ball
    commands.spawn((
        Ball {
            velocity: Vec3::new(300.0, 150.0, 0.0),
        },
        Sprite::from_color(Color::WHITE, Vec2::new(10.0, 10.0)),
        Transform::from_xyz(0.0, 0.0, 0.0),
        GlobalTransform::default(),
    ));
}

fn paddle(side: Side, image: Handle<Image>) -> impl Scene {
    let x = match side {
        Side::Left => -(WINDOW_SIZE.0 as f32 / 2.0),
        Side::Right => WINDOW_SIZE.0 as f32 / 2.0,
    };
    bsn! {
        Paddle {
            speed: 500.0,
            side: side,
        }
        Sprite {
            image: image,
            custom_size: Vec2::new(10.0, 100.0),
        }
        Transform::from_xyz(x, 0.0, 0.0)
    }
}

fn move_paddle(
    gamepads: Query<&Gamepad>,
    mut paddle_query: Query<(&mut Transform, &Paddle)>,
    time: Res<Time>,
) {
    for (mut transform, paddle) in &mut paddle_query {
        let mut direction = 0.0;
        match paddle.side {
            Side::Left => {
                for gamepad in gamepads {
                    direction += gamepad.left_stick().y;
                }
            }
            Side::Right => {
                for gamepad in gamepads {
                    direction += gamepad.right_stick().y;
                }
            }
        }
        transform.translation.y += direction * paddle.speed * time.delta_secs();
    }
}

fn move_ball(mut ball_query: Query<(&mut Transform, &Ball)>, time: Res<Time>) {
    for (mut transform, ball) in &mut ball_query {
        transform.translation += ball.velocity * time.delta_secs();
    }
}

fn bounce_ball(
    mut ball_query: Query<(&mut Transform, &mut Ball)>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let half_height = window.height() / 2.0;
    let ball_radius = 5.0;
    for (mut transform, mut ball) in &mut ball_query {
        if transform.translation.y + ball_radius >= half_height {
            transform.translation.y = half_height - ball_radius;
            ball.velocity.y = -ball.velocity.y;
        }
        if transform.translation.y - ball_radius <= -half_height {
            transform.translation.y = -half_height + ball_radius;
            ball.velocity.y = -ball.velocity.y;
        }
    }
}

fn check_paddle_collision(
    mut ball_query: Query<(&mut Transform, &mut Ball), Without<Paddle>>,
    paddle_query: Query<(&Transform, &Paddle), Without<Ball>>,
) {
    for (mut ball_transform, mut ball) in &mut ball_query {
        let ball_pos = ball_transform.translation.truncate();
        let ball_size = Vec2::new(10.0, 10.0);
        for (paddle_transform, _paddle) in &paddle_query {
            let paddle_pos = paddle_transform.translation.truncate();
            let paddle_size = Vec2::new(10.0, 100.0);
            // AABB collision check
            let overlap = !(ball_pos.x + ball_size.x / 2.0 < paddle_pos.x - paddle_size.x / 2.0
                || ball_pos.x - ball_size.x / 2.0 > paddle_pos.x + paddle_size.x / 2.0
                || ball_pos.y + ball_size.y / 2.0 < paddle_pos.y - paddle_size.y / 2.0
                || ball_pos.y - ball_size.y / 2.0 > paddle_pos.y + paddle_size.y / 2.0);
            if overlap {
                ball.velocity.x = -ball.velocity.x;
                // Snap ball to paddle edge to prevent sticking
                if ball.velocity.x > 0.0 {
                    ball_transform.translation.x =
                        paddle_pos.x + paddle_size.x / 2.0 + ball_size.x / 2.0;
                } else {
                    ball_transform.translation.x =
                        paddle_pos.x - paddle_size.x / 2.0 - ball_size.x / 2.0;
                }
            }
        }
    }
}

fn score_goal(
    mut ball_query: Query<&mut Transform, With<Ball>>,
    mut score: ResMut<Score>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let width = window.width();
    let half_width = width / 2.0;
    for mut transform in &mut ball_query {
        if transform.translation.x > half_width + 10.0 {
            score.left += 1;
            transform.translation = Vec3::new(0.0, 0.0, 0.0);
        } else if transform.translation.x < -half_width - 10.0 {
            score.right += 1;
            transform.translation = Vec3::new(0.0, 0.0, 0.0);
        } else {
            continue;
        }
    }
}

fn handle_window_resize(
    mut resize_reader: MessageReader<WindowResized>,
    window: Single<Entity, With<PrimaryWindow>>,
    mut paddle_query: Query<(&mut Transform, &Paddle)>,
) {
    for e in resize_reader.read() {
        if e.window == *window {
            for (mut paddle_transform, paddle) in paddle_query.iter_mut() {
                match paddle.side {
                    Side::Left => paddle_transform.translation.x = -(e.width / 2.0),
                    Side::Right => paddle_transform.translation.x = e.width / 2.0,
                }
            }
        }
    }
}

use std::{
    num::NonZero,
    time::{Duration, Instant},
};

use bevy::{
    app::{MainScheduleOrder, PluginsState},
    camera::{RenderTarget, RenderTargetInfo},
    ecs::{schedule::ScheduleLabel, system::NonSendMarker},
    image::{CompressedImageFormats, ImageLoader},
    platform::thread,
    prelude::*,
    time::TimeSender,
    window::{PrimaryWindow, WindowEvent, WindowRef, WindowResized},
};
use sdl2::event as sdlevent;

use crate::{
    input::SdlInput, registry::SdlRegistry, snapshot::SdlSnapshot, texture::SdlTexturePlugin,
};

pub struct SdlRenderPlugin;

impl Plugin for SdlRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_schedule(Schedule::new(Render));
        let mut main_schedule_order = app.world_mut().resource_mut::<MainScheduleOrder>();
        main_schedule_order.insert_after(Last, Render);

        let (sender, receiver) = bevy::time::create_time_channels();
        app.add_plugins(SdlTexturePlugin)
            .set_runner(runner)
            .insert_resource(sender)
            .insert_resource(receiver)
            .init_resource::<SdlInput>()
            .init_resource::<SdlSnapshot>()
            // bevy_render usually registers this
            .register_asset_loader(ImageLoader::new(CompressedImageFormats::NONE))
            .configure_sets(
                Render,
                (RenderSystems::Prepare, RenderSystems::Render).chain(),
            )
            .add_systems(First, sdl_events)
            .add_systems(
                Render,
                (
                    render.in_set(RenderSystems::Render),
                    send_time.after(RenderSystems::Render),
                ),
            );
    }
}

#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct Render;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum RenderSystems {
    Prepare,
    Render,
}

fn sdl_events(
    mut app_exit_writer: MessageWriter<AppExit>,
    mut window_resized: MessageWriter<WindowResized>,
    mut window_event: MessageWriter<WindowEvent>,
    mut window: Single<(Entity, &mut Window), With<PrimaryWindow>>,
    mut camera: Single<(&mut Camera, &RenderTarget, &mut Projection), With<Camera2d>>,
    mut input: ResMut<SdlInput>,
    _non_send: NonSendMarker,
) -> Result<()> {
    let mut sized = false;
    input.begin_frame();
    SdlRegistry::with_borrow_mut(|registry| {
        for event in registry.events() {
            match event {
                // Bevy's own `ButtonInput` is never populated on this backend
                // (bevy_winit never runs), so SDL state is republished as-is.
                sdlevent::Event::KeyDown {
                    keycode: Some(key),
                    repeat,
                    ..
                } => input.press_key(key, repeat),
                sdlevent::Event::KeyUp {
                    keycode: Some(key), ..
                } => input.release_key(key),
                // Raw joystick events, not controller events: the controller
                // layer needs a gamecontrollerdb mapping for this pad and
                // there is none, so it stays silent.
                sdlevent::Event::JoyButtonDown { button_idx, .. } => {
                    input.press_button(button_idx);
                }
                sdlevent::Event::JoyButtonUp { button_idx, .. } => {
                    input.release_button(button_idx);
                }
                sdlevent::Event::JoyAxisMotion {
                    axis_idx, value, ..
                } => input.set_axis(axis_idx, value),
                sdlevent::Event::JoyHatMotion {
                    hat_idx, state, ..
                } => input.set_hat(hat_idx, state),
                sdlevent::Event::JoyDeviceAdded { which, .. } => {
                    input.devices += 1;
                    input.device_change = true;
                    println!("SDL: joystick added (index {which})");
                }
                sdlevent::Event::JoyDeviceRemoved { which, .. } => {
                    input.devices = input.devices.saturating_sub(1);
                    input.device_change = true;
                    println!("SDL: joystick removed (index {which})");
                }
                sdlevent::Event::Quit { .. } => {
                    app_exit_writer.write(AppExit::Success);
                }
                sdlevent::Event::Window {
                    win_event: sdlevent::WindowEvent::Shown,
                    ..
                }
                | sdlevent::Event::Window {
                    win_event: sdlevent::WindowEvent::SizeChanged(..),
                    ..
                } => sized = true,
                _ => (),
            }
        }
    });

    if sized {
        let (width, height) = SdlRegistry::with_borrow(|registry| registry.window().size());
        // We just support a single window
        let (window_entity, ref mut window) = *window;
        window.resolution.set_physical_resolution(width, height);

        let event = WindowResized {
            window: window_entity,
            width: width as f32,
            height: height as f32,
        };
        window_resized.write(event.clone());
        window_event.write(WindowEvent::WindowResized(event));

        let (ref mut camera, render_target, ref mut camera_projection) = *camera;
        if !matches!(render_target, RenderTarget::Window(WindowRef::Primary)) {
            return Err("Unsupported RenderTarget".into());
        }

        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: window.physical_size(),
            scale_factor: window.resolution.scale_factor(),
        });

        if let Some(logical_viewport_size) = camera.logical_viewport_size()
            && logical_viewport_size.x != 0.0
            && logical_viewport_size.y != 0.0
        {
            camera_projection.update(logical_viewport_size.x, logical_viewport_size.y);
            camera.computed.clip_from_view = match &camera.sub_camera_view {
                Some(sub_view) => camera_projection.get_clip_from_view_for_sub(sub_view),
                None => camera_projection.get_clip_from_view(),
            };
        }
    }

    Ok(())
}

fn send_time(time_sender: Res<TimeSender>) -> Result<()> {
    time_sender.0.send(Instant::now())?;
    Ok(())
}

fn render(
    camera: Single<(&Camera, &GlobalTransform), With<Camera2d>>,
    sprites: Query<(&Sprite, &GlobalTransform)>,
    texture_atlases: Res<Assets<TextureAtlasLayout>>,
    clear_color: Res<ClearColor>,
    mut snapshot: ResMut<SdlSnapshot>,
    _non_send: NonSendMarker,
) -> Result<()> {
    let (camera, camera_transform) = *camera;
    let request = snapshot.take();
    let captured = request.is_some();
    SdlRegistry::with_borrow_mut(|registry| -> Result<()> {
        let clear = match camera.clear_color {
            ClearColorConfig::Default => Some(clear_color.0),
            ClearColorConfig::Custom(color) => Some(color),
            ClearColorConfig::None => None,
        };
        registry.clear(clear);

        // Sorted back-to-front on Z. A game cannot influence query iteration
        // order, so without this every depth-sorted scene (a character walking
        // behind a table) draws in whatever order the archetypes happen to be
        // in. SDL paints later copies over earlier ones, so ascending Z is
        // "further away first".
        let mut ordered: Vec<(&Sprite, &GlobalTransform)> = sprites.iter().collect();
        ordered.sort_by(|a, b| {
            a.1.translation()
                .z
                .partial_cmp(&b.1.translation().z)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for (sprite, sprite_transform) in ordered {
            registry.render_sprite(
                sprite,
                camera,
                camera_transform,
                sprite_transform,
                texture_atlases.as_ref(),
            )?;
        }
        // Before present(): after the swap the back buffer is undefined.
        if let Some(path) = &request {
            let (width, height) = registry.capture(path)?;
            println!("SNAPSHOT: {width}x{height} -> {}", path.display());
        }
        registry.present();
        Ok(())
    })?;
    if captured {
        snapshot.done();
    }
    Ok(())
}
pub fn runner(mut app: App) -> AppExit {
    if app.plugins_state() != PluginsState::Cleaned {
        while app.plugins_state() == PluginsState::Adding {
            bevy::tasks::tick_global_task_pools_on_main_thread();
        }
        app.finish();
        app.cleanup();
    }

    match event_loop(app) {
        Ok(exit) => exit,
        Err(e) => {
            error!("Error: {e:?}");
            AppExit::Error(NonZero::new(1u8).unwrap())
        }
    }
}

fn event_loop(mut app: App) -> Result<AppExit> {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&Window, With<PrimaryWindow>>();
    let primary_window = query.single(world)?;
    SdlRegistry::init(primary_window)?;

    let refresh_rate = SdlRegistry::with_borrow(|registry| registry.display_mode())?.refresh_rate;
    let wait = Duration::from_secs_f32(if refresh_rate > 0 {
        1.0 / refresh_rate as f32
    } else {
        0.0
    });

    loop {
        let start_time = Instant::now();
        app.update();
        if let Some(exit) = app.should_exit() {
            return Ok(exit);
        }

        let elapsed = start_time.elapsed();
        if elapsed < wait {
            thread::sleep(wait - elapsed);
        }
    }
}

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

use crate::{registry::SdlRegistry, texture::SdlTexturePlugin};

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
    mut camera: Single<(&mut Camera, &RenderTarget), With<Camera2d>>,
    _non_send: NonSendMarker,
) -> Result<()> {
    let mut resize = false;
    SdlRegistry::with_borrow_mut(|registry| {
        for event in registry.events() {
            match event {
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
                } => resize = true,
                _ => (),
            }
        }
    });

    if resize {
        let (width, height) = SdlRegistry::with_borrow(|registry| registry.window().size());
        // We just support a single window
        let (window_entity, ref mut window) = *window;
        window.resolution.set_physical_resolution(width, height);

        let (ref mut camera, render_target) = *camera;
        if matches!(render_target, RenderTarget::Window(WindowRef::Primary)) {
            let computed_target_info = RenderTargetInfo {
                physical_size: window.physical_size(),
                scale_factor: window.resolution.scale_factor(),
            };
            camera.computed.target_info = Some(computed_target_info);
        } else {
            return Err("Unsupported RenderTarget".into());
        }

        let event = WindowResized {
            window: window_entity,
            width: width as f32,
            height: height as f32,
        };
        window_resized.write(event.clone());
        window_event.write(WindowEvent::WindowResized(event));
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
    _non_send: NonSendMarker,
) -> Result<()> {
    let (camera, camera_transform) = *camera;
    SdlRegistry::with_borrow_mut(|registry| -> Result<()> {
        //XXX set clear color?
        registry.clear();
        for (sprite, sprite_transform) in sprites {
            registry.render_sprite(
                sprite,
                camera,
                camera_transform,
                sprite_transform,
                texture_atlases.as_ref(),
            )?;
        }
        registry.present();
        Ok(())
    })?;
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

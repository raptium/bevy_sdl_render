use std::{
    num::NonZero,
    time::{Duration, Instant},
};

use bevy::{
    app::MainScheduleOrder,
    ecs::{
        schedule::{ScheduleLabel, SingleThreadedExecutor},
        system::NonSendMarker,
    },
    platform::thread,
    prelude::*,
    time::TimeSender,
    window::{PrimaryWindow, WindowEvent, WindowResized},
};
use sdl2::event as sdlevent;

use crate::{registry::SdlRegistry, texture::SdlTexturePlugin};

pub struct SdlRenderPlugin;

impl Plugin for SdlRenderPlugin {
    fn build(&self, app: &mut App) {
        let mut render_schedule = Schedule::new(Render);
        render_schedule.set_executor(SingleThreadedExecutor::new());
        app.add_schedule(render_schedule);
        let mut main_schedule_order = app.world_mut().resource_mut::<MainScheduleOrder>();
        main_schedule_order.insert_after(Last, Render);

        app.add_plugins(SdlTexturePlugin)
            .set_runner(runner)
            .configure_sets(
                Render,
                (RenderSystems::Extract, RenderSystems::Render).chain(),
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
    Extract,
    Render,
}

fn sdl_events(
    mut app_exit_writer: MessageWriter<AppExit>,
    mut window_resized: MessageWriter<WindowResized>,
    mut window_event: MessageWriter<WindowEvent>,
    mut window: Single<(Entity, &mut Window), With<PrimaryWindow>>,
) {
    SdlRegistry::with_borrow_mut(|registry| {
        for event in registry.events() {
            match event {
                sdlevent::Event::Quit { .. } => {
                    app_exit_writer.write(AppExit::Success);
                }
                sdlevent::Event::Window {
                    win_event: sdlevent::WindowEvent::SizeChanged(width, height),
                    ..
                } => {
                    // We just support a single window
                    let (entity, ref mut window) = *window;
                    window
                        .resolution
                        .set_physical_resolution(width as u32, height as u32);
                    let event = WindowResized {
                        window: entity,
                        width: width as f32,
                        height: height as f32,
                    };
                    window_resized.write(event.clone());
                    window_event.write(WindowEvent::WindowResized(event));
                }
                _ => (),
            }
        }
    });
}

fn send_time(time_sender: Res<TimeSender>) -> Result<()> {
    time_sender.0.send(Instant::now())?;
    Ok(())
}

fn render() {
    //XXX set clear color?
    SdlRegistry::with_borrow_mut(|registry| registry.clear());

    //XXX copy/draw textures based on sprite positions

    SdlRegistry::with_borrow_mut(|registry| registry.present());
}

pub fn runner(mut app: App) -> AppExit {
    app.finish();
    app.cleanup();

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
        let end_time = Instant::now();

        let exe_time = end_time - start_time;
        if exe_time < wait {
            thread::sleep(wait - exe_time);
        }
    }
}

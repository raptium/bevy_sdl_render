use std::{num::NonZero, time::Instant};

use bevy::{
    app::MainScheduleOrder,
    ecs::{
        schedule::{ScheduleLabel, SingleThreadedExecutor},
        system::NonSendMarker,
    },
    prelude::*,
    time::TimeSender,
    window::PrimaryWindow,
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
            .add_systems(Startup, setup)
            .add_systems(Render, send_time.after(RenderSystems::Render));
    }
}

#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct Render;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum RenderSystems {
    Extract,
    Render,
}

fn setup(
    primary_window: Single<&Window, With<PrimaryWindow>>,
    _non_send: NonSendMarker,
) -> Result<()> {
    SdlRegistry::init(*primary_window)?;
    Ok(())
}

fn send_time(time_sender: Res<TimeSender>) -> Result<()> {
    time_sender.0.send(Instant::now())?;
    Ok(())
}

pub fn runner(mut app: App) -> AppExit {
    app.finish();
    app.cleanup();

    match event_loop(app) {
        Ok(_) => AppExit::Success,
        Err(e) => {
            error!("Error: {e:?}");
            AppExit::Error(NonZero::new(1u8).unwrap())
        }
    }
}

fn event_loop(mut app: App) -> Result<()> {
    app.update();

    let mut event_pump = SdlRegistry::with_borrow(|registry| registry.event_pump())?;
    'running: loop {
        for event in event_pump.poll_iter() {
            //XXX send events into bevy
            match event {
                sdlevent::Event::Quit { .. } => break 'running,
                _ => (),
            }
        }

        app.update();

        SdlRegistry::with_borrow_mut(|registry| {
            //XXX set clear color
            registry.clear();
            //XXX copy/draw textures based on sprite positions
            registry.present();
        });
    }
    Ok(())
}

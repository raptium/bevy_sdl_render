use std::{num::NonZero, time::Instant};

use bevy::{
    ecs::system::SystemState, platform::collections::HashMap, prelude::*, time::TimeSender,
    window::PrimaryWindow,
};
use sdl2::event as sdlevent;

use crate::texture::ImageChanges;

pub fn render(mut app: App) -> AppExit {
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
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let time_sender = app
        .world()
        .get_resource::<TimeSender>()
        .ok_or("TimeSender not found")?
        .0
        .clone();

    let world = app.world_mut();
    let mut query = world.query_filtered::<&Window, With<PrimaryWindow>>();
    let primary_window = query.single(world)?;

    let window = video_subsystem
        .window(
            &primary_window.title,
            primary_window.resolution.physical_width(),
            primary_window.resolution.physical_height(),
        )
        .position_centered()
        .opengl()
        .build()?;

    //XXX .accelerated()?
    let mut canvas = window.into_canvas().present_vsync().build()?;
    let texture_creator = canvas.texture_creator();
    let mut textures = HashMap::new();

    let mut system_state: SystemState<(ResMut<ImageChanges>, Res<Assets<Image>>)> =
        SystemState::new(app.world_mut());

    let mut event_pump = sdl_context.event_pump()?;
    'running: loop {
        for event in event_pump.poll_iter() {
            //XXX send events into bevy
            match event {
                sdlevent::Event::Quit { .. } => break 'running,
                _ => (),
            }
        }

        time_sender.send(Instant::now())?;
        app.update();

        let (mut changes, images) = system_state.get_mut(app.world_mut())?;
        changes.update_textures(&mut textures, images, &texture_creator)?;

        //XXX set clear color
        canvas.clear();
        //XXX copy/draw textures based on sprite positions
        canvas.present();
    }
    Ok(())
}

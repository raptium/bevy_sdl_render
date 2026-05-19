use std::cell::RefCell;

use bevy::{platform::collections::HashMap, prelude::*};
use sdl2::{
    render::{Canvas, Texture as SdlTexture, TextureCreator},
    video::{Window as SdlWindow, WindowContext},
};

thread_local! {
    pub static SDL_REGISTRY: RefCell<Option<SdlRegistry<'static>>> = const { RefCell::new(None) };
}

pub struct SdlRegistry<'a> {
    canvas: Canvas<SdlWindow>,
    texture_creator: TextureCreator<WindowContext>,
    textures: HashMap<AssetId<Image>, SdlTexture<'a>>,
}

impl SdlRegistry<'static> {
    pub fn init(primary_window: &Window) -> Result<()> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;

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
        let canvas = window.into_canvas().present_vsync().build()?;
        let texture_creator = canvas.texture_creator();
        let textures = HashMap::new();
        SDL_REGISTRY.with(move |r| {
            r.replace(Some(Self {
                canvas,
                texture_creator,
                textures,
            }));
        });
        Ok(())
    }

    #[inline]
    pub fn with_borrow(f: impl FnOnce(&Self)) {
        SDL_REGISTRY.with_borrow(|r| f(r.as_ref().expect("SDL_REGISTRY was not initialized")));
    }

    #[inline]
    pub fn with_borrow_mut(f: impl FnOnce(&mut Self)) {
        SDL_REGISTRY.with_borrow_mut(|r| f(r.as_mut().expect("SDL_REGISTRY was not initialized")));
    }
}

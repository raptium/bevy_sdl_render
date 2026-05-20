use std::cell::RefCell;

use bevy::{platform::collections::HashMap, prelude::*};
use sdl2::{
    EventPump,
    pixels::PixelFormatEnum,
    render::{Canvas, Texture as SdlTexture, TextureCreator},
    video::{Window as SdlWindow, WindowContext},
};

thread_local! {
    static SDL_REGISTRY: RefCell<SdlRegistry<'static>> = panic!("SDL_REGISTRY was not initialized");
}

pub struct SdlRegistry<'a> {
    context: sdl2::Sdl,
    canvas: Canvas<SdlWindow>,
    texture_creator: &'static TextureCreator<WindowContext>,
    textures: HashMap<AssetId<Image>, SdlTexture<'a>>,
}

impl SdlRegistry<'static> {
    pub fn init(primary_window: &Window) -> Result<()> {
        let context = sdl2::init()?;
        let video_subsystem = context.video()?;

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
        // Leak the TextureCreator so we can store a static lifetime.
        // We store the textures in a static HashMap, so TextureCreator needs to be static so they don't outlive it
        let texture_creator: &'static TextureCreator<WindowContext> =
            Box::leak(Box::new(canvas.texture_creator()));
        SDL_REGISTRY.set(Self {
            context,
            canvas,
            texture_creator,
            textures: HashMap::new(),
        });
        Ok(())
    }

    #[inline]
    pub fn with_borrow<T>(f: impl FnOnce(&Self) -> T) -> T {
        SDL_REGISTRY.with_borrow(f)
    }

    #[inline]
    pub fn with_borrow_mut<T>(f: impl FnOnce(&mut Self) -> T) -> T {
        SDL_REGISTRY.with_borrow_mut(f)
    }

    pub fn event_pump(&self) -> Result<EventPump> {
        Ok(self.context.event_pump()?)
    }

    pub fn clear(&mut self) {
        self.canvas.clear();
    }

    pub fn present(&mut self) {
        self.canvas.present();
    }

    pub fn remove_texture(&mut self, id: &AssetId<Image>) {
        self.textures.remove(id);
    }

    pub fn create_texture(
        &mut self,
        id: AssetId<Image>,
        pixel_format: PixelFormatEnum,
        width: u32,
        height: u32,
    ) -> Result<()> {
        self.textures.insert(
            id,
            self.texture_creator
                .create_texture_static(pixel_format, width, height)?,
        );
        Ok(())
    }

    pub fn modify_texture(&mut self, id: &AssetId<Image>, data: &[u8], pitch: usize) -> Result<()> {
        let Some(texture) = self.textures.get_mut(id) else {
            return Err("Texture not found".into());
        };
        texture.update(None, data, pitch)?;
        Ok(())
    }
}

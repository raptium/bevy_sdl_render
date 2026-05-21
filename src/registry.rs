use std::cell::RefCell;

use bevy::{image::TextureFormatPixelInfo, platform::collections::HashMap, prelude::*};
use sdl2::{
    EventPump, VideoSubsystem,
    event::EventPollIterator,
    pixels::{Color as SdlColor, PixelFormatEnum},
    rect::{FPoint, FRect, Rect as SdlRect},
    render::{BlendMode, Canvas, Texture as SdlTexture, TextureCreator},
    video::{DisplayMode, Window as SdlWindow, WindowContext},
};

thread_local! {
    static SDL_REGISTRY: RefCell<SdlRegistry<'static>> = panic!("SDL_REGISTRY was not initialized");
}

pub struct SdlRegistry<'a> {
    video_subsystem: VideoSubsystem,
    canvas: Canvas<SdlWindow>,
    texture_creator: &'static TextureCreator<WindowContext>,
    textures: HashMap<AssetId<Image>, SdlTexture<'a>>,
    event_pump: EventPump,
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
            .build()?;

        let mut canvas = window.into_canvas().accelerated().present_vsync().build()?;
        canvas.set_blend_mode(BlendMode::Blend);

        // Leak the TextureCreator so we can store a static lifetime.
        // We store the textures in a static HashMap, so TextureCreator needs to be static so they don't outlive it
        let texture_creator: &'static TextureCreator<WindowContext> =
            Box::leak(Box::new(canvas.texture_creator()));
        SDL_REGISTRY.set(Self {
            event_pump: context.event_pump()?,
            video_subsystem,
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

    pub fn window(&self) -> &SdlWindow {
        self.canvas.window()
    }

    pub fn display_mode(&self) -> Result<DisplayMode> {
        let display_index = self.canvas.window().display_index()?;
        Ok(self.video_subsystem.current_display_mode(display_index)?)
    }

    pub fn events(&mut self) -> EventPollIterator<'_> {
        self.event_pump.poll_iter()
    }

    pub fn clear(&mut self, color: Option<Color>) {
        if let Some(color) = color {
            let rgba = LinearRgba::from(color);
            self.canvas.set_draw_color(SdlColor {
                r: (rgba.red * 255.0) as u8,
                g: (rgba.green * 255.0) as u8,
                b: (rgba.blue * 255.0) as u8,
                a: (rgba.alpha * 255.0) as u8,
            });
            self.canvas.clear();
        }
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
        let mut texture =
            self.texture_creator
                .create_texture_static(pixel_format, width, height)?;
        texture.set_blend_mode(BlendMode::Blend);
        self.textures.insert(id, texture);
        Ok(())
    }

    pub fn modify_texture(&mut self, id: &AssetId<Image>, data: &[u8], pitch: usize) -> Result<()> {
        let Some(texture) = self.textures.get_mut(id) else {
            return Err("Texture not found".into());
        };
        texture.update(None, data, pitch)?;
        Ok(())
    }

    pub fn modify_texture_from_image(&mut self, id: &AssetId<Image>, image: &Image) -> Result<()> {
        if let Some(ref data) = image.data
            && let Ok(pixel_size) = image.texture_descriptor.format.pixel_size()
        {
            let pitch = pixel_size * image.width() as usize;
            self.modify_texture(id, data.as_slice(), pitch)?;
        }
        Ok(())
    }

    pub fn render_sprite(
        &mut self,
        sprite: &Sprite,
        camera: &Camera,
        camera_transform: &GlobalTransform,
        sprite_transform: &GlobalTransform,
        texture_atlases: &Assets<TextureAtlasLayout>,
    ) -> Result<()> {
        // XXX not handling sprite.image of Handle::default() (i.e. Sprite::from_color)
        // XXX also not handling color tinted image
        let Some(texture) = self.textures.get(&sprite.image.id()) else {
            return Ok(());
        };

        let (src, sprite_size) = if let Some(ref texture_atlas) = sprite.texture_atlas
            && let Some(atlas_rect) = texture_atlas.texture_rect(texture_atlases)
        {
            let mut atlas_rect = atlas_rect.as_rect();
            if let Some(sprite_rect) = sprite.rect {
                atlas_rect.min += sprite_rect.min;
                atlas_rect.max += sprite_rect.min;
            }
            (
                SdlRect::new(
                    atlas_rect.min.x as i32,
                    atlas_rect.min.y as i32,
                    atlas_rect.width() as u32,
                    atlas_rect.height() as u32,
                ),
                atlas_rect.size(),
            )
        } else if let Some(rect) = sprite.rect {
            let size = rect.size();
            (
                SdlRect::new(
                    rect.min.x as i32,
                    rect.min.y as i32,
                    size.x.max(0.0) as u32,
                    size.y.max(0.0) as u32,
                ),
                size,
            )
        } else {
            let texinfo = texture.query();
            (
                SdlRect::new(0, 0, texinfo.width, texinfo.height),
                Vec2::new(texinfo.width as f32, texinfo.height as f32),
            )
        };

        let sprite_size = sprite.custom_size.unwrap_or(sprite_size);
        let (scale, rotation, translation) = sprite_transform.to_scale_rotation_translation();
        let viewport_scale = camera.target_scaling_factor().unwrap_or(1.0);
        let to_sdl_point = |world_position| -> Option<Vec2> {
            camera
                .world_to_viewport(camera_transform, world_position)
                .ok()
                .map(|position| position * viewport_scale)
        };

        let Some(center_position) = to_sdl_point(translation) else {
            return Ok(());
        };
        let right_position =
            to_sdl_point(translation + rotation * Vec3::X * sprite_size.x * scale.x.abs())
                .unwrap_or(center_position);
        let up_position =
            to_sdl_point(translation + rotation * Vec3::Y * sprite_size.y * scale.y.abs())
                .unwrap_or(center_position);

        let right = right_position - center_position;
        let up = up_position - center_position;
        let width = right.length();
        let height = up.length();
        if width <= 0.0 || height <= 0.0 {
            return Ok(());
        }

        let dst = FRect::new(
            center_position.x - width / 2.0,
            center_position.y - height / 2.0,
            width,
            height,
        );
        let angle = right.y.atan2(right.x).to_degrees() as f64;
        let center = FPoint::new(width / 2.0, height / 2.0);

        self.canvas.copy_ex_f(
            texture,
            src,
            dst,
            angle,
            center,
            sprite.flip_x ^ scale.x.is_sign_negative(),
            sprite.flip_y ^ scale.y.is_sign_negative(),
        )?;
        Ok(())
    }
}

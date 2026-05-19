use bevy::{image::TextureFormatPixelInfo, platform::collections::HashMap, prelude::*};
use sdl2::{pixels as sdlpixels, render as sdlrender, video as sdlvideo};

pub struct TexturePlugin;

impl Plugin for TexturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ImageChanges>()
            .add_systems(Last, monitor_images);
    }
}

#[derive(Resource, Default)]
pub struct ImageChanges {
    add: Vec<AssetId<Image>>,
    remove: Vec<AssetId<Image>>,
    modify: Vec<AssetId<Image>>,
}

impl ImageChanges {
    fn clear(&mut self) {
        self.add.clear();
        self.remove.clear();
        self.modify.clear();
    }

    pub fn update_textures<'tc>(
        &mut self,
        textures: &mut HashMap<AssetId<Image>, sdlrender::Texture<'tc>>,
        images: Res<Assets<Image>>,
        texture_creator: &'tc sdlrender::TextureCreator<sdlvideo::WindowContext>,
    ) -> Result<()> {
        for id in self.remove.iter() {
            textures.remove(id);
        }
        for id in self.add.iter() {
            if let Some(image) = images.get(*id) {
                let pixel_format = match image.texture_descriptor.format {
                    wgpu_types::TextureFormat::Rgba8UnormSrgb => sdlpixels::PixelFormatEnum::RGBA32,
                    wgpu_types::TextureFormat::Bgra8UnormSrgb => sdlpixels::PixelFormatEnum::BGRA32,
                    _ => return Err("Unsupported image format".into()),
                };
                textures.insert(
                    *id,
                    texture_creator.create_texture_static(
                        pixel_format,
                        image.width(),
                        image.height(),
                    )?,
                );
            }
        }
        for id in self.modify.iter() {
            if let Some(image) = images.get(*id)
                && let Some(ref data) = image.data
                && let Some(texture) = textures.get_mut(id)
                && let Ok(pixel_size) = image.texture_descriptor.format.pixel_size()
            {
                let pitch = pixel_size * image.width() as usize;
                texture.update(None, data.as_slice(), pitch)?;
            }
        }
        Ok(())
    }
}

fn monitor_images(mut events: MessageReader<AssetEvent<Image>>, mut changes: ResMut<ImageChanges>) {
    changes.clear();
    for event in events.read() {
        match event {
            AssetEvent::Added { .. } => {}
            AssetEvent::Modified { id } => changes.modify.push(*id),
            AssetEvent::Removed { id } => changes.remove.push(*id),
            AssetEvent::Unused { id } => changes.remove.push(*id),
            AssetEvent::LoadedWithDependencies { id } => changes.add.push(*id),
        }
    }
}

use bevy::{asset::RenderAssetUsages, ecs::system::NonSendMarker, prelude::*};
use sdl2::pixels as sdlpixels;
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

use crate::{
    registry::SdlRegistry,
    render::{Render, RenderSystems},
};

pub struct SdlTexturePlugin;

impl Plugin for SdlTexturePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Render, prepare_textures.in_set(RenderSystems::Prepare));
    }
}

fn prepare_textures(
    mut events: MessageReader<AssetEvent<Image>>,
    images: Res<Assets<Image>>,
    _non_send: NonSendMarker,
) -> Result<()> {
    for event in events.read() {
        match event {
            AssetEvent::LoadedWithDependencies { .. } => {}
            AssetEvent::Modified { id } => {
                SdlRegistry::with_borrow_mut(|registry| -> Result<()> {
                    if let Some(image) = images.get(*id) {
                        registry.modify_texture_from_image(id, image)?
                    }
                    Ok(())
                })?;
            }
            AssetEvent::Removed { id } => {
                SdlRegistry::with_borrow_mut(|registry| registry.remove_texture(id))
            }
            AssetEvent::Unused { id } => {
                SdlRegistry::with_borrow_mut(|registry| registry.remove_texture(id))
            }
            AssetEvent::Added { id } => {
                SdlRegistry::with_borrow_mut(|registry| -> Result<()> {
                    if let Some(image) = images.get(*id) {
                        let pixel_format = match image.texture_descriptor.format {
                            wgpu_types::TextureFormat::Rgba8UnormSrgb => {
                                sdlpixels::PixelFormatEnum::RGBA32
                            }
                            wgpu_types::TextureFormat::Bgra8UnormSrgb => {
                                sdlpixels::PixelFormatEnum::BGRA32
                            }
                            _ => return Err("Unsupported image format".into()),
                        };
                        registry.create_texture(
                            *id,
                            pixel_format,
                            image.width(),
                            image.height(),
                        )?;
                        registry.modify_texture_from_image(id, image)?;
                    }
                    Ok(())
                })?;
            }
        }
    }
    Ok(())
}

pub trait FromColor {
    fn from_color(color: Color) -> Self;
}

impl FromColor for Image {
    fn from_color(color: Color) -> Self {
        let rgba: Srgba = color.into();
        let data = rgba.to_u8_array().to_vec();
        Image::new(
            Extent3d::default(),
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        )
    }
}

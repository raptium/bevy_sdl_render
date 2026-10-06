//! Decode an image straight into a Bevy [`Image`], with no asset server.
//!
//! The demo embeds its art with `include_bytes!`, so that a build is a single
//! self-contained binary with no asset directory to deploy or resolve at
//! runtime. `Image::from_buffer` would do the decoding, but it needs the
//! `ImageType`/`ImageSampler` plumbing that only matters to the asset loader;
//! going through the `image` crate directly is shorter and has no loader
//! configuration to get wrong.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

/// Decode PNG bytes into an `Image` in the one format the SDL backend uploads:
/// `Rgba8UnormSrgb`.
pub fn image_from_png(bytes: &[u8]) -> Result<Image> {
    let decoded = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|e| format!("decode png: {e}"))?
        .to_rgba8();
    let (width, height) = decoded.dimensions();
    Ok(Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        decoded.into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

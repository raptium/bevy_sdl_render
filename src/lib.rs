//! 2D sprite rendering for Bevy on top of SDL2, for hardware that has no
//! windowing system for `bevy_winit`/`wgpu` to use.
//!
//! See `README.md` for what that costs you. This fork is pinned by the `boat`
//! project, which builds for the TrimUI Brick (Allwinner A133P, PowerVR GE8300,
//! fbdev + PowerVR EGL, no X11/Wayland/DRM).

mod input;
mod loader;
mod registry;
mod render;
mod snapshot;
mod texture;

pub use input::SdlInput;
pub use loader::image_from_png;
pub use registry::SdlRegistry;
pub use render::SdlRenderPlugin;
pub use snapshot::{capture, SdlSnapshot};
pub use texture::FromColor;

//! Framebuffer capture.
//!
//! The Brick has no window manager and no screenshot tool, and the only way to
//! see what the game actually drew is to ask the renderer for its pixels. This
//! is also what makes the demo testable on a desktop: with `SDL_VIDEODRIVER=dummy`
//! the same code path renders headlessly and writes a PNG, so a change can be
//! inspected without a display.
//!
//! A capture is requested by writing to the [`SdlSnapshot`] resource. It is
//! serviced by the render schedule after every sprite for that frame has been
//! drawn and before `present()`, so the PNG is exactly the frame that was asked
//! for, not the previous one.

use std::path::PathBuf;

use bevy::prelude::*;

use crate::registry::SdlRegistry;

/// Requests a framebuffer capture.
#[derive(Resource, Default)]
pub struct SdlSnapshot {
    request: Option<PathBuf>,
    captures: u32,
}

impl SdlSnapshot {
    /// Ask for the frame currently being rendered to be written to `path`.
    /// A later request replaces an unserviced one.
    pub fn request(&mut self, path: impl Into<PathBuf>) {
        self.request = Some(path.into());
    }

    /// How many captures have been written this run.
    pub fn captures(&self) -> u32 {
        self.captures
    }

    pub(crate) fn take(&mut self) -> Option<PathBuf> {
        self.request.take()
    }

    pub(crate) fn done(&mut self) {
        self.captures += 1;
    }
}

/// Capture the current framebuffer to `path`. Returns the size written.
///
/// Mostly useful from the render schedule; from a game system, prefer
/// [`SdlSnapshot::request`] so the capture happens at a defined point in the
/// frame.
pub fn capture(path: impl AsRef<std::path::Path>) -> Result<(u32, u32)> {
    SdlRegistry::with_borrow(|registry| registry.capture(path.as_ref()))
}

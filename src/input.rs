//! SDL input state, republished so a game can read it.
//!
//! A game built on this backend cannot use Bevy's own input at all:
//!
//! * `ButtonInput<KeyCode>` is only ever populated by `bevy_winit`, which never
//!   runs here — this plugin replaces the runner before `bevy_winit` can
//!   create a window.
//! * `bevy_gilrs` is the usual answer for gamepads, but it enumerates devices
//!   through `libudev`, which the TrimUI Brick's OpenWrt rootfs does not ship.
//!   (The device does expose the pad, but as a bare evdev node; see
//!   `BOAT_EVDEV` in the demo for the direct-read path.)
//!
//! So rather than pretend, this module publishes exactly what SDL gives us:
//! a set of pressed keys, a set of pressed joystick button indices, joystick
//! axes and hats. Mapping those onto game actions is the game's job, because
//! only the game knows its buttons.
//!
//! Raw joystick buttons are used deliberately instead of SDL's game-controller
//! API: the controller layer needs an entry in SDL's gamecontrollerdb, and on
//! this device's "TRIMUI Player1" there is none, so no controller button event
//! is ever emitted. Joystick events do not need a mapping.

use std::collections::HashSet;

use bevy::prelude::*;
use sdl2::joystick::HatState;
use sdl2::keyboard::Keycode;

/// How many joystick axes / hats are tracked. SDL supports more, this is
/// already far past anything the target hardware has.
const MAX_AXES: usize = 16;
const MAX_HATS: usize = 4;

/// Live SDL input state. Updated once per frame in `First`, before `Update`.
#[derive(Resource, Debug)]
pub struct SdlInput {
    keys: HashSet<Keycode>,
    keys_just: HashSet<Keycode>,
    buttons: HashSet<u8>,
    buttons_just: HashSet<u8>,
    axes: [i16; MAX_AXES],
    hats: [(i8, i8); MAX_HATS],
    /// Set when SDL reports a device arriving or leaving, so the game can tell
    /// "the pad is not mapped" apart from "the pad was never seen".
    pub devices: usize,
    pub device_change: bool,
}

impl Default for SdlInput {
    fn default() -> Self {
        Self {
            keys: HashSet::new(),
            keys_just: HashSet::new(),
            buttons: HashSet::new(),
            buttons_just: HashSet::new(),
            axes: [0; MAX_AXES],
            hats: [(0, 0); MAX_HATS],
            devices: 0,
            device_change: false,
        }
    }
}

impl SdlInput {
    /// Drop the one-frame "just" sets. Called by the plugin at the top of the
    /// frame, before SDL events are drained.
    pub(crate) fn begin_frame(&mut self) {
        self.keys_just.clear();
        self.buttons_just.clear();
        self.device_change = false;
    }

    pub fn key_down(&self, key: Keycode) -> bool {
        self.keys.contains(&key)
    }

    pub fn key_just_down(&self, key: Keycode) -> bool {
        self.keys_just.contains(&key)
    }

    /// Any key at all — used by the idle watchdog.
    pub fn any_key_down(&self) -> bool {
        !self.keys.is_empty()
    }

    pub fn button_down(&self, button: u8) -> bool {
        self.buttons.contains(&button)
    }

    pub fn button_just_down(&self, button: u8) -> bool {
        self.buttons_just.contains(&button)
    }

    pub fn any_button_down(&self) -> bool {
        !self.buttons.is_empty()
    }

    /// Raw axis value, `-32768..=32767` for sticks. Out-of-range indices read 0.
    pub fn axis(&self, index: u8) -> i16 {
        self.axes.get(index as usize).copied().unwrap_or(0)
    }

    /// Hat as `(x, y)`, each `-1`, `0` or `1`.
    pub fn hat(&self, index: u8) -> (i8, i8) {
        self.hats.get(index as usize).copied().unwrap_or((0, 0))
    }

    pub(crate) fn press_key(&mut self, key: Keycode, repeat: bool) {
        if !repeat {
            self.keys_just.insert(key);
        }
        self.keys.insert(key);
    }

    pub(crate) fn release_key(&mut self, key: Keycode) {
        self.keys.remove(&key);
    }

    pub(crate) fn press_button(&mut self, button: u8) {
        self.buttons_just.insert(button);
        self.buttons.insert(button);
    }

    pub(crate) fn release_button(&mut self, button: u8) {
        self.buttons.remove(&button);
    }

    pub(crate) fn set_axis(&mut self, index: u8, value: i16) {
        if let Some(slot) = self.axes.get_mut(index as usize) {
            *slot = value;
        }
    }

    pub(crate) fn set_hat(&mut self, index: u8, state: HatState) {
        let raw = state.to_raw();
        let x = if raw & 0x02 != 0 {
            1
        } else if raw & 0x08 != 0 {
            -1
        } else {
            0
        };
        let y = if raw & 0x01 != 0 {
            1
        } else if raw & 0x04 != 0 {
            -1
        } else {
            0
        };
        if let Some(slot) = self.hats.get_mut(index as usize) {
            *slot = (x, y);
        }
    }
}

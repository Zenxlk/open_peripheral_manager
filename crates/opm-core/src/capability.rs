//! The `Capability` pattern: how a [`crate::device::Device`] exposes
//! *optional* per-device features (RGB, battery, profiles, ...) without
//! forcing every device to implement every possible feature. See
//! `docs/architecture/driver-model.md`'s "The `Capability` pattern" for
//! why this is explicit accessor methods on `Device` rather than
//! `dyn Any` downcasting.
//!
//! Every method returns a `Result` — talking to real hardware can always
//! fail, so none of these can honestly be infallible.

use crate::error::Error;

/// An RGB color, 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

/// Devices that can report and change a solid RGB color.
///
/// Deliberately minimal for Phase 3/4 — per-key color, effects/
/// animations, and brightness are real AK820 features but are Phase 6
/// protocol questions, not part of proving the trait shape end-to-end.
/// Extend once a real driver's `Protocol` work needs more.
pub trait Rgb: Send {
    /// Reads the device's current color.
    fn get_color(&self) -> Result<RgbColor, Error>;
    /// Sets the device's color.
    fn set_color(&self, color: RgbColor) -> Result<(), Error>;
}

/// Devices that report a battery level (wireless keyboards, mice, ...).
pub trait Battery: Send {
    /// Battery level, `0`-`100`.
    fn level_percent(&self) -> Result<u8, Error>;
}

/// Devices that store multiple configuration profiles switchable at
/// runtime.
pub trait Profiles: Send {
    /// The currently active profile's index.
    fn active_profile(&self) -> Result<u8, Error>;
    /// Switches to a different stored profile.
    fn set_active_profile(&self, profile: u8) -> Result<(), Error>;
}

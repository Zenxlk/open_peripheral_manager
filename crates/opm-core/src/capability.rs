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
/// Deliberately minimal — one solid color, nothing else. A device that
/// also supports animated modes/speed/direction implements [`Lighting`]
/// as a separate capability; see
/// `docs/architecture/decisions/0005-lighting-capability-and-shared-effect-vocabulary.md`
/// for why these weren't merged into one trait.
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

/// Devices that support animated lighting effects — multiple modes,
/// each with its own color/brightness/speed/direction — beyond a single
/// solid [`RgbColor`].
///
/// This vocabulary ([`LightingMode`], [`Direction`], [`LightingEffect`])
/// is shaped around what this project's one real driver (the AK820)
/// actually exposes, not a guessed-at cross-vendor generalization — see
/// ADR 0005 for why, and revisit if a second driver's needs don't fit.
pub trait Lighting: Send {
    /// Applies a full lighting effect.
    fn set_effect(&self, effect: LightingEffect) -> Result<(), Error>;
}

/// Devices with an idle timer that puts their lighting to sleep after a
/// period of inactivity.
///
/// Same reasoning as [`Lighting`] (see ADR 0005): a separate capability
/// rather than folding into `Lighting`, since not every lighting-capable
/// device necessarily has an idle timer, and [`SleepTime`]'s vocabulary
/// is shaped around what the AK820 exposes today.
pub trait SleepTimer: Send {
    /// Sets how long the device stays idle before its lighting sleeps.
    fn set_sleep_time(&self, time: SleepTime) -> Result<(), Error>;
}

/// How long a device waits, idle, before its lighting sleeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SleepTime {
    /// Never sleep.
    Never = 0,
    /// Sleep after one minute idle.
    OneMinute = 1,
    /// Sleep after five minutes idle.
    FiveMinutes = 2,
    /// Sleep after thirty minutes idle.
    ThirtyMinutes = 3,
}

impl SleepTime {
    /// Every preset.
    pub const ALL: &'static [SleepTime] = &[
        Self::Never,
        Self::OneMinute,
        Self::FiveMinutes,
        Self::ThirtyMinutes,
    ];

    /// A lowercase-hyphenated name, for CLI display/parsing.
    pub fn name(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::OneMinute => "1m",
            Self::FiveMinutes => "5m",
            Self::ThirtyMinutes => "30m",
        }
    }

    /// Parses [`Self::name`]'s output back into a preset, case-insensitive.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|t| t.name().eq_ignore_ascii_case(name))
            .copied()
    }
}

/// A full lighting effect: which [`LightingMode`] to run, its color, and
/// its brightness/speed/direction. `brightness`/`speed` are raw,
/// device-specific ranges (0-5 on the AK820 today) — see ADR 0005 for
/// why these aren't normalized to a percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightingEffect {
    /// Which animation (or `Static`/`Off`) to run.
    pub mode: LightingMode,
    /// The effect's primary color. Ignored by modes that don't use one.
    pub color: RgbColor,
    /// Brightness, device-specific range.
    pub brightness: u8,
    /// Animation speed, device-specific range. Ignored by non-animated
    /// modes.
    pub speed: u8,
    /// Which way the animation runs, for modes that support one — see
    /// [`LightingMode::supported_directions`].
    pub direction: Direction,
}

/// Every lighting effect mode the AK820 exposes. Numeric values are the
/// wire opcode on that device (`repr(u8)`, matched 1:1 in
/// `drivers/opm-driver-ajazz-ak820/src/protocol.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LightingMode {
    /// Lighting off entirely.
    Off = 0x00,
    /// A single solid color.
    Static = 0x01,
    /// A single key lit on press.
    SingleOn = 0x02,
    /// A single key lit, then turned off on press.
    SingleOff = 0x03,
    /// Random keys glittering.
    Glittering = 0x04,
    /// A falling-keys animation.
    Falling = 0x05,
    /// A colorful, multi-hue animation.
    Colourful = 0x06,
    /// A breathing (fade in/out) effect.
    Breath = 0x07,
    /// A spectrum-cycling animation.
    Spectrum = 0x08,
    /// An animation radiating outward from the center.
    Outward = 0x09,
    /// A scrolling animation; supports [`Direction::Up`]/[`Direction::Down`].
    Scrolling = 0x0a,
    /// A rolling animation; supports [`Direction::Left`]/[`Direction::Right`].
    Rolling = 0x0b,
    /// A rotating animation.
    Rotating = 0x0c,
    /// An exploding animation.
    Explode = 0x0d,
    /// A launching animation.
    Launch = 0x0e,
    /// A ripple animation.
    Ripples = 0x0f,
    /// A flowing animation; supports [`Direction::Left`]/[`Direction::Right`].
    Flowing = 0x10,
    /// A pulsating animation.
    Pulsating = 0x11,
    /// A tilt animation; supports [`Direction::Left`]/[`Direction::Right`].
    Tilt = 0x12,
    /// A shuttle animation.
    Shuttle = 0x13,
}

impl LightingMode {
    /// Every mode, in wire-opcode order.
    pub const ALL: &'static [LightingMode] = &[
        Self::Off,
        Self::Static,
        Self::SingleOn,
        Self::SingleOff,
        Self::Glittering,
        Self::Falling,
        Self::Colourful,
        Self::Breath,
        Self::Spectrum,
        Self::Outward,
        Self::Scrolling,
        Self::Rolling,
        Self::Rotating,
        Self::Explode,
        Self::Launch,
        Self::Ripples,
        Self::Flowing,
        Self::Pulsating,
        Self::Tilt,
        Self::Shuttle,
    ];

    /// A lowercase-hyphenated name, for CLI display/parsing (e.g.
    /// `pmctl lighting set --mode single-on`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Static => "static",
            Self::SingleOn => "single-on",
            Self::SingleOff => "single-off",
            Self::Glittering => "glittering",
            Self::Falling => "falling",
            Self::Colourful => "colourful",
            Self::Breath => "breath",
            Self::Spectrum => "spectrum",
            Self::Outward => "outward",
            Self::Scrolling => "scrolling",
            Self::Rolling => "rolling",
            Self::Rotating => "rotating",
            Self::Explode => "explode",
            Self::Launch => "launch",
            Self::Ripples => "ripples",
            Self::Flowing => "flowing",
            Self::Pulsating => "pulsating",
            Self::Tilt => "tilt",
            Self::Shuttle => "shuttle",
        }
    }

    /// Parses [`Self::name`]'s output back into a mode, case-insensitive.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|m| m.name().eq_ignore_ascii_case(name))
            .copied()
    }

    /// Which [`Direction`]s this mode honours, if any.
    pub fn supported_directions(self) -> &'static [Direction] {
        match self {
            Self::Scrolling => &[Direction::Up, Direction::Down],
            Self::Rolling | Self::Flowing | Self::Tilt => &[Direction::Left, Direction::Right],
            _ => &[],
        }
    }
}

/// The direction an animated [`LightingMode`] runs in, for the modes
/// that support one (see [`LightingMode::supported_directions`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Direction {
    /// Leftward.
    Left = 0,
    /// Downward.
    Down = 1,
    /// Upward.
    Up = 2,
    /// Rightward.
    Right = 3,
}

impl Direction {
    /// Every direction.
    pub const ALL: &'static [Direction] = &[Self::Left, Self::Down, Self::Up, Self::Right];

    /// A lowercase name, for CLI display/parsing.
    pub fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Down => "down",
            Self::Up => "up",
            Self::Right => "right",
        }
    }

    /// Parses [`Self::name`]'s output back into a direction, case-insensitive.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|d| d.name().eq_ignore_ascii_case(name))
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lighting_mode_name_round_trips() {
        for mode in LightingMode::ALL {
            assert_eq!(LightingMode::from_name(mode.name()), Some(*mode));
        }
    }

    #[test]
    fn lighting_mode_from_name_is_case_insensitive() {
        assert_eq!(
            LightingMode::from_name("STATIC"),
            Some(LightingMode::Static)
        );
        assert_eq!(
            LightingMode::from_name("Single-On"),
            Some(LightingMode::SingleOn)
        );
    }

    #[test]
    fn lighting_mode_from_name_rejects_unknown() {
        assert_eq!(LightingMode::from_name("not-a-real-mode"), None);
    }

    #[test]
    fn direction_name_round_trips() {
        for direction in Direction::ALL {
            assert_eq!(Direction::from_name(direction.name()), Some(*direction));
        }
    }

    #[test]
    fn direction_from_name_rejects_unknown() {
        assert_eq!(Direction::from_name("sideways"), None);
    }

    #[test]
    fn sleep_time_name_round_trips() {
        for time in SleepTime::ALL {
            assert_eq!(SleepTime::from_name(time.name()), Some(*time));
        }
    }

    #[test]
    fn sleep_time_from_name_rejects_unknown() {
        assert_eq!(SleepTime::from_name("2 hours"), None);
    }
}

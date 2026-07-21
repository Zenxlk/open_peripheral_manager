//! AK820 lighting-mode vocabulary and packet layout.
//!
//! Ported from [gohv/EPOMAKER-Ajazz-AK820-Pro](https://github.com/gohv/EPOMAKER-Ajazz-AK820-Pro)'s
//! `src/protocol.rs` (itself built on
//! [TaxMachine/ajazz-keyboard-software-linux](https://github.com/TaxMachine/ajazz-keyboard-software-linux)'s
//! original reverse-engineering), adapted to this crate's `Transport`
//! trait — see `docs/protocols/ajazz-ak820/findings.md`'s 2026-07-20
//! "root cause" entries for the full writeup, credits, and how this
//! was verified against real hardware. Neither source repo has a
//! `LICENSE` file GitHub recognizes (see findings.md); this is an
//! independent re-expression in this project's own code style of the
//! protocol facts they discovered (opcodes, byte offsets, enum
//! values), not a copy of their source files.
//!
//! Only a solid color (via `lib.rs`'s `LightingMode::Static` →
//! `LightingMode::Breath` substitution) is wired into a real
//! `Capability` so far (`Rgb::set_color`). The rest of this vocabulary
//! — every other lighting mode, [`Direction`], [`SleepTime`] — is
//! carried over ready to use, but nothing in `opm_core::capability`
//! exposes them yet; see `docs/roadmap.md`'s Phase 6 known gaps.

/// The Report ID shared by every `START`/`START_MODE`/`FINISH`/`SLEEP`
/// control packet — distinct from a lighting-data packet's Report ID,
/// which is that packet's own mode value.
pub const CONTROL_REPORT_ID: u8 = 0x04;
/// Control-packet command byte: begin a configuration transaction.
pub const CMD_START: u8 = 0x18;
/// Control-packet command byte: the packet that follows is a lighting
/// mode+color data packet.
pub const CMD_MODE: u8 = 0x13;
/// Control-packet command byte: end a configuration transaction.
pub const CMD_FINISH: u8 = 0xf0;
/// Control-packet command byte: the packet that follows is a sleep-timer
/// data packet. Not wired into a `Capability` yet.
pub const CMD_SLEEP: u8 = 0x17;

/// The highest brightness value the device accepts.
pub const MAX_BRIGHTNESS: u8 = 5;
/// The highest animation-speed value the device accepts.
pub const MAX_SPEED: u8 = 5;

/// Every lighting effect the AK820 (Pro and this project's `0x800a`
/// unit alike) exposes. Numeric values are the wire opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LightingMode {
    /// Lighting off entirely.
    Off = 0x00,
    /// A single solid color. Requesting this directly does not work on
    /// real hardware — see `lib.rs`'s `Static` → `Breath` substitution.
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
    /// A breathing (fade in/out) effect. `speed = 0` renders as a
    /// static color — the substitution `Static` actually uses.
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

/// The direction an animated [`LightingMode`] runs in, for the modes
/// that support one.
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

/// How long the keyboard stays idle before its lighting sleeps. Not
/// wired into a `Capability` yet — see the module docs.
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

/// Builds a `START`/`START_MODE`/`FINISH`/`SLEEP`-style control
/// packet: byte 0 is [`CONTROL_REPORT_ID`], byte 1 the command, byte 8
/// a flag every real capture of these commands carried set to `0x01`.
pub fn control_packet(command: u8, byte8: u8) -> [u8; 64] {
    let mut pkt = [0u8; 64];
    pkt[0] = CONTROL_REPORT_ID;
    pkt[1] = command;
    pkt[8] = byte8;
    pkt
}

/// Builds a lighting mode+color data packet. Byte 0 (the mode value)
/// doubles as this packet's own wire Report ID.
#[allow(clippy::too_many_arguments)]
pub fn mode_data_packet(
    mode: LightingMode,
    r: u8,
    g: u8,
    b: u8,
    rainbow: bool,
    brightness: u8,
    speed: u8,
    direction: Direction,
) -> [u8; 64] {
    let mut pkt = [0u8; 64];
    pkt[0] = mode as u8;
    pkt[1] = r;
    pkt[2] = g;
    pkt[3] = b;
    pkt[8] = u8::from(rainbow);
    pkt[9] = brightness.min(MAX_BRIGHTNESS);
    pkt[10] = speed.min(MAX_SPEED);
    pkt[11] = direction as u8;
    pkt[14] = 0x55;
    pkt[15] = 0xaa;
    pkt
}

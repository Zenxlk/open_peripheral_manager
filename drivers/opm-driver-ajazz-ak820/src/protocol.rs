//! AK820 wire-level packet layout.
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
//! The mode/direction/sleep-timer vocabulary itself
//! (`LightingMode`/`Direction`/`LightingEffect`/`SleepTime`) lives in
//! `opm_core::capability` as of
//! `docs/architecture/decisions/0005-lighting-capability-and-shared-effect-vocabulary.md`
//! — this module only owns what's genuinely wire-specific: the
//! control-packet command bytes and the packet-layout builders.

use opm_core::capability::{Direction, LightingMode, SleepTime};

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
/// data packet.
pub const CMD_SLEEP: u8 = 0x17;

/// The highest brightness value the device accepts.
pub const MAX_BRIGHTNESS: u8 = 5;
/// The highest animation-speed value the device accepts.
pub const MAX_SPEED: u8 = 5;

/// Builds a `START`/`START_MODE`/`FINISH`/`SLEEP`-style control packet:
/// byte 0 is [`CONTROL_REPORT_ID`], byte 1 the command, byte 2 a
/// per-command-family value (`0x00` for `START`/`MODE`/`FINISH`, `0x01`
/// for `SLEEP` — ported from gohv, meaning not otherwise understood),
/// byte 8 a flag every real capture of these commands carried set to
/// `0x01`.
pub fn control_packet(command: u8, byte2: u8, byte8: u8) -> [u8; 64] {
    let mut pkt = [0u8; 64];
    pkt[0] = CONTROL_REPORT_ID;
    pkt[1] = command;
    pkt[2] = byte2;
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

/// Builds a sleep-timer data packet. Unlike [`mode_data_packet`], byte 0
/// stays `0` — this packet has no per-value "own report ID" convention,
/// per gohv's `sleep_data_packet`. Note the footer's byte order (`0xaa`
/// then `0x55`, at the packet's *last* two bytes) is reversed from
/// [`mode_data_packet`]'s (`0x55` then `0xaa`, at bytes 14-15) — another
/// unexplained per-command-family footer difference, same category as
/// the `05 03`/`AA 55` bytes findings.md already flags as unexplored.
pub fn sleep_data_packet(sleep_time: SleepTime) -> [u8; 64] {
    let mut pkt = [0u8; 64];
    pkt[8] = sleep_time as u8;
    pkt[62] = 0xaa;
    pkt[63] = 0x55;
    pkt
}

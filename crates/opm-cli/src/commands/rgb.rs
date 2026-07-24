//! `pmctl rgb` — reads or sets a device's RGB color, for devices whose
//! `opm-core` driver exposes the `Rgb` capability. Against every driver
//! that exists today (just `opm-driver-ajazz-ak820`), this always fails
//! with "not yet implemented" — see `docs/roadmap.md`'s Phase 6. This
//! command proves the CLI wiring works, not that RGB does anything real.

use opm_core::capability::RgbColor;

use super::{device, registry};

/// `pmctl rgb`'s arguments.
#[derive(Debug, clap::Args)]
pub struct Args {
    /// Which device to target, as `VID:PID` hex (e.g. `0c45:800a`).
    /// Required only when more than one supported device is detected.
    #[arg(long, value_name = "VID:PID")]
    device: Option<String>,

    #[command(subcommand)]
    action: Action,
}

/// `pmctl rgb`'s subcommands.
#[derive(Debug, clap::Subcommand)]
enum Action {
    /// Reads the current color.
    Get,
    /// Sets the color.
    Set {
        /// `RRGGBB` hex, e.g. `ff0000` for red.
        color: String,
    },
}

/// Runs the `rgb` subcommand.
pub fn run(args: Args) {
    let registry = registry::build();
    let supported = device::supported_devices(&registry);
    let found = device::select_device(&supported, args.device.as_deref());
    let opened = device::open_or_exit(&registry, found);

    let outcome = act(opened.as_ref(), args);

    // Drop the device (and its Transport) explicitly before exiting.
    // Every exit below goes through std::process::exit, which skips
    // destructors — some Transport implementations need theirs to run
    // (LibusbTransport re-attaches the kernel HID driver it detached on
    // open, see ADR 0004); skip it and every later invocation targeting
    // the same interface fails until a manual driver rebind.
    drop(opened);

    match outcome {
        Ok(message) => {
            if let Some(message) = message {
                println!("{message}");
            }
            std::process::exit(device::EXIT_OK);
        }
        Err((message, code)) => {
            eprintln!("{message}");
            std::process::exit(code);
        }
    }
}

/// Runs `args.action` against an already-opened device, returning what
/// to print and exit with instead of doing either directly — so `run`
/// can drop the device first. `Ok(None)` prints nothing (still exits
/// [`device::EXIT_OK`]).
fn act(opened: &dyn opm_core::device::Device, args: Args) -> Result<Option<String>, (String, i32)> {
    let Some(rgb) = opened.rgb() else {
        return Err((
            "this device does not support RGB".to_owned(),
            device::EXIT_FAILED,
        ));
    };

    match args.action {
        Action::Get => rgb
            .get_color()
            .map(|color| Some(format!("{:02x}{:02x}{:02x}", color.r, color.g, color.b)))
            .map_err(|err| (format!("failed to read color: {err}"), device::EXIT_FAILED)),
        Action::Set { color } => {
            let parsed = parse_hex_color(&color).map_err(|err| {
                (
                    format!("invalid color {color:?}: {err}"),
                    device::EXIT_USAGE,
                )
            })?;
            rgb.set_color(parsed)
                .map(|()| Some("color set".to_owned()))
                .map_err(|err| (format!("failed to set color: {err}"), device::EXIT_FAILED))
        }
    }
}

/// Parses `RRGGBB` hex into an [`RgbColor`] — shared with `lighting.rs`'s
/// `--color` flag.
pub(super) fn parse_hex_color(s: &str) -> Result<RgbColor, String> {
    let s = s.trim_start_matches('#');
    // `s.len()` is a byte count, not a char count — a 6-*byte* string
    // can still contain a multi-byte UTF-8 character whose boundary
    // falls inside one of the byte-index slices below (e.g. "aé1é" is
    // 6 bytes but only 4 chars). Slicing at a non-char-boundary index
    // panics; confirmed by direct testing, not hypothetical. Requiring
    // ASCII first guarantees every subsequent byte index is a valid
    // char boundary, since ASCII chars are always exactly 1 byte.
    if s.len() != 6 || !s.is_ascii() {
        return Err("expected 6 hex digits, e.g. ff0000".to_owned());
    }
    let byte = |range: std::ops::Range<usize>| u8::from_str_radix(&s[range], 16);
    Ok(RgbColor {
        r: byte(0..2).map_err(|err| err.to_string())?,
        g: byte(2..4).map_err(|err| err.to_string())?,
        b: byte(4..6).map_err(|err| err.to_string())?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_valid_color() {
        assert_eq!(
            parse_hex_color("ff0000"),
            Ok(RgbColor {
                r: 0xff,
                g: 0,
                b: 0
            })
        );
        assert_eq!(
            parse_hex_color("#7c5cff"),
            Ok(RgbColor {
                r: 0x7c,
                g: 0x5c,
                b: 0xff
            })
        );
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(parse_hex_color("fff").is_err());
        assert!(parse_hex_color("ff00000").is_err());
    }

    #[test]
    fn rejects_non_hex_ascii() {
        assert!(parse_hex_color("zzzzzz").is_err());
    }

    #[test]
    fn rejects_multibyte_utf8_without_panicking() {
        // 6 *bytes*, but a multi-byte char straddles a slice boundary
        // — this used to panic ("byte index 2 is not a char
        // boundary"), confirmed by direct testing before the fix.
        assert!(parse_hex_color("aé1é").is_err());
    }
}

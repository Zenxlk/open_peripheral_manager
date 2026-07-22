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
    if s.len() != 6 {
        return Err("expected 6 hex digits, e.g. ff0000".to_owned());
    }
    let byte = |range: std::ops::Range<usize>| u8::from_str_radix(&s[range], 16);
    Ok(RgbColor {
        r: byte(0..2).map_err(|err| err.to_string())?,
        g: byte(2..4).map_err(|err| err.to_string())?,
        b: byte(4..6).map_err(|err| err.to_string())?,
    })
}

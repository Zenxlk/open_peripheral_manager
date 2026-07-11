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

    let Some(rgb) = opened.rgb() else {
        eprintln!("this device does not support RGB");
        std::process::exit(device::EXIT_FAILED);
    };

    match args.action {
        Action::Get => match rgb.get_color() {
            Ok(color) => println!("{:02x}{:02x}{:02x}", color.r, color.g, color.b),
            Err(err) => {
                eprintln!("failed to read color: {err}");
                std::process::exit(device::EXIT_FAILED);
            }
        },
        Action::Set { color } => {
            let parsed = parse_hex_color(&color).unwrap_or_else(|err| {
                eprintln!("invalid color {color:?}: {err}");
                std::process::exit(device::EXIT_USAGE);
            });
            match rgb.set_color(parsed) {
                Ok(()) => println!("color set"),
                Err(err) => {
                    eprintln!("failed to set color: {err}");
                    std::process::exit(device::EXIT_FAILED);
                }
            }
        }
    }

    std::process::exit(device::EXIT_OK);
}

fn parse_hex_color(s: &str) -> Result<RgbColor, String> {
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

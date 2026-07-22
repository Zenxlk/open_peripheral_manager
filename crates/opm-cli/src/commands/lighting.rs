//! `pmctl lighting` — applies a full lighting effect (mode, color,
//! brightness, speed, direction), for devices whose `opm-core` driver
//! exposes the `Lighting` capability. See `docs/roadmap.md`'s Phase 6a
//! and `docs/architecture/decisions/0005-lighting-capability-and-shared-effect-vocabulary.md`.

use opm_core::capability::{Direction, LightingEffect, LightingMode};

use super::rgb::parse_hex_color;
use super::{device, registry};

/// `pmctl lighting`'s arguments.
#[derive(Debug, clap::Args)]
pub struct Args {
    /// Which device to target, as `VID:PID` hex (e.g. `0c45:800a`).
    /// Required only when more than one supported device is detected.
    /// Ignored by `modes`, which doesn't need a device.
    #[arg(long, value_name = "VID:PID")]
    device: Option<String>,

    #[command(subcommand)]
    action: Action,
}

/// `pmctl lighting`'s subcommands.
#[derive(Debug, clap::Subcommand)]
enum Action {
    /// Applies a lighting effect.
    Set {
        /// Effect mode name — see `pmctl lighting modes` for the full
        /// list (e.g. `static`, `breath`, `spectrum`, `ripples`).
        #[arg(long)]
        mode: String,
        /// `RRGGBB` hex color, e.g. `ff0000` for red. Ignored by modes
        /// that don't use a color.
        #[arg(long, default_value = "ffffff")]
        color: String,
        /// Brightness, device-specific range (0-5 on the AK820).
        #[arg(long, default_value_t = 5)]
        brightness: u8,
        /// Animation speed, device-specific range (0-5 on the AK820).
        /// Ignored by non-animated modes.
        #[arg(long, default_value_t = 3)]
        speed: u8,
        /// Direction the animation runs, for modes that support one:
        /// `left`, `down`, `up`, or `right`.
        #[arg(long, default_value = "left")]
        direction: String,
    },
    /// Lists every supported mode name.
    Modes,
}

/// Runs the `lighting` subcommand.
pub fn run(args: Args) {
    if matches!(args.action, Action::Modes) {
        for mode in LightingMode::ALL {
            println!("{}", mode.name());
        }
        std::process::exit(device::EXIT_OK);
    }

    let registry = registry::build();
    let supported = device::supported_devices(&registry);
    let found = device::select_device(&supported, args.device.as_deref());
    let opened = device::open_or_exit(&registry, found);

    let outcome = act(opened.as_ref(), args);

    // Drop the device (and its Transport) explicitly before exiting —
    // see rgb.rs's `run` for why (ADR 0004, LibusbTransport's Drop
    // re-attaching the kernel HID driver).
    drop(opened);

    match outcome {
        Ok(message) => {
            println!("{message}");
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
/// can drop the device first. [`Action::Modes`] never reaches here —
/// `run` handles it before opening any device.
fn act(opened: &dyn opm_core::device::Device, args: Args) -> Result<String, (String, i32)> {
    let Some(lighting) = opened.lighting() else {
        return Err((
            "this device does not support lighting effects".to_owned(),
            device::EXIT_FAILED,
        ));
    };

    let Action::Set {
        mode,
        color,
        brightness,
        speed,
        direction,
    } = args.action
    else {
        unreachable!("Action::Modes is handled in run() before opening a device");
    };

    let parsed_mode = LightingMode::from_name(&mode).ok_or_else(|| {
        (
            format!("unknown mode {mode:?} — see `pmctl lighting modes`"),
            device::EXIT_USAGE,
        )
    })?;
    let parsed_color = parse_hex_color(&color).map_err(|err| {
        (
            format!("invalid color {color:?}: {err}"),
            device::EXIT_USAGE,
        )
    })?;
    let parsed_direction = Direction::from_name(&direction).ok_or_else(|| {
        (
            format!("unknown direction {direction:?} — expected left, down, up, or right"),
            device::EXIT_USAGE,
        )
    })?;

    lighting
        .set_effect(LightingEffect {
            mode: parsed_mode,
            color: parsed_color,
            brightness,
            speed,
            direction: parsed_direction,
        })
        .map(|()| format!("lighting set to {}", parsed_mode.name()))
        .map_err(|err| {
            (
                format!("failed to set lighting effect: {err}"),
                device::EXIT_FAILED,
            )
        })
}

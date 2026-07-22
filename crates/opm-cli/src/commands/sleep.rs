//! `pmctl sleep` — sets a device's idle lighting-sleep timer, for
//! devices whose `opm-core` driver exposes the `SleepTimer` capability.
//! See `docs/roadmap.md`'s Phase 6c.

use opm_core::capability::SleepTime;

use super::{device, registry};

/// `pmctl sleep`'s arguments.
#[derive(Debug, clap::Args)]
pub struct Args {
    /// Which device to target, as `VID:PID` hex (e.g. `0c45:800a`).
    /// Required only when more than one supported device is detected.
    /// Ignored by `presets`, which doesn't need a device.
    #[arg(long, value_name = "VID:PID")]
    device: Option<String>,

    #[command(subcommand)]
    action: Action,
}

/// `pmctl sleep`'s subcommands.
#[derive(Debug, clap::Subcommand)]
enum Action {
    /// Sets the idle sleep timer.
    Set {
        /// Preset name — see `pmctl sleep presets` for the full list
        /// (e.g. `never`, `1m`, `5m`, `30m`).
        preset: String,
    },
    /// Lists every supported preset name.
    Presets,
}

/// Runs the `sleep` subcommand.
pub fn run(args: Args) {
    if matches!(args.action, Action::Presets) {
        for preset in SleepTime::ALL {
            println!("{}", preset.name());
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
/// can drop the device first. [`Action::Presets`] never reaches here —
/// `run` handles it before opening any device.
fn act(opened: &dyn opm_core::device::Device, args: Args) -> Result<String, (String, i32)> {
    let Some(sleep_timer) = opened.sleep_timer() else {
        return Err((
            "this device does not support a sleep timer".to_owned(),
            device::EXIT_FAILED,
        ));
    };

    let Action::Set { preset } = args.action else {
        unreachable!("Action::Presets is handled in run() before opening a device");
    };

    let parsed = SleepTime::from_name(&preset).ok_or_else(|| {
        (
            format!("unknown preset {preset:?} — see `pmctl sleep presets`"),
            device::EXIT_USAGE,
        )
    })?;

    sleep_timer
        .set_sleep_time(parsed)
        .map(|()| format!("sleep timer set to {}", parsed.name()))
        .map_err(|err| {
            (
                format!("failed to set sleep timer: {err}"),
                device::EXIT_FAILED,
            )
        })
}

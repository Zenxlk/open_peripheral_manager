//! `pmctl profile` — reads or switches a device's active profile, for
//! devices whose `opm-core` driver exposes the `Profiles` capability.
//! Against every driver that exists today (just
//! `opm-driver-ajazz-ak820`), this always fails with "not yet
//! implemented" — see `docs/roadmap.md`'s Phase 6. This command proves
//! the CLI wiring works, not that profile switching does anything real.

use super::{device, registry};

/// `pmctl profile`'s arguments.
#[derive(Debug, clap::Args)]
pub struct Args {
    /// Which device to target, as `VID:PID` hex (e.g. `0c45:800a`).
    /// Required only when more than one supported device is detected.
    #[arg(long, value_name = "VID:PID")]
    device: Option<String>,

    #[command(subcommand)]
    action: Action,
}

/// `pmctl profile`'s subcommands.
#[derive(Debug, clap::Subcommand)]
enum Action {
    /// Shows the currently active profile.
    Get,
    /// Switches to a different profile.
    Set {
        /// The profile index to switch to.
        profile: u8,
    },
}

/// Runs the `profile` subcommand.
pub fn run(args: Args) {
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
/// can drop the device first.
fn act(opened: &dyn opm_core::device::Device, args: Args) -> Result<String, (String, i32)> {
    let Some(profiles) = opened.profiles() else {
        return Err((
            "this device does not support profiles".to_owned(),
            device::EXIT_FAILED,
        ));
    };

    match args.action {
        Action::Get => profiles
            .active_profile()
            .map(|active| active.to_string())
            .map_err(|err| {
                (
                    format!("failed to read active profile: {err}"),
                    device::EXIT_FAILED,
                )
            }),
        Action::Set { profile } => profiles
            .set_active_profile(profile)
            .map(|()| format!("switched to profile {profile}"))
            .map_err(|err| {
                (
                    format!("failed to switch profile: {err}"),
                    device::EXIT_FAILED,
                )
            }),
    }
}

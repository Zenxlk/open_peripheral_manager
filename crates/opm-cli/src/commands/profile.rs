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

    let Some(profiles) = opened.profiles() else {
        eprintln!("this device does not support profiles");
        std::process::exit(device::EXIT_FAILED);
    };

    match args.action {
        Action::Get => match profiles.active_profile() {
            Ok(active) => println!("{active}"),
            Err(err) => {
                eprintln!("failed to read active profile: {err}");
                std::process::exit(device::EXIT_FAILED);
            }
        },
        Action::Set { profile } => match profiles.set_active_profile(profile) {
            Ok(()) => println!("switched to profile {profile}"),
            Err(err) => {
                eprintln!("failed to switch profile: {err}");
                std::process::exit(device::EXIT_FAILED);
            }
        },
    }

    std::process::exit(device::EXIT_OK);
}

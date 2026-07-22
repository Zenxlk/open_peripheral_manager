//! `pmctl` — command-line front-end for the Open Peripheral Manager.
//!
//! `discover` (Phase 1) and `list`/`info`/`rgb`/`profile` (Phase 5) are
//! all implemented, backed by an explicit `DriverRegistry` — see
//! `commands::registry` and `docs/architecture/driver-model.md`. `rgb`/
//! `profile` always fail against every driver that exists today, since
//! `opm-driver-ajazz-ak820`'s capabilities are still Phase 6 stubs (see
//! `docs/roadmap.md`) — that's expected, not a bug in this wiring.

mod commands;

use clap::{Parser, Subcommand};

/// Open Peripheral Manager command-line tool.
#[derive(Debug, Parser)]
#[command(name = "pmctl", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Top-level `pmctl` subcommands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Enumerate every HID device on the system, supported or not.
    Discover(commands::discover::Args),
    /// List peripherals recognized by a registered driver.
    List,
    /// Show detailed information about a peripheral.
    Info(commands::info::Args),
    /// Configure RGB lighting.
    Rgb(commands::rgb::Args),
    /// Manage device profiles.
    Profile(commands::profile::Args),
    /// Apply an animated lighting effect (mode, brightness, speed,
    /// direction), beyond a single solid color.
    Lighting(commands::lighting::Args),
    /// Set the idle lighting-sleep timer.
    Sleep(commands::sleep::Args),
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Discover(args) => commands::discover::run(args),
        Command::List => commands::list::run(),
        Command::Info(args) => commands::info::run(args),
        Command::Rgb(args) => commands::rgb::run(args),
        Command::Profile(args) => commands::profile::run(args),
        Command::Lighting(args) => commands::lighting::run(args),
        Command::Sleep(args) => commands::sleep::run(args),
    }
}

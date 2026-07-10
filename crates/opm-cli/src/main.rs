//! `pmctl` — command-line front-end for the Open Peripheral Manager.
//!
//! This binary is currently a **structural placeholder**: it wires up the
//! subcommands described in the project README (`list`, `info`, `rgb`,
//! `profile`) so the shape of the CLI is settled early, but none of them
//! do real work yet. Behavior will be implemented once `opm-core` exposes
//! a driver registry to query — see `docs/architecture/driver-model.md`.

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
    Discover,
    /// List detected peripherals.
    List,
    /// Show detailed information about a peripheral.
    Info,
    /// Configure RGB lighting.
    Rgb,
    /// Manage device profiles.
    Profile,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Discover => commands::discover::run(),
        Command::List => commands::list::run(),
        Command::Info => commands::info::run(),
        Command::Rgb => commands::rgb::run(),
        Command::Profile => commands::profile::run(),
    }
}

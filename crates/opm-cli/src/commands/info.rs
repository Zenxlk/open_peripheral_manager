//! `pmctl info` — shows detailed information about one specific
//! supported peripheral: identity and which capabilities it exposes.

use super::{device, registry};

/// `pmctl info`'s arguments.
#[derive(Debug, clap::Args)]
pub struct Args {
    /// Which device to target, as `VID:PID` hex (e.g. `0c45:800a`).
    /// Required only when more than one supported device is detected.
    #[arg(long, value_name = "VID:PID")]
    device: Option<String>,
}

/// Runs the `info` subcommand.
pub fn run(args: Args) {
    let registry = registry::build();
    let supported = device::supported_devices(&registry);
    let found = device::select_device(&supported, args.device.as_deref());
    let opened = device::open_or_exit(&registry, found);
    let identity = opened.identity();

    println!(
        "{}",
        identity.product.as_deref().unwrap_or("(unknown product)")
    );
    println!(
        "  manufacturer: {}",
        identity.manufacturer.as_deref().unwrap_or("(unknown)")
    );
    println!(
        "  VID:PID: {:#06x}:{:#06x}",
        identity.vendor_id, identity.product_id
    );
    println!("  interfaces: {}", identity.interfaces.len());
    println!("  capabilities:");
    println!("    rgb: {}", opened.rgb().is_some());
    println!("    battery: {}", opened.battery().is_some());
    println!("    profiles: {}", opened.profiles().is_some());

    // Drop the device (and its Transport) explicitly before exiting —
    // see rgb.rs's `run` for why (ADR 0004, LibusbTransport's Drop
    // re-attaching the kernel HID driver).
    drop(opened);
    std::process::exit(device::EXIT_OK);
}

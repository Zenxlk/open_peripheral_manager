//! `pmctl list` — enumerates peripherals recognized by any registered
//! `opm-core` driver. See `docs/architecture/discovery.md`'s
//! "Relationship to `list` and `info`": this is `discover` filtered to
//! `driver status == supported`, without the raw interface detail.

use super::{device, registry};

/// Runs the `list` subcommand.
pub fn run() {
    let registry = registry::build();
    let supported = device::supported_devices(&registry);

    if supported.is_empty() {
        println!(
            "No supported peripherals detected. Run `pmctl discover` to see every HID device."
        );
        std::process::exit(device::EXIT_OK);
    }

    for found in &supported {
        let driver = registry
            .find(&found.identity)
            .expect("already filtered to supported devices");
        let identity = &found.identity;
        println!(
            "{} — {:#06x}:{:#06x} — {}",
            identity.product.as_deref().unwrap_or("(unknown product)"),
            identity.vendor_id,
            identity.product_id,
            driver.name(),
        );
    }

    std::process::exit(device::EXIT_OK);
}

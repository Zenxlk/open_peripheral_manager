//! `pmctl discover` — enumerates every HID device on the system,
//! grouped and classified, regardless of whether any driver supports it
//! yet. See `docs/architecture/discovery.md`.
//!
//! This first implementation covers the default, human-readable output
//! only. `--export`, `--verbose`, and the documented exit-code table are
//! follow-up work — see `docs/roadmap.md`.

use opm_discovery::Classification;

/// Runs the `discover` subcommand.
pub fn run() {
    let discovered = match opm_discovery::discover() {
        Ok(devices) => devices,
        Err(err) => {
            eprintln!("failed to initialize the HID backend: {err}");
            std::process::exit(1);
        }
    };

    if discovered.is_empty() {
        println!("No HID devices detected.");
        return;
    }

    for device in &discovered {
        let identity = &device.identity;
        let name = identity.product.as_deref().unwrap_or("(unknown product)");
        let manufacturer = identity
            .manufacturer
            .as_deref()
            .unwrap_or("(unknown manufacturer)");

        println!(
            "{manufacturer} {name} — {:#06x}:{:#06x} — {}",
            identity.vendor_id,
            identity.product_id,
            classification_label(device.classification),
        );

        for interface in &identity.interfaces {
            let accessible = if opm_discovery::accessible::is_accessible(&interface.path) {
                "ok"
            } else {
                "permission denied"
            };
            let usages: Vec<String> = interface
                .usage_pairs
                .iter()
                .map(|p| format!("{:#06x}/{:#06x}", p.usage_page, p.usage))
                .collect();
            let report_ids = if interface.report_ids.is_empty() {
                "no report IDs".to_owned()
            } else {
                format!("report IDs {:?}", interface.report_ids)
            };
            println!(
                "  interface {}: {} [{}] ({report_ids}) ({accessible})",
                interface.interface_number,
                interface.path,
                usages.join(", "),
            );
        }
    }
}

fn classification_label(classification: Classification) -> &'static str {
    match classification {
        Classification::ConfigurableKeyboard => "Configurable Keyboard",
        Classification::Keyboard => "Keyboard",
        Classification::VendorOnly => "Vendor-only",
        Classification::UnknownHid => "Unknown HID",
    }
}

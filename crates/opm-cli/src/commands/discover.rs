//! `pmctl discover` — enumerates every HID device on the system,
//! grouped and classified, regardless of whether any driver supports it
//! yet. See `docs/architecture/discovery.md`.

mod report;

use opm_discovery::{Classification, Discovered};

/// Exit code for a normal run, including devices found but reported
/// inaccessible — that's per-device data, not a process failure.
const EXIT_OK: i32 = 0;
/// Exit code for the HID backend itself failing to initialize.
const EXIT_BACKEND_FAILED: i32 = 1;
// Invalid invocation (bad flags/arguments) is exit code 2, handled by
// `clap` itself — not something this module needs to produce.

/// `pmctl discover`'s arguments.
#[derive(Debug, clap::Args)]
pub struct Args {
    /// Show per-interface detail: usage pairs, report IDs, path,
    /// accessibility.
    #[arg(short, long)]
    verbose: bool,

    /// Write a portable discovery report instead of printing a table.
    #[arg(long, value_name = "FORMAT")]
    export: Option<ExportFormat>,

    /// Where to write the report (default: an auto-named file in the
    /// current directory). Use "-" for stdout.
    #[arg(long, value_name = "PATH", requires = "export")]
    output: Option<String>,

    /// Include real serial numbers in the report. Redacted by default —
    /// see `discovery.md`'s privacy default; a report is meant to be
    /// shareable.
    #[arg(long, requires = "export")]
    include_serial: bool,
}

/// The formats `--export` supports. Only `json` today — `clap` rejects
/// anything else itself (exit code 2), so this module never has to.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum ExportFormat {
    /// A single JSON report — see `docs/architecture/discovery.md`'s
    /// draft schema.
    Json,
}

/// Runs the `discover` subcommand.
pub fn run(args: Args) {
    let discovered = match opm_discovery::discover() {
        Ok(devices) => devices,
        Err(err) => {
            eprintln!("failed to initialize the HID backend: {err}");
            std::process::exit(EXIT_BACKEND_FAILED);
        }
    };

    match args.export {
        Some(ExportFormat::Json) => {
            export_json(&discovered, args.output.as_deref(), args.include_serial)
        }
        None => print_table(&discovered, args.verbose),
    }

    std::process::exit(EXIT_OK);
}

fn print_table(discovered: &[Discovered], verbose: bool) {
    if discovered.is_empty() {
        println!("No HID devices detected.");
        return;
    }

    for device in discovered {
        let identity = &device.identity;
        let name = identity.product.as_deref().unwrap_or("(unknown product)");
        let manufacturer = identity
            .manufacturer
            .as_deref()
            .unwrap_or("(unknown manufacturer)");
        let accessible = identity
            .interfaces
            .iter()
            .all(|interface| opm_discovery::accessible::is_accessible(&interface.path));

        println!(
            "{manufacturer} {name} — {:#06x}:{:#06x} — {} interface(s) — {} — unsupported — {}",
            identity.vendor_id,
            identity.product_id,
            identity.interfaces.len(),
            classification_label(device.classification),
            if accessible {
                "ok"
            } else {
                "permission denied"
            },
        );

        if verbose {
            for interface in &identity.interfaces {
                let interface_accessible =
                    if opm_discovery::accessible::is_accessible(&interface.path) {
                        "ok"
                    } else {
                        "permission denied"
                    };
                let usages: Vec<String> = interface
                    .usage_pairs
                    .iter()
                    .map(|pair| format!("{:#06x}/{:#06x}", pair.usage_page, pair.usage))
                    .collect();
                let report_ids = if interface.report_ids.is_empty() {
                    "no report IDs".to_owned()
                } else {
                    format!("report IDs {:?}", interface.report_ids)
                };
                println!(
                    "  interface {}: {} [{}] ({report_ids}) ({interface_accessible})",
                    interface.interface_number,
                    interface.path,
                    usages.join(", "),
                );
            }
        }
    }
}

fn export_json(discovered: &[Discovered], output: Option<&str>, include_serial: bool) {
    let report = report::build(discovered, include_serial);
    let json =
        serde_json::to_string_pretty(&report).expect("serializing the report should never fail");

    if output == Some("-") {
        println!("{json}");
        return;
    }

    let path = output
        .map(str::to_owned)
        .unwrap_or_else(|| format!("opm-discover-report-{}.json", report::today_utc_date()));

    if let Err(err) = std::fs::write(&path, &json) {
        eprintln!("failed to write report to {path}: {err}");
        std::process::exit(EXIT_BACKEND_FAILED);
    }
    eprintln!("wrote {path}");
}

fn classification_label(classification: Classification) -> &'static str {
    match classification {
        Classification::ConfigurableKeyboard => "Configurable Keyboard",
        Classification::Keyboard => "Keyboard",
        Classification::VendorOnly => "Vendor-only",
        Classification::UnknownHid => "Unknown HID",
    }
}

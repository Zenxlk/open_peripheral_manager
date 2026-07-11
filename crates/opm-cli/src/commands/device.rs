//! Shared device discovery/selection logic for every subcommand that
//! targets one specific peripheral (`info`, `rgb`, `profile`) or lists
//! every supported one (`list`).

use opm_core::device::Device;
use opm_core::registry::DriverRegistry;
use opm_discovery::Discovered;

/// Exit code for a normal run.
pub const EXIT_OK: i32 = 0;
/// Exit code for a failure talking to hardware: the HID backend itself
/// failing to initialize, opening a device, or a capability call
/// failing.
pub const EXIT_FAILED: i32 = 1;
/// Exit code for a usage problem this command can't proceed without
/// more information from the caller (ambiguous or missing device
/// selection) — the same category `discover`'s exit-code table reserves
/// for `clap`-level usage errors, extended here since disambiguating a
/// device isn't something `clap` itself can validate.
pub const EXIT_USAGE: i32 = 2;

/// Enumerates every HID device and returns only the ones some
/// registered driver recognizes. Exits the process on a backend
/// failure, matching `discover`'s own exit-code table.
pub fn supported_devices(registry: &DriverRegistry) -> Vec<Discovered> {
    let discovered = match opm_discovery::discover() {
        Ok(devices) => devices,
        Err(err) => {
            eprintln!("failed to initialize the HID backend: {err}");
            std::process::exit(EXIT_FAILED);
        }
    };

    discovered
        .into_iter()
        .filter(|found| registry.find(&found.identity).is_some())
        .collect()
}

/// Picks one device out of `supported`, either by an explicit
/// `vid:pid` selector or, if there's exactly one, automatically. Exits
/// the process with [`EXIT_USAGE`] if the selection is ambiguous,
/// matches nothing, or nothing is supported at all.
pub fn select_device<'a>(supported: &'a [Discovered], selector: Option<&str>) -> &'a Discovered {
    if let Some(selector) = selector {
        let (vendor_id, product_id) = parse_vid_pid(selector).unwrap_or_else(|err| {
            eprintln!("invalid --device {selector:?}: {err}");
            std::process::exit(EXIT_USAGE);
        });

        return supported
            .iter()
            .find(|found| {
                found.identity.vendor_id == vendor_id && found.identity.product_id == product_id
            })
            .unwrap_or_else(|| {
                eprintln!("no supported device matches {selector}");
                std::process::exit(EXIT_USAGE);
            });
    }

    match supported {
        [] => {
            eprintln!(
                "no supported peripherals detected. Run `pmctl discover` to see every HID device."
            );
            std::process::exit(EXIT_USAGE);
        }
        [only] => only,
        many => {
            eprintln!("more than one supported device detected — pass --device VID:PID:");
            for found in many {
                eprintln!(
                    "  {:#06x}:{:#06x} — {}",
                    found.identity.vendor_id,
                    found.identity.product_id,
                    found.identity.product.as_deref().unwrap_or("(unknown)")
                );
            }
            std::process::exit(EXIT_USAGE);
        }
    }
}

fn parse_vid_pid(s: &str) -> Result<(u16, u16), String> {
    let (vid, pid) = s
        .split_once(':')
        .ok_or_else(|| "expected VID:PID, e.g. 0c45:800a".to_owned())?;
    let vendor_id = u16::from_str_radix(vid.trim_start_matches("0x"), 16)
        .map_err(|err| format!("bad vendor id {vid:?}: {err}"))?;
    let product_id = u16::from_str_radix(pid.trim_start_matches("0x"), 16)
        .map_err(|err| format!("bad product id {pid:?}: {err}"))?;
    Ok((vendor_id, product_id))
}

/// Opens `found`'s identity via `registry`, exiting the process with
/// [`EXIT_FAILED`] on failure.
pub fn open_or_exit(registry: &DriverRegistry, found: &Discovered) -> Box<dyn Device> {
    registry.open(&found.identity).unwrap_or_else(|err| {
        eprintln!("failed to open device: {err}");
        std::process::exit(EXIT_FAILED);
    })
}

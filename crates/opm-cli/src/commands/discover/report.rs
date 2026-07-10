//! The `--export` report schema (draft) from `docs/architecture/discovery.md`.

use serde::Serialize;

use opm_discovery::Discovered;

/// The full report written by `--export`.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Schema version — additive changes bump the minor number;
    /// consumers should ignore fields they don't recognize.
    pub schema_version: &'static str,
    /// The OPM version that produced this report.
    pub opm_version: &'static str,
    /// Context about the machine the report was captured on.
    pub host: Host,
    /// Every device found.
    pub devices: Vec<DeviceReport>,
}

/// Host context — without this, a report can't be interpreted (e.g.
/// whether `usage_page`/`usage` should even be populated depends on the
/// `hidapi` backend in use).
#[derive(Debug, Serialize)]
pub struct Host {
    /// `"linux"`, `"macos"`, `"windows"`, ...
    pub os: String,
    /// Best-effort distro name/version (Linux: `/etc/os-release`'s
    /// `PRETTY_NAME`).
    pub os_version: String,
    /// Kernel version (Linux: `/proc/sys/kernel/osrelease`).
    pub kernel_version: String,
    /// CPU architecture, e.g. `"x86_64"`.
    pub arch: String,
    /// The `hidapi` backend compiled in — pinned at build time, see
    /// `opm-discovery`'s `Cargo.toml`.
    pub hidapi_backend: String,
}

/// One physical device in the report.
#[derive(Debug, Serialize)]
pub struct DeviceReport {
    /// Report-local identifier — stable only within this file.
    pub local_id: usize,
    /// USB vendor id, as `"0x____"`.
    pub vendor_id: String,
    /// USB product id, as `"0x____"`.
    pub product_id: String,
    /// The device's own manufacturer string, if any.
    pub manufacturer_string: Option<String>,
    /// The device's own product string, if any.
    pub product_string: Option<String>,
    /// Redacted to `None` unless `--include-serial` was passed — see
    /// `discovery.md`'s privacy default.
    pub serial_number: Option<String>,
    /// The classification heuristic's verdict, as a stable identifier.
    pub classification: &'static str,
    /// The OPM version whose heuristic produced `classification` — so a
    /// later heuristic change doesn't make an old report look more
    /// authoritative than it is.
    pub classified_by: &'static str,
    /// Always `"unsupported"` today: no `Driver` exists yet to match
    /// against. Will reflect `opm-core`'s `DriverRegistry` once Phase 3
    /// lands.
    pub driver_status: &'static str,
    /// Every HID interface belonging to this device.
    pub interfaces: Vec<InterfaceReport>,
}

/// One HID interface in the report.
#[derive(Debug, Serialize)]
pub struct InterfaceReport {
    /// The USB interface number, or `-1` where the OS doesn't expose one.
    pub interface_number: i32,
    /// The OS-specific path used to open this interface.
    pub path: String,
    /// Whether this process can currently open the path (zero-I/O
    /// open-then-close check, not just a permission-bits guess).
    pub accessible: bool,
    /// Every top-level usage pair this interface declares.
    pub usage_pairs: Vec<UsagePairReport>,
    /// The distinct report IDs this interface's report descriptor
    /// declares.
    pub report_ids: Vec<u8>,
}

/// One usage pair in the report.
#[derive(Debug, Serialize)]
pub struct UsagePairReport {
    /// The HID usage page, as `"0x____"`.
    pub usage_page: String,
    /// The usage within that page, as `"0x____"`.
    pub usage: String,
}

fn hex4(value: u16) -> String {
    format!("{value:#06x}")
}

/// Builds the full report from a `discover()` result.
pub fn build(discovered: &[Discovered], include_serial: bool) -> Report {
    let opm_version = env!("CARGO_PKG_VERSION");

    let devices = discovered
        .iter()
        .enumerate()
        .map(|(local_id, device)| {
            let identity = &device.identity;
            DeviceReport {
                local_id,
                vendor_id: hex4(identity.vendor_id),
                product_id: hex4(identity.product_id),
                manufacturer_string: identity.manufacturer.clone(),
                product_string: identity.product.clone(),
                serial_number: if include_serial {
                    identity.serial.clone()
                } else {
                    None
                },
                classification: device.classification.as_str(),
                classified_by: opm_version,
                driver_status: "unsupported",
                interfaces: identity
                    .interfaces
                    .iter()
                    .map(|interface| InterfaceReport {
                        interface_number: interface.interface_number,
                        accessible: opm_discovery::accessible::is_accessible(&interface.path),
                        path: interface.path.clone(),
                        usage_pairs: interface
                            .usage_pairs
                            .iter()
                            .map(|pair| UsagePairReport {
                                usage_page: hex4(pair.usage_page),
                                usage: hex4(pair.usage),
                            })
                            .collect(),
                        report_ids: interface.report_ids.clone(),
                    })
                    .collect(),
            }
        })
        .collect();

    Report {
        schema_version: "1.0",
        opm_version,
        host: host_info(),
        devices,
    }
}

fn host_info() -> Host {
    Host {
        os: std::env::consts::OS.to_owned(),
        os_version: os_version(),
        kernel_version: kernel_version(),
        arch: std::env::consts::ARCH.to_owned(),
        // Pinned in opm-discovery/Cargo.toml; update this if that pin
        // ever changes.
        hidapi_backend: "linux-static-hidraw".to_owned(),
    }
}

fn os_version() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|contents| {
            contents.lines().find_map(|line| {
                line.strip_prefix("PRETTY_NAME=")
                    .map(|value| value.trim_matches('"').to_owned())
            })
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

fn kernel_version() -> String {
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|contents| contents.trim().to_owned())
        .unwrap_or_else(|_| "unknown".to_owned())
}

/// Today's UTC date as `YYYY-MM-DD`, for the default `--export` output
/// filename. No date/time crate needed for something this small — see
/// [`civil_from_days`].
pub fn today_utc_date() -> String {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() / 86_400)
        .unwrap_or(0) as i64;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Days-since-1970-01-01 (UTC) to a (year, month, day) civil date.
/// Howard Hinnant's well-known, widely-used `civil_from_days` algorithm
/// — see <http://howardhinnant.github.io/date_algorithms.html>.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let year = if month <= 2 { y + 1 } else { y };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::civil_from_days;

    #[test]
    fn known_epoch_days_map_to_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(11_017), (2000, 3, 1));
        assert_eq!(civil_from_days(20_643), (2026, 7, 9));
    }
}

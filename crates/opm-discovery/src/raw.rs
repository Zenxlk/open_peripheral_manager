//! The thin adapter over `hidapi`. Everything past this module is pure
//! logic over [`RawEntry`] — see `docs/architecture/discovery.md`'s
//! "Testing strategy".

use crate::error::Error;

/// One raw `hidapi` enumeration entry. Mirrors `hidapi::DeviceInfo`
/// field-for-field, decoupled from it so the rest of this crate can be
/// tested without ever calling into `hidapi`.
///
/// Since libhidapi 0.13, `hidapi` yields one entry per top-level usage
/// pair, not per interface — a single interface with multiple top-level
/// collections produces multiple entries sharing the same `path` and
/// `interface_number`. See [`crate::group::dedupe_by_path`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEntry {
    /// USB vendor id.
    pub vendor_id: u16,
    /// USB product id.
    pub product_id: u16,
    /// The device's serial number string, if any.
    pub serial_number: Option<String>,
    /// The device's manufacturer string, if any.
    pub manufacturer_string: Option<String>,
    /// The device's product string, if any.
    pub product_string: Option<String>,
    /// This entry's top-level HID usage page.
    pub usage_page: u16,
    /// This entry's top-level HID usage.
    pub usage: u16,
    /// The USB interface number, or `-1` where the OS doesn't expose one.
    pub interface_number: i32,
    /// The OS-specific path used to open this interface later.
    pub path: String,
}

/// Enumerates every HID interface currently visible to the OS.
///
/// Read-only: never opens a device, never blocks waiting for hardware.
/// Fails only if the backend itself can't be initialized — an
/// individual device's fields being unavailable is not an error at this
/// layer.
pub fn enumerate() -> Result<Vec<RawEntry>, Error> {
    let api = hidapi::HidApi::new()?;
    Ok(api
        .device_list()
        .map(|d| RawEntry {
            vendor_id: d.vendor_id(),
            product_id: d.product_id(),
            serial_number: d.serial_number().map(str::to_owned),
            manufacturer_string: d.manufacturer_string().map(str::to_owned),
            product_string: d.product_string().map(str::to_owned),
            usage_page: d.usage_page(),
            usage: d.usage(),
            interface_number: d.interface_number(),
            path: d.path().to_string_lossy().into_owned(),
        })
        .collect())
}

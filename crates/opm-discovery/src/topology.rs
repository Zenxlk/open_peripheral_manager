//! Resolving which physical USB device a `hidraw` interface belongs to.
//!
//! This is the one piece of OS-topology I/O the grouping algorithm needs
//! (see `docs/architecture/discovery.md`'s "How to determine what HID
//! interfaces a device has?"); deliberately isolated in its own small,
//! impure function so [`crate::group`] can stay pure and testable.

use std::path::{Path, PathBuf};

use crate::error::Error;

/// Resolves the sysfs path of the physical USB device that owns the
/// given `hidraw` path (e.g. `/dev/hidraw0`).
///
/// Two interfaces on the same physical composite USB device resolve to
/// the same path here, regardless of whether they share a serial number
/// — confirmed against a real device, see `discovery.md`'s Findings.
///
/// Linux-only for now; other platforms are explicitly deferred (see
/// `discovery.md`'s risks).
pub fn usb_device_path(hidraw_path: &str) -> Result<PathBuf, Error> {
    let name = Path::new(hidraw_path)
        .file_name()
        .ok_or_else(|| Error::Sysfs {
            path: hidraw_path.to_owned(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a hidraw path"),
        })?;

    let device_link = Path::new("/sys/class/hidraw").join(name).join("device");
    let device = std::fs::canonicalize(&device_link).map_err(|source| Error::Sysfs {
        path: device_link.display().to_string(),
        source,
    })?;

    // `device` is the HID device node; its parent is the USB interface,
    // and that interface's parent is the physical USB device shared by
    // every interface of the same composite device.
    device
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| Error::Sysfs {
            path: device.display().to_string(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "hidraw device has no grandparent in sysfs",
            ),
        })
}

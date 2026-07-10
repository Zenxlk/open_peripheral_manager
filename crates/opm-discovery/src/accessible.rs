//! The accessibility check from `docs/architecture/discovery.md`: is
//! this interface actually openable, not just enumerable?
//!
//! A `stat`/`access(2)`-only check would misreport devices made
//! accessible via a udev-installed ACL (common, and not visible in the
//! mode bits alone) — so this opens the path read-only and immediately
//! closes it without issuing any read or write. The one narrow,
//! explicitly-documented exception to "discovery never opens a device".

use std::fs::OpenOptions;

/// Whether `path` (e.g. `/dev/hidraw0`) can currently be opened
/// read-only by this process. Transfers zero bytes and holds the handle
/// no longer than the check itself.
pub fn is_accessible(path: &str) -> bool {
    OpenOptions::new().read(true).open(path).is_ok()
}

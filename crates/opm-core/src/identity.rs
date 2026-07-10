//! The `Identity` facet: everything OPM can learn about a HID device
//! without opening it. See `docs/architecture/discovery.md` (design) and
//! `docs/architecture/domain-model.md` (vocabulary).
//!
//! This module is plain, serializable data only. It never touches
//! `hidapi` or any transport library — see ADR 0002. Producing an
//! `Identity` is the `opm-discovery` crate's job.

use serde::{Deserialize, Serialize};

/// A HID usage page/usage pair, as declared by a device's report
/// descriptor (or surfaced directly by `hidapi`'s enumeration).
///
/// See the USB HID Usage Tables specification for the meaning of any
/// given pair; OPM only ever compares these numerically (e.g. `0x01`
/// Generic Desktop / `0x06` Keyboard), never interprets them further.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UsagePair {
    /// The HID usage page (e.g. `0x01` for Generic Desktop, `0xFF00`-
    /// `0xFFFF` for vendor-defined).
    pub usage_page: u16,
    /// The usage within that page (e.g. `0x06` for Keyboard).
    pub usage: u16,
}

/// One HID interface belonging to a device, after collapsing every
/// `hidapi` enumeration entry that shares the same OS path (see
/// `docs/architecture/discovery.md`'s "How to determine what HID
/// interfaces a device has?").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interface {
    /// The USB interface number, or `-1` where the OS doesn't expose one.
    pub interface_number: i32,
    /// The OS-specific handle used to open this interface later (e.g.
    /// `/dev/hidraw0` on Linux). Not guaranteed stable across reboots.
    pub path: String,
    /// Every top-level usage pair this interface declares. More than one
    /// entry means the interface has multiple top-level collections —
    /// confirmed to happen in practice, see `discovery.md`'s Findings.
    pub usage_pairs: Vec<UsagePair>,
    /// The distinct report IDs this interface's report descriptor
    /// declares (input, output, and feature reports combined). Empty
    /// means the interface uses a single, implicit report with no ID
    /// byte.
    pub report_ids: Vec<u8>,
}

/// Everything OPM knows about a detected-but-unopened HID device: the
/// `Identity` facet from `docs/architecture/domain-model.md`. Produced
/// by `opm-discovery`, consumed by a `Driver`'s future `probe()` method.
///
/// Deliberately **not** the `Device` from `driver-model.md` — an
/// `Identity` is inert data; a `Device` is a live handle to something a
/// driver has already opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    /// USB vendor id.
    pub vendor_id: u16,
    /// USB product id.
    pub product_id: u16,
    /// The device's own manufacturer string, if it reports one non-empty.
    /// Often resolves to an OEM controller vendor, not the brand printed
    /// on the device — see `discovery.md`'s manufacturer-identification
    /// findings.
    pub manufacturer: Option<String>,
    /// The device's own product string, if it reports one non-empty.
    pub product: Option<String>,
    /// The device's serial number, if it reports one both present and
    /// non-empty. Deliberately **not** used as a device-grouping key —
    /// see `discovery.md`'s grouping algorithm — budget controllers are
    /// known to report an empty or hardcoded-identical serial across
    /// every unit.
    pub serial: Option<String>,
    /// Every HID interface grouped into this physical device.
    pub interfaces: Vec<Interface>,
}

//! HID device enumeration, grouping, and classification — the crate
//! behind `pmctl discover`. Works with zero drivers registered; see
//! `docs/architecture/discovery.md` for the design and
//! `docs/architecture/decisions/0002-discovery-lives-outside-opm-core.md`
//! for why this lives outside `opm-core`.
//!
//! Discovery only enumerates. It never opens a device for sustained I/O,
//! never sends a report, and never reads/writes device state. The one
//! narrow exception is [`accessible::is_accessible`]'s zero-I/O
//! open-then-close check.

pub mod accessible;
pub mod classify;
pub mod descriptor;
pub mod error;
pub mod group;
pub mod raw;
pub mod topology;

use std::collections::HashMap;
use std::path::PathBuf;

pub use classify::Classification;
pub use error::Error;
pub use opm_core::identity::Identity;

/// One physical device found by [`discover`], already grouped and
/// classified.
#[derive(Debug, Clone)]
pub struct Discovered {
    /// Everything known about the device without opening it.
    pub identity: Identity,
    /// The category the classification heuristic placed it in.
    pub classification: Classification,
}

/// Enumerates every HID device currently visible to the OS, groups their
/// interfaces into physical devices, and classifies each one.
///
/// Fails only if the HID backend itself can't be initialized. Per-
/// interface problems (topology resolution failing, a report descriptor
/// being unreadable) degrade gracefully rather than aborting the whole
/// scan — an unusual interface shouldn't hide every other device.
pub fn discover() -> Result<Vec<Discovered>, Error> {
    let entries = raw::enumerate()?;
    let interfaces = group::dedupe_by_path(&entries);

    let with_topology: Vec<_> = interfaces
        .into_iter()
        .map(|interface| {
            // Falling back to a per-interface key (rather than failing
            // outright) means an interface whose topology can't be
            // resolved is reported as its own device instead of being
            // silently dropped or merged incorrectly.
            let key = topology::usb_device_path(&interface.path)
                .unwrap_or_else(|_| PathBuf::from(&interface.path));
            (interface, key)
        })
        .collect();

    let report_ids_by_path: HashMap<String, Vec<u8>> = with_topology
        .iter()
        .map(|(interface, _)| {
            let ids = descriptor::report_ids(&interface.path).unwrap_or_default();
            (interface.path.clone(), ids)
        })
        .collect();

    let groups = group::group_by_topology(with_topology);

    Ok(groups
        .into_iter()
        .map(|group| {
            let identity = group::build_identity(group, &report_ids_by_path);
            let classification = classify::classify(&identity);
            Discovered {
                identity,
                classification,
            }
        })
        .collect())
}

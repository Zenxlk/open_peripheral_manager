//! Pure grouping logic: raw `hidapi` entries → one [`opm_core::identity::Identity`]
//! per physical device. See `docs/architecture/discovery.md`'s "How to
//! determine what HID interfaces a device has?" for the design this
//! implements.

use std::collections::HashMap;
use std::path::PathBuf;

use opm_core::identity::{Identity, Interface, UsagePair};

use crate::raw::RawEntry;

/// One HID interface after collapsing every raw entry that shares a
/// `path` — undoes `hidapi`'s one-entry-per-top-level-usage-pair
/// enumeration (confirmed in practice, see `discovery.md`'s Findings).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceEntry {
    /// USB vendor id (same for every entry sharing this `path`).
    pub vendor_id: u16,
    /// USB product id (same for every entry sharing this `path`).
    pub product_id: u16,
    /// Serial number, normalized: an empty string is treated the same
    /// as absent, since it carries no grouping or identifying value —
    /// confirmed to occur in practice (`Some("")` on the AK820).
    pub serial_number: Option<String>,
    /// Manufacturer string, if any.
    pub manufacturer_string: Option<String>,
    /// Product string, if any.
    pub product_string: Option<String>,
    /// The USB interface number.
    pub interface_number: i32,
    /// The OS-specific path used to open this interface later.
    pub path: String,
    /// Every top-level usage pair declared on this interface.
    pub usage_pairs: Vec<UsagePair>,
}

/// Collapses raw `hidapi` entries sharing a `path` into one
/// [`InterfaceEntry`] each, preserving first-seen order.
pub fn dedupe_by_path(entries: &[RawEntry]) -> Vec<InterfaceEntry> {
    let mut order: Vec<String> = Vec::new();
    let mut by_path: HashMap<String, InterfaceEntry> = HashMap::new();

    for entry in entries {
        by_path
            .entry(entry.path.clone())
            .and_modify(|existing| {
                existing.usage_pairs.push(UsagePair {
                    usage_page: entry.usage_page,
                    usage: entry.usage,
                });
            })
            .or_insert_with(|| {
                order.push(entry.path.clone());
                InterfaceEntry {
                    vendor_id: entry.vendor_id,
                    product_id: entry.product_id,
                    serial_number: entry.serial_number.clone().filter(|s| !s.is_empty()),
                    manufacturer_string: entry.manufacturer_string.clone(),
                    product_string: entry.product_string.clone(),
                    interface_number: entry.interface_number,
                    path: entry.path.clone(),
                    usage_pairs: vec![UsagePair {
                        usage_page: entry.usage_page,
                        usage: entry.usage,
                    }],
                }
            });
    }

    order
        .into_iter()
        .map(|path| by_path.remove(&path).expect("path was just inserted"))
        .collect()
}

/// Groups already-deduped interfaces into physical devices by their
/// resolved OS topology key — **not** by serial number. See
/// `discovery.md` for why: budget controllers are known to report an
/// empty or hardcoded-identical serial across every unit, which would
/// silently merge distinct physical devices if used for grouping.
pub fn group_by_topology(interfaces: Vec<(InterfaceEntry, PathBuf)>) -> Vec<Vec<InterfaceEntry>> {
    let mut order: Vec<PathBuf> = Vec::new();
    let mut by_key: HashMap<PathBuf, Vec<InterfaceEntry>> = HashMap::new();

    for (interface, key) in interfaces {
        by_key
            .entry(key.clone())
            .or_insert_with(|| {
                order.push(key.clone());
                Vec::new()
            })
            .push(interface);
    }

    order
        .into_iter()
        .map(|key| by_key.remove(&key).expect("key was just inserted"))
        .collect()
}

/// Builds an [`Identity`] from one physical device's grouped interfaces.
///
/// `report_ids_by_path` supplies each interface's declared report IDs
/// (from `docs/architecture/discovery.md`'s report-descriptor layer,
/// via [`crate::descriptor::report_ids`]) — fetched separately since
/// that requires I/O, keeping this function itself pure.
pub fn build_identity(
    group: Vec<InterfaceEntry>,
    report_ids_by_path: &HashMap<String, Vec<u8>>,
) -> Identity {
    let first = group.first().expect("a group is never empty");
    let vendor_id = first.vendor_id;
    let product_id = first.product_id;
    let manufacturer = first.manufacturer_string.clone();
    let product = first.product_string.clone();
    let serial = first.serial_number.clone();

    let interfaces = group
        .into_iter()
        .map(|interface| Interface {
            interface_number: interface.interface_number,
            report_ids: report_ids_by_path
                .get(&interface.path)
                .cloned()
                .unwrap_or_default(),
            path: interface.path,
            usage_pairs: interface.usage_pairs,
        })
        .collect();

    Identity {
        vendor_id,
        product_id,
        manufacturer,
        product,
        serial,
        interfaces,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real `hidapi` enumeration entries captured from the maintainer's
    /// Ajazz AK820 (see
    /// `docs/inventory/captures/ajazz-ak820-2026-07-09.json` and
    /// `docs/architecture/discovery.md`'s Findings) — interface 1 alone
    /// declaring five top-level usage pairs is exactly the real-world
    /// case that forced `hidapi`'s one-entry-per-usage-pair behavior to
    /// be handled here, not assumed away.
    fn ak820_entries() -> Vec<RawEntry> {
        let mk = |usage_page: u16, usage: u16, interface_number: i32, path: &str| RawEntry {
            vendor_id: 0x0c45,
            product_id: 0x800a,
            serial_number: Some(String::new()),
            manufacturer_string: Some("SONiX".to_owned()),
            product_string: Some("AK820".to_owned()),
            usage_page,
            usage,
            interface_number,
            path: path.to_owned(),
        };

        vec![
            mk(0x0001, 0x0006, 0, "/dev/hidraw0"),
            mk(0x000c, 0x0001, 1, "/dev/hidraw1"),
            mk(0x0001, 0x0080, 1, "/dev/hidraw1"),
            mk(0x0001, 0x0006, 1, "/dev/hidraw1"),
            mk(0x0001, 0x0002, 1, "/dev/hidraw1"),
            mk(0xffff, 0x0001, 1, "/dev/hidraw1"),
            mk(0xff68, 0x0061, 2, "/dev/hidraw2"),
            mk(0xff13, 0x0001, 3, "/dev/hidraw3"),
        ]
    }

    #[test]
    fn dedupe_collapses_shared_path_into_one_interface_with_all_usage_pairs() {
        let interfaces = dedupe_by_path(&ak820_entries());

        assert_eq!(interfaces.len(), 4, "4 distinct paths, not 8 raw entries");

        let interface1 = interfaces
            .iter()
            .find(|i| i.path == "/dev/hidraw1")
            .expect("interface 1 present");
        assert_eq!(interface1.usage_pairs.len(), 5);
        assert_eq!(
            interface1.serial_number, None,
            "empty serial normalized to None"
        );
    }

    #[test]
    fn group_by_topology_merges_interfaces_sharing_a_parent_and_keeps_others_separate() {
        let interfaces = dedupe_by_path(&ak820_entries());
        let shared_parent: PathBuf = "/sys/devices/pci0000:00/0000:00:14.0/usb3/3-1".into();

        let with_keys: Vec<_> = interfaces
            .into_iter()
            .map(|i| (i, shared_parent.clone()))
            .collect();

        let groups = group_by_topology(with_keys);

        assert_eq!(groups.len(), 1, "all 4 AK820 interfaces share one parent");
        assert_eq!(groups[0].len(), 4);
    }

    #[test]
    fn group_by_topology_keeps_different_parents_separate() {
        let interfaces = dedupe_by_path(&ak820_entries());
        let ak820_parent: PathBuf = "/sys/devices/.../usb3/3-1".into();
        let other_device_parent: PathBuf = "/sys/devices/.../usb3/3-2".into();

        let with_keys: Vec<_> = interfaces
            .into_iter()
            .enumerate()
            .map(|(i, interface)| {
                let key = if i == 0 {
                    other_device_parent.clone()
                } else {
                    ak820_parent.clone()
                };
                (interface, key)
            })
            .collect();

        let groups = group_by_topology(with_keys);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn build_identity_uses_first_interfaces_shared_fields() {
        let interfaces = dedupe_by_path(&ak820_entries());
        let identity = build_identity(interfaces, &HashMap::new());

        assert_eq!(identity.vendor_id, 0x0c45);
        assert_eq!(identity.product_id, 0x800a);
        assert_eq!(identity.manufacturer.as_deref(), Some("SONiX"));
        assert_eq!(identity.product.as_deref(), Some("AK820"));
        assert_eq!(identity.serial, None);
        assert_eq!(identity.interfaces.len(), 4);
    }
}

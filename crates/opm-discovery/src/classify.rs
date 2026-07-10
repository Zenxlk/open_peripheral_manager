//! The classification heuristic from `docs/architecture/discovery.md`.
//! A heuristic, not a proof: it says "worth investigating", not
//! "definitely has RGB/macros".

use opm_core::identity::Identity;

const GENERIC_DESKTOP: u16 = 0x0001;
const KEYBOARD: u16 = 0x0006;
const CONSUMER: u16 = 0x000c;
const VENDOR_RANGE_START: u16 = 0xff00;

/// The category a device's declared usage pairs place it in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    /// Declares a standard keyboard usage pair and at least one
    /// vendor-defined usage pair — a candidate for a new driver.
    ConfigurableKeyboard,
    /// Declares a standard keyboard usage pair and nothing vendor-defined.
    Keyboard,
    /// Declares a vendor-defined usage pair but no standard keyboard one.
    VendorOnly,
    /// Matches none of the above signals this heuristic currently knows.
    UnknownHid,
}

/// Classifies a device by walking every usage pair declared across all
/// of its interfaces — regardless of whether those pairs came from
/// separate interfaces or from multiple top-level collections on the
/// same one (confirmed to happen in practice, see `discovery.md`'s
/// Findings).
pub fn classify(identity: &Identity) -> Classification {
    let mut usage_pairs = identity
        .interfaces
        .iter()
        .flat_map(|interface| interface.usage_pairs.iter());

    let is_keyboard = usage_pairs
        .clone()
        .any(|p| p.usage_page == GENERIC_DESKTOP && p.usage == KEYBOARD);
    let is_vendor = usage_pairs.any(|p| p.usage_page >= VENDOR_RANGE_START);

    match (is_keyboard, is_vendor) {
        (true, true) => Classification::ConfigurableKeyboard,
        (true, false) => Classification::Keyboard,
        (false, true) => Classification::VendorOnly,
        (false, false) => Classification::UnknownHid,
    }
}

/// Whether a device also declares a consumer-control usage pair
/// (`0x0C`/media keys). Present on almost any modern keyboard, standard
/// or not — informational only, never part of [`classify`]'s decision.
pub fn has_consumer_control(identity: &Identity) -> bool {
    identity
        .interfaces
        .iter()
        .flat_map(|interface| interface.usage_pairs.iter())
        .any(|p| p.usage_page == CONSUMER)
}

#[cfg(test)]
mod tests {
    use super::*;
    use opm_core::identity::{Interface, UsagePair};

    fn identity_with_pairs(pairs: &[(u16, u16)]) -> Identity {
        Identity {
            vendor_id: 0x0c45,
            product_id: 0x800a,
            manufacturer: None,
            product: None,
            serial: None,
            interfaces: vec![Interface {
                interface_number: 0,
                path: "/dev/hidraw0".to_owned(),
                usage_pairs: pairs
                    .iter()
                    .map(|&(usage_page, usage)| UsagePair { usage_page, usage })
                    .collect(),
                report_ids: vec![],
            }],
        }
    }

    #[test]
    fn keyboard_plus_vendor_usage_is_configurable_keyboard() {
        let identity = identity_with_pairs(&[(0x0001, 0x0006), (0xffff, 0x0001)]);
        assert_eq!(classify(&identity), Classification::ConfigurableKeyboard);
    }

    #[test]
    fn keyboard_alone_is_keyboard() {
        let identity = identity_with_pairs(&[(0x0001, 0x0006)]);
        assert_eq!(classify(&identity), Classification::Keyboard);
    }

    #[test]
    fn vendor_alone_is_vendor_only() {
        let identity = identity_with_pairs(&[(0xff13, 0x0001)]);
        assert_eq!(classify(&identity), Classification::VendorOnly);
    }

    #[test]
    fn mouse_usage_alone_is_unknown_hid() {
        let identity = identity_with_pairs(&[(0x0001, 0x0002)]);
        assert_eq!(classify(&identity), Classification::UnknownHid);
    }

    #[test]
    fn vendor_signal_split_across_separate_interfaces_still_counts() {
        // The AK820's real shape: the keyboard usage on one interface,
        // vendor usage pairs on two entirely separate ones.
        let identity = Identity {
            vendor_id: 0x0c45,
            product_id: 0x800a,
            manufacturer: None,
            product: None,
            serial: None,
            interfaces: vec![
                Interface {
                    interface_number: 0,
                    path: "/dev/hidraw0".to_owned(),
                    usage_pairs: vec![UsagePair {
                        usage_page: 0x0001,
                        usage: 0x0006,
                    }],
                    report_ids: vec![],
                },
                Interface {
                    interface_number: 3,
                    path: "/dev/hidraw3".to_owned(),
                    usage_pairs: vec![UsagePair {
                        usage_page: 0xff13,
                        usage: 0x0001,
                    }],
                    report_ids: vec![],
                },
            ],
        };
        assert_eq!(classify(&identity), Classification::ConfigurableKeyboard);
    }

    #[test]
    fn consumer_control_is_detected_but_not_part_of_classification() {
        let identity = identity_with_pairs(&[(0x0001, 0x0006), (0x000c, 0x0001)]);
        assert_eq!(classify(&identity), Classification::Keyboard);
        assert!(has_consumer_control(&identity));
    }
}

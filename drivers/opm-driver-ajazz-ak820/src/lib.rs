//! Driver for the Ajazz AK820 (Pro), the first real implementation of
//! `opm-core`'s `Driver`/`Device`/`Capability` traits (Phase 4 — see
//! `docs/roadmap.md`).
//!
//! `Capability` implementations are deliberately stubs: they return
//! [`opm_core::error::Error::Driver`] rather than pretend to succeed,
//! since nothing about the AK820's actual protocol is known yet — that's
//! Phase 6, `docs/protocols/ajazz-ak820/`. This crate's job is only to
//! prove discovery → transport → driver → capability plumbing works
//! end-to-end against the real device.

use opm_core::capability::{Profiles, Rgb, RgbColor};
use opm_core::device::Device;
use opm_core::driver::Driver;
use opm_core::error::Error;
use opm_core::identity::{Identity, UsagePair};
use opm_core::transport::Transport;

/// USB vendor id. `0x0c45` is Sonix Technology, the OEM controller
/// vendor, not Ajazz itself — see
/// `docs/protocols/ajazz-ak820/README.md`'s "Hardware identity". A
/// known, not-yet-observed risk: a different Sonix-based keyboard
/// rebrand could share this exact VID:PID, which `probe` alone can't
/// tell apart.
const VENDOR_ID: u16 = 0x0c45;
/// USB product id, as captured against the real AK820 Pro.
const PRODUCT_ID: u16 = 0x800a;

/// The vendor usage pair this driver opens: the one confirmed, in
/// `docs/architecture/transport.md`'s 2026-07-10 Findings, to actually
/// answer a real `get_feature` request on the real AK820 Pro (interface
/// 3, `/dev/hidraw4`). The device's other two vendor channels
/// (interface 1's usage pair shared with keyboard/consumer/mouse usages,
/// and interface 2's `0xff68/0x61`) are unexplored — Phase 6 may need
/// them too; this driver only opens the one already proven reachable.
const VENDOR_USAGE_PAIR: UsagePair = UsagePair {
    usage_page: 0xff13,
    usage: 0x01,
};

/// The Ajazz AK820 (Pro) driver. Stateless, per
/// `docs/architecture/driver-model.md`.
#[derive(Debug, Default)]
pub struct AjazzAk820Driver;

impl Driver for AjazzAk820Driver {
    fn name(&self) -> &str {
        "Ajazz AK820"
    }

    fn probe(&self, identity: &Identity) -> bool {
        identity.vendor_id == VENDOR_ID && identity.product_id == PRODUCT_ID
    }

    fn open(&self, identity: &Identity) -> Result<Box<dyn Device>, Error> {
        let vendor_path = vendor_interface_path(identity).ok_or_else(|| {
            Error::Driver(
                "AK820: no interface declares the expected vendor usage pair (0xff13/0x01)"
                    .to_owned(),
            )
        })?;

        let vendor = opm_transport::HidTransport::open(vendor_path)?;

        Ok(Box::new(AjazzAk820Device {
            identity: identity.clone(),
            vendor: Box::new(vendor),
        }))
    }
}

fn vendor_interface_path(identity: &Identity) -> Option<&str> {
    identity
        .interfaces
        .iter()
        .find(|iface| iface.usage_pairs.contains(&VENDOR_USAGE_PAIR))
        .map(|iface| iface.path.as_str())
}

/// A live handle to an opened AK820.
struct AjazzAk820Device {
    identity: Identity,
    /// The vendor configuration channel (see [`VENDOR_USAGE_PAIR`]).
    /// Held open for the device's lifetime so future, real `Capability`
    /// implementations (Phase 6) have something to read/write — nothing
    /// in this crate uses it yet, see the module docs.
    vendor: Box<dyn Transport>,
}

impl Device for AjazzAk820Device {
    fn identity(&self) -> &Identity {
        &self.identity
    }

    fn rgb(&self) -> Option<&dyn Rgb> {
        Some(self)
    }

    fn profiles(&self) -> Option<&dyn Profiles> {
        Some(self)
    }
}

/// A single message shared by every capability stub below: the real
/// behavior needs Phase 6's protocol reverse-engineering first.
fn not_yet_implemented(capability: &str) -> Error {
    Error::Driver(format!(
        "AK820: {capability} not yet implemented — see docs/protocols/ajazz-ak820/ (Phase 6, not started)"
    ))
}

impl Rgb for AjazzAk820Device {
    fn get_color(&self) -> Result<RgbColor, Error> {
        // Proves the vendor transport is genuinely alive on every call,
        // using the exact read-only get_feature(0, ..) exchange already
        // validated safe against real hardware (transport.md's
        // 2026-07-10 Findings) — the 64-byte buffer is the same
        // arbitrary size used there, not the interface's real report
        // length (transport.md flags this as a known gap). A transport
        // failure (unplugged, permission revoked) surfaces as-is; a
        // successful read still can't be decoded into a color yet.
        let mut buf = [0u8; 64];
        self.vendor.get_feature(0, &mut buf)?;
        Err(not_yet_implemented("Rgb::get_color (protocol unknown)"))
    }

    fn set_color(&self, color: RgbColor) -> Result<(), Error> {
        // Byte layouts reverse-engineered from real USB captures against
        // the vendor's official software — see
        // docs/protocols/ajazz-ak820/findings.md.
        //
        // A plain `set_color` write alone doesn't light up the keyboard:
        // the vendor software only applies a color while its "Static"
        // effect mode is active, selected via a separate command. Send
        // that "activate Static mode" command first (opcode 0x02,
        // substituting the target color for the default red the real
        // capture carried — unconfirmed the firmware reads it at all),
        // then the color-update command (opcode 0x01) every real capture
        // agreed on.
        let mut activate_static = [0u8; 64];
        activate_static[0] = 0x02;
        activate_static[1] = color.r;
        activate_static[2] = color.g;
        activate_static[3] = color.b;
        activate_static[9] = 0x01;
        activate_static[10] = 0x05;
        activate_static[11] = 0x03;
        activate_static[15] = 0xaa;
        activate_static[16] = 0x55;
        self.vendor.set_feature(0, &activate_static)?;

        let mut set_color = [0u8; 64];
        set_color[0] = 0x01;
        set_color[1] = color.r;
        set_color[2] = color.g;
        set_color[3] = color.b;
        set_color[9] = 0x05;
        set_color[10] = 0x03;
        set_color[14] = 0xaa;
        set_color[15] = 0x55;
        self.vendor.set_feature(0, &set_color)?;

        Ok(())
    }
}

impl Profiles for AjazzAk820Device {
    fn active_profile(&self) -> Result<u8, Error> {
        Err(not_yet_implemented("Profiles::active_profile"))
    }

    fn set_active_profile(&self, _profile: u8) -> Result<(), Error> {
        Err(not_yet_implemented("Profiles::set_active_profile"))
    }
}

#[cfg(test)]
mod tests {
    use opm_core::identity::Interface;

    use super::*;

    fn identity_with_interfaces(interfaces: Vec<Interface>) -> Identity {
        Identity {
            vendor_id: VENDOR_ID,
            product_id: PRODUCT_ID,
            manufacturer: Some("SONiX".to_owned()),
            product: Some("AK820".to_owned()),
            serial: None,
            interfaces,
        }
    }

    fn interface(usage_pairs: Vec<UsagePair>) -> Interface {
        Interface {
            interface_number: 0,
            path: "/dev/hidraw4".to_owned(),
            usage_pairs,
            report_ids: Vec::new(),
        }
    }

    // Real AK820 Pro shape, per docs/protocols/ajazz-ak820/README.md.
    fn real_ak820_identity() -> Identity {
        identity_with_interfaces(vec![
            interface(vec![UsagePair {
                usage_page: 0x01,
                usage: 0x06,
            }]),
            interface(vec![
                UsagePair {
                    usage_page: 0x0c,
                    usage: 0x01,
                },
                UsagePair {
                    usage_page: 0xffff,
                    usage: 0x01,
                },
            ]),
            interface(vec![UsagePair {
                usage_page: 0xff68,
                usage: 0x61,
            }]),
            interface(vec![VENDOR_USAGE_PAIR]),
        ])
    }

    #[test]
    fn probe_matches_the_real_ak820_vid_pid() {
        let driver = AjazzAk820Driver;
        assert!(driver.probe(&real_ak820_identity()));
    }

    #[test]
    fn probe_rejects_a_different_product_id() {
        let driver = AjazzAk820Driver;
        let mut other = real_ak820_identity();
        other.product_id = 0x0000;
        assert!(!driver.probe(&other));
    }

    #[test]
    fn probe_rejects_a_different_vendor_id() {
        let driver = AjazzAk820Driver;
        let mut other = real_ak820_identity();
        other.vendor_id = 0x0000;
        assert!(!driver.probe(&other));
    }

    #[test]
    fn vendor_interface_path_finds_the_dedicated_vendor_interface() {
        let identity = real_ak820_identity();
        assert_eq!(vendor_interface_path(&identity), Some("/dev/hidraw4"));
    }

    #[test]
    fn open_fails_cleanly_when_the_vendor_interface_is_missing() {
        // No interface declares VENDOR_USAGE_PAIR — open() must fail
        // before ever touching opm-transport/hidapi, not panic.
        let driver = AjazzAk820Driver;
        let identity = identity_with_interfaces(vec![interface(vec![UsagePair {
            usage_page: 0x01,
            usage: 0x06,
        }])]);

        let err = match driver.open(&identity) {
            Err(err) => err,
            Ok(_) => panic!("expected an error"),
        };
        assert!(matches!(err, Error::Driver(_)));
    }

    type SetFeatureCall = (u8, Vec<u8>);

    #[derive(Clone, Default)]
    struct FakeTransport {
        set_feature_calls: std::sync::Arc<std::sync::Mutex<Vec<SetFeatureCall>>>,
    }

    impl Transport for FakeTransport {
        fn write_output(
            &self,
            _report_id: u8,
            _data: &[u8],
        ) -> Result<usize, opm_core::transport::Error> {
            unimplemented!("not exercised by this test")
        }

        fn read_input(
            &self,
            _buf: &mut [u8],
            _timeout: opm_core::transport::ReadTimeout,
        ) -> Result<(u8, usize), opm_core::transport::Error> {
            unimplemented!("not exercised by this test")
        }

        fn get_feature(
            &self,
            _report_id: u8,
            _buf: &mut [u8],
        ) -> Result<usize, opm_core::transport::Error> {
            unimplemented!("not exercised by this test")
        }

        fn set_feature(
            &self,
            report_id: u8,
            data: &[u8],
        ) -> Result<(), opm_core::transport::Error> {
            self.set_feature_calls
                .lock()
                .unwrap()
                .push((report_id, data.to_vec()));
            Ok(())
        }
    }

    #[test]
    fn set_color_activates_static_mode_then_writes_the_color() {
        let transport = FakeTransport::default();
        let calls = transport.set_feature_calls.clone();
        let device = AjazzAk820Device {
            identity: real_ak820_identity(),
            vendor: Box::new(transport),
        };

        device
            .set_color(RgbColor {
                r: 0xfe,
                g: 0xb9,
                b: 0x73,
            })
            .expect("set_color should succeed against a fake transport");

        let calls = calls.lock().unwrap();
        assert_eq!(calls.len(), 2);

        let (report_id, activate_static) = &calls[0];
        assert_eq!(*report_id, 0);
        let mut expected_activate = [0u8; 64];
        expected_activate[0] = 0x02;
        expected_activate[1] = 0xfe;
        expected_activate[2] = 0xb9;
        expected_activate[3] = 0x73;
        expected_activate[9] = 0x01;
        expected_activate[10] = 0x05;
        expected_activate[11] = 0x03;
        expected_activate[15] = 0xaa;
        expected_activate[16] = 0x55;
        assert_eq!(activate_static.as_slice(), &expected_activate[..]);

        let (report_id, set_color) = &calls[1];
        assert_eq!(*report_id, 0);
        let mut expected_set_color = [0u8; 64];
        expected_set_color[0] = 0x01;
        expected_set_color[1] = 0xfe;
        expected_set_color[2] = 0xb9;
        expected_set_color[3] = 0x73;
        expected_set_color[9] = 0x05;
        expected_set_color[10] = 0x03;
        expected_set_color[14] = 0xaa;
        expected_set_color[15] = 0x55;
        assert_eq!(set_color.as_slice(), &expected_set_color[..]);
    }
}

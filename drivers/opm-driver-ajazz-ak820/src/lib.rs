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
use opm_core::identity::{Identity, Interface, UsagePair};
use opm_core::transport::Transport;

/// The AK820's lighting-mode vocabulary and packet layout — public so
/// other code (a future GUI, another capability implementation) can
/// build on the full mode/direction/sleep-timer catalog this crate
/// carries even though only `Static` is wired into a `Capability` so
/// far. See the module's own docs for credits.
pub mod protocol;
use protocol::{CONTROL_REPORT_ID, Direction, LightingMode, control_packet, mode_data_packet};

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
        let vendor_iface = vendor_interface(identity).ok_or_else(|| {
            Error::Driver(
                "AK820: no interface declares the expected vendor usage pair (0xff13/0x01)"
                    .to_owned(),
            )
        })?;

        let interface_number = u8::try_from(vendor_iface.interface_number).map_err(|_| {
            Error::Driver(format!(
                "AK820: vendor interface has no usable USB interface number ({})",
                vendor_iface.interface_number
            ))
        })?;

        // LibusbTransport, not HidTransport — see ADR 0004 and
        // docs/protocols/ajazz-ak820/findings.md's 2026-07-20 "root
        // cause" entry: this device's Feature-report writes are
        // silently swallowed by Linux's hid-generic kernel driver over
        // hidraw, and only work with the kernel driver detached.
        let vendor = opm_transport::LibusbTransport::open(
            identity.vendor_id,
            identity.product_id,
            interface_number,
        )?;

        Ok(Box::new(AjazzAk820Device {
            identity: identity.clone(),
            vendor: Box::new(vendor),
        }))
    }
}

fn vendor_interface(identity: &Identity) -> Option<&Interface> {
    identity
        .interfaces
        .iter()
        .find(|iface| iface.usage_pairs.contains(&VENDOR_USAGE_PAIR))
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

/// How long to wait after each packet for the device to process it —
/// matching gohv/EPOMAKER-Ajazz-AK820-Pro's `send_feature`, whose
/// comment calls this "small delay for device to process". Our own
/// first libusb attempt sent every packet back-to-back with no delay
/// at all and had no visible effect on real hardware; this is untested
/// as its own variable but is the leading suspect — see findings.md.
const INTER_PACKET_DELAY: std::time::Duration = std::time::Duration::from_millis(10);

/// How long to wait after a full `START`/`START_MODE`/data/`FINISH`
/// transaction completes, matching gohv's `transaction`'s "extra delay
/// after full transaction for device processing".
const POST_TRANSACTION_DELAY: std::time::Duration = std::time::Duration::from_millis(100);

/// Sends one 64-byte packet's `Transport::set_feature` call (byte 0 as
/// `report_id`, the rest as `data`), then — only for `CONTROL_REPORT_ID`
/// packets — a `get_feature` handshake read, matching
/// gohv/EPOMAKER-Ajazz-AK820-Pro's finding that reading back the
/// lighting-data packet's report ID (a mode value, not `0x04`) makes
/// the device unresponsive. Sleeps `INTER_PACKET_DELAY` afterward either
/// way, also matching gohv.
fn send(transport: &dyn Transport, packet: [u8; 64]) -> Result<(), Error> {
    let report_id = packet[0];
    transport.set_feature(report_id, &packet[1..])?;
    if report_id == CONTROL_REPORT_ID {
        let mut discard = [0u8; 64];
        let _ = transport.get_feature(report_id, &mut discard);
    }
    std::thread::sleep(INTER_PACKET_DELAY);
    Ok(())
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
        // successful read still can't be decoded into a color yet — the
        // background GET_REPORT polling traffic doesn't obviously
        // mirror the active color, see findings.md.
        let mut buf = [0u8; 64];
        self.vendor.get_feature(0, &mut buf)?;
        Err(not_yet_implemented("Rgb::get_color (protocol unknown)"))
    }

    fn set_color(&self, color: RgbColor) -> Result<(), Error> {
        // Requesting LightingMode::Static (0x01) directly does not
        // work on this hardware — an empirically-found substitution,
        // not understood, see findings.md. Breath with speed = 0
        // produces a static-looking color instead.
        send(&*self.vendor, control_packet(protocol::CMD_START, 0x01))?;
        send(&*self.vendor, control_packet(protocol::CMD_MODE, 0x01))?;
        send(
            &*self.vendor,
            mode_data_packet(
                LightingMode::Breath,
                color.r,
                color.g,
                color.b,
                false,
                protocol::MAX_BRIGHTNESS,
                0,
                Direction::Left,
            ),
        )?;
        send(&*self.vendor, control_packet(protocol::CMD_FINISH, 0x01))?;
        std::thread::sleep(POST_TRANSACTION_DELAY);
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

    fn interface(interface_number: i32, usage_pairs: Vec<UsagePair>) -> Interface {
        Interface {
            interface_number,
            path: format!("/dev/hidraw{interface_number}"),
            usage_pairs,
            report_ids: Vec::new(),
        }
    }

    // Real AK820 Pro shape, per docs/protocols/ajazz-ak820/README.md.
    fn real_ak820_identity() -> Identity {
        identity_with_interfaces(vec![
            interface(
                0,
                vec![UsagePair {
                    usage_page: 0x01,
                    usage: 0x06,
                }],
            ),
            interface(
                1,
                vec![
                    UsagePair {
                        usage_page: 0x0c,
                        usage: 0x01,
                    },
                    UsagePair {
                        usage_page: 0xffff,
                        usage: 0x01,
                    },
                ],
            ),
            interface(
                2,
                vec![UsagePair {
                    usage_page: 0xff68,
                    usage: 0x61,
                }],
            ),
            interface(3, vec![VENDOR_USAGE_PAIR]),
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
    fn vendor_interface_finds_the_dedicated_vendor_interface() {
        let identity = real_ak820_identity();
        assert_eq!(
            vendor_interface(&identity).map(|iface| iface.interface_number),
            Some(3)
        );
    }

    #[test]
    fn open_fails_cleanly_when_the_vendor_interface_is_missing() {
        // No interface declares VENDOR_USAGE_PAIR — open() must fail
        // before ever touching opm-transport/rusb, not panic.
        let driver = AjazzAk820Driver;
        let identity = identity_with_interfaces(vec![interface(
            0,
            vec![UsagePair {
                usage_page: 0x01,
                usage: 0x06,
            }],
        )]);

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
        get_feature_report_ids: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
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
            report_id: u8,
            _buf: &mut [u8],
        ) -> Result<usize, opm_core::transport::Error> {
            self.get_feature_report_ids.lock().unwrap().push(report_id);
            Ok(0)
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
    fn set_color_sends_the_start_mode_data_finish_sequence() {
        let transport = FakeTransport::default();
        let set_feature_calls = transport.set_feature_calls.clone();
        let get_feature_report_ids = transport.get_feature_report_ids.clone();
        let device = AjazzAk820Device {
            identity: real_ak820_identity(),
            vendor: Box::new(transport),
        };
        let color = RgbColor {
            r: 0xfe,
            g: 0xb9,
            b: 0x73,
        };

        device
            .set_color(color)
            .expect("set_color should succeed against a fake transport");

        let expected_packets = [
            control_packet(protocol::CMD_START, 0x01),
            control_packet(protocol::CMD_MODE, 0x01),
            mode_data_packet(
                LightingMode::Breath,
                color.r,
                color.g,
                color.b,
                false,
                protocol::MAX_BRIGHTNESS,
                0,
                Direction::Left,
            ),
            control_packet(protocol::CMD_FINISH, 0x01),
        ];

        let calls = set_feature_calls.lock().unwrap();
        assert_eq!(calls.len(), expected_packets.len());
        for (call, packet) in calls.iter().zip(expected_packets.iter()) {
            assert_eq!(call.0, packet[0]);
            assert_eq!(call.1.as_slice(), &packet[1..]);
        }

        // Only the three CONTROL_REPORT_ID packets (START/START_MODE/
        // FINISH) get a GET_REPORT handshake — the data packet's report
        // ID is the mode value, which findings.md documents as making
        // the device unresponsive if queried.
        assert_eq!(
            get_feature_report_ids.lock().unwrap().as_slice(),
            &[CONTROL_REPORT_ID, CONTROL_REPORT_ID, CONTROL_REPORT_ID]
        );
    }
}

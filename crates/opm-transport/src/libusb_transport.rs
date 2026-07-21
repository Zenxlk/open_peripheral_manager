//! `libusb`-backed implementation of `opm-core`'s [`Transport`] trait,
//! for devices whose Feature-report writes are silently swallowed by
//! Linux's kernel HID driver when reached through `hidraw` — see ADR
//! 0004 and `docs/protocols/ajazz-ak820/findings.md`'s 2026-07-20
//! "root cause" entry. [`super::HidTransport`] remains the default
//! choice for devices without this quirk; this exists specifically for
//! ones that need the kernel driver detached to accept writes at all.

use std::time::Duration;

use opm_core::transport::{Error, ReadTimeout, Transport};

const CONTROL_TIMEOUT: Duration = Duration::from_millis(1000);

const HID_GET_REPORT: u8 = 0x01;
const HID_SET_REPORT: u8 = 0x09;
const HID_REPORT_TYPE_FEATURE: u16 = 0x0300;

/// An open `libusb` handle to exactly one HID interface, with the
/// kernel driver detached from it for the duration.
///
/// Opens by `(vendor_id, product_id, interface_number)` rather than an
/// OS path — `libusb` addresses devices/interfaces differently than
/// `HidTransport`'s `hidraw` paths; see ADR 0004. If more than one
/// device shares the same `vendor_id`/`product_id`, the first one
/// `libusb` enumerates is used — the same "which physical unit"
/// ambiguity already flagged elsewhere in this project (e.g.
/// `opm-cli`'s `select_device`), not solved here either.
pub struct LibusbTransport {
    handle: rusb::DeviceHandle<rusb::GlobalContext>,
    interface_number: u8,
    kernel_driver_was_active: bool,
}

impl LibusbTransport {
    /// Opens `interface_number` on the first USB device matching
    /// `vendor_id`/`product_id`, detaching the kernel driver from that
    /// interface first if one is attached.
    pub fn open(vendor_id: u16, product_id: u16, interface_number: u8) -> Result<Self, Error> {
        let open_err = |reason: String| Error::Open {
            path: format!("usb:{vendor_id:04x}:{product_id:04x}/if{interface_number}"),
            reason,
        };

        let device = rusb::devices()
            .map_err(|source| open_err(source.to_string()))?
            .iter()
            .find(|d| {
                d.device_descriptor()
                    .map(|desc| desc.vendor_id() == vendor_id && desc.product_id() == product_id)
                    .unwrap_or(false)
            })
            .ok_or_else(|| open_err("no matching USB device found".to_owned()))?;

        let handle = device
            .open()
            .map_err(|source| open_err(source.to_string()))?;

        let kernel_driver_was_active = handle
            .kernel_driver_active(interface_number)
            .unwrap_or(false);
        if kernel_driver_was_active {
            handle
                .detach_kernel_driver(interface_number)
                .map_err(|source| open_err(source.to_string()))?;
        }

        handle
            .claim_interface(interface_number)
            .map_err(|source| open_err(source.to_string()))?;

        Ok(Self {
            handle,
            interface_number,
            kernel_driver_was_active,
        })
    }

    /// Finds this interface's first interrupt endpoint in the given
    /// direction, for [`Transport::write_output`]/[`Transport::read_input`].
    fn interrupt_endpoint(&self, direction: rusb::Direction) -> Result<u8, Error> {
        let config = self
            .handle
            .device()
            .active_config_descriptor()
            .map_err(|source| io_err(source.to_string()))?;

        config
            .interfaces()
            .flat_map(|iface| iface.descriptors())
            .find(|desc| desc.interface_number() == self.interface_number)
            .and_then(|desc| {
                desc.endpoint_descriptors()
                    .find(|ep| {
                        ep.direction() == direction
                            && ep.transfer_type() == rusb::TransferType::Interrupt
                    })
                    .map(|ep| ep.address())
            })
            .ok_or_else(|| {
                io_err(format!(
                    "interface {} has no {direction:?} interrupt endpoint",
                    self.interface_number
                ))
            })
    }
}

impl Transport for LibusbTransport {
    fn write_output(&self, report_id: u8, data: &[u8]) -> Result<usize, Error> {
        let endpoint = self.interrupt_endpoint(rusb::Direction::Out)?;
        let mut buf = Vec::with_capacity(data.len() + 1);
        buf.push(report_id);
        buf.extend_from_slice(data);

        let written = self
            .handle
            .write_interrupt(endpoint, &buf, CONTROL_TIMEOUT)
            .map_err(|source| io_err(source.to_string()))?;
        Ok(written.saturating_sub(1))
    }

    fn read_input(&self, buf: &mut [u8], timeout: ReadTimeout) -> Result<(u8, usize), Error> {
        let endpoint = self.interrupt_endpoint(rusb::Direction::In)?;
        let timeout = match timeout {
            ReadTimeout::NonBlocking => Duration::from_millis(1),
            ReadTimeout::Millis(millis) => Duration::from_millis(u64::from(millis)),
            ReadTimeout::Blocking => Duration::from_millis(5_000),
        };

        let mut raw = vec![0u8; buf.len() + 1];
        let read = self
            .handle
            .read_interrupt(endpoint, &mut raw, timeout)
            .map_err(|source| match source {
                rusb::Error::Timeout => Error::WouldBlock,
                other => io_err(other.to_string()),
            })?;

        if read == 0 {
            return Err(Error::WouldBlock);
        }

        let report_id = raw[0];
        let payload_len = read - 1;
        buf[..payload_len].copy_from_slice(&raw[1..read]);
        Ok((report_id, payload_len))
    }

    fn get_feature(&self, report_id: u8, buf: &mut [u8]) -> Result<usize, Error> {
        let request_type = rusb::request_type(
            rusb::Direction::In,
            rusb::RequestType::Class,
            rusb::Recipient::Interface,
        );
        let value = HID_REPORT_TYPE_FEATURE | u16::from(report_id);

        // Unlike HidTransport's hidraw ioctls (which always synthesize a
        // leading Report ID byte in the buffer regardless of the
        // device's own framing), a raw libusb control transfer's DATA
        // stage is exactly what the device sends — no extra byte to
        // strip. Requesting one byte more than needed (as an earlier
        // version of this method did) sent the wrong wLength to a
        // device already known to be picky about this exact exchange
        // (see docs/protocols/ajazz-ak820/findings.md); read exactly
        // `buf.len()` bytes instead.
        let read = self
            .handle
            .read_control(
                request_type,
                HID_GET_REPORT,
                value,
                u16::from(self.interface_number),
                buf,
                CONTROL_TIMEOUT,
            )
            .map_err(|source| io_err(source.to_string()))?;

        Ok(read)
    }

    fn set_feature(&self, report_id: u8, data: &[u8]) -> Result<(), Error> {
        let request_type = rusb::request_type(
            rusb::Direction::Out,
            rusb::RequestType::Class,
            rusb::Recipient::Interface,
        );
        let value = HID_REPORT_TYPE_FEATURE | u16::from(report_id);

        let mut buf = Vec::with_capacity(data.len() + 1);
        buf.push(report_id);
        buf.extend_from_slice(data);

        self.handle
            .write_control(
                request_type,
                HID_SET_REPORT,
                value,
                u16::from(self.interface_number),
                &buf,
                CONTROL_TIMEOUT,
            )
            .map(|_| ())
            .map_err(|source| io_err(source.to_string()))
    }
}

impl Drop for LibusbTransport {
    fn drop(&mut self) {
        let _ = self.handle.release_interface(self.interface_number);
        if self.kernel_driver_was_active {
            let _ = self.handle.attach_kernel_driver(self.interface_number);
        }
    }
}

fn io_err(reason: String) -> Error {
    Error::Io { reason }
}

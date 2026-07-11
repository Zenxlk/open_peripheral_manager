//! `hidapi`-backed implementation of `opm-core`'s [`Transport`] trait.
//! See `docs/architecture/transport.md` (design) and
//! `docs/architecture/decisions/0003-transport-trait-in-core-impl-in-opm-transport.md`
//! (why this crate exists separately from `opm-core`).
//!
//! Depends on `opm-core` and `hidapi` internally; `opm-core` never
//! depends back on this crate. Does not depend on `opm-discovery`, and
//! vice versa — see `transport.md` for why the two crates deliberately
//! don't share code despite both wrapping `hidapi`.

use std::ffi::CString;

use opm_core::transport::{Error, ReadTimeout, Transport};

/// How long [`HidTransport::read_input`]'s [`ReadTimeout::Blocking`]
/// waits, in milliseconds, before giving up — see `transport.md`'s
/// "Risks": deliberately finite, not a literal indefinite wait, so a
/// stalled device can't hang the caller forever. Not yet informed by any
/// real device's observed response latency; revisit once one exists.
const BLOCKING_TIMEOUT_MILLIS: i32 = 5_000;

/// An open `hidapi` handle to exactly one HID interface.
///
/// Opens one `Identity::Interface::path` (e.g. `/dev/hidraw3` on Linux)
/// — a physical device with multiple relevant interfaces needs one
/// `HidTransport` per interface, see `transport.md`.
pub struct HidTransport {
    device: hidapi::HidDevice,
}

impl HidTransport {
    /// Opens the HID interface at `path`.
    ///
    /// Read-write; fails if the path doesn't exist, is already
    /// exclusively held, or isn't accessible to the current user (no
    /// `plugdev`-style group membership or udev rule — the same
    /// permission gap `pmctl discover` reports per-device rather than
    /// failing on, see `docs/architecture/discovery.md`).
    pub fn open(path: &str) -> Result<Self, Error> {
        let open_err = |reason: String| Error::Open {
            path: path.to_owned(),
            reason,
        };

        let api = hidapi::HidApi::new().map_err(|source| open_err(source.to_string()))?;
        let c_path = CString::new(path).map_err(|source| open_err(source.to_string()))?;
        let device = api
            .open_path(&c_path)
            .map_err(|source| open_err(source.to_string()))?;

        Ok(Self { device })
    }
}

impl Transport for HidTransport {
    fn write_output(&self, report_id: u8, data: &[u8]) -> Result<usize, Error> {
        let mut buf = Vec::with_capacity(data.len() + 1);
        buf.push(report_id);
        buf.extend_from_slice(data);

        let written = self
            .device
            .write(&buf)
            .map_err(|source| io_err(source.to_string()))?;
        // `hidapi::write`'s returned count includes the Report ID byte;
        // `Transport`'s contract is payload bytes only.
        Ok(written.saturating_sub(1))
    }

    fn read_input(&self, buf: &mut [u8], timeout: ReadTimeout) -> Result<(u8, usize), Error> {
        let timeout_millis = match timeout {
            ReadTimeout::NonBlocking => 0,
            // Report IDs are `u8`, so a `u32` millisecond count can
            // exceed `i32::MAX` only in a value nobody would sanely
            // pass; saturate rather than panic on the conversion.
            ReadTimeout::Millis(ms) => i32::try_from(ms).unwrap_or(i32::MAX),
            ReadTimeout::Blocking => BLOCKING_TIMEOUT_MILLIS,
        };

        // hidapi always tags a read with the Report ID as the first
        // byte, even for devices without numbered reports (`0` in that
        // case) — one extra byte of headroom vs. the caller's buffer.
        let mut raw = vec![0u8; buf.len() + 1];
        let read = self
            .device
            .read_timeout(&mut raw, timeout_millis)
            .map_err(|source| io_err(source.to_string()))?;

        if read == 0 {
            return Err(Error::WouldBlock);
        }

        let report_id = raw[0];
        let payload_len = read - 1;
        buf[..payload_len].copy_from_slice(&raw[1..read]);
        Ok((report_id, payload_len))
    }

    fn get_feature(&self, report_id: u8, buf: &mut [u8]) -> Result<usize, Error> {
        let mut raw = vec![0u8; buf.len() + 1];
        raw[0] = report_id;

        let read = self
            .device
            .get_feature_report(&mut raw)
            .map_err(|source| io_err(source.to_string()))?;

        // `get_feature_report`'s returned count includes the Report ID
        // byte still sitting in `raw[0]`; the payload starts at `raw[1]`.
        let payload_len = read.saturating_sub(1);
        buf[..payload_len].copy_from_slice(&raw[1..1 + payload_len]);
        Ok(payload_len)
    }

    fn set_feature(&self, report_id: u8, data: &[u8]) -> Result<(), Error> {
        let mut buf = Vec::with_capacity(data.len() + 1);
        buf.push(report_id);
        buf.extend_from_slice(data);

        self.device
            .send_feature_report(&buf)
            .map_err(|source| io_err(source.to_string()))
    }
}

fn io_err(reason: String) -> Error {
    Error::Io { reason }
}

//! The `Transport` facet: opening one HID interface and exchanging
//! Input/Output/Feature reports with it. See
//! `docs/architecture/transport.md` (design) and
//! `docs/architecture/domain-model.md` (vocabulary).
//!
//! This module is trait/signature only. It never touches `hidapi` or any
//! transport library — see ADR 0003
//! (`docs/architecture/decisions/0003-transport-trait-in-core-impl-in-opm-transport.md`).
//! Implementing `Transport` against real hardware is `opm-transport`'s
//! job.

/// How long [`Transport::read_input`] waits for an Input report before
/// giving up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadTimeout {
    /// Return immediately with [`Error::WouldBlock`] if nothing is queued.
    NonBlocking,
    /// Wait up to this many milliseconds.
    Millis(u32),
    /// Wait a long, but finite, default (see the implementation crate for
    /// the exact value) — deliberately not a literal indefinite wait, so
    /// a stalled device can't hang the caller forever. Prefer `Millis`
    /// with a value informed by the device's actual protocol once known.
    Blocking,
}

/// Everything that can go wrong opening or using a [`Transport`].
///
/// Deliberately doesn't wrap `hidapi::HidError` (or any transport
/// library's error type) directly — see ADR 0002/0003 for why `opm-core`
/// can't name that type at all.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Opening the interface itself failed (missing, permission denied,
    /// already gone).
    #[error("failed to open {path}: {reason}")]
    Open {
        /// The interface path that failed to open (e.g. `/dev/hidraw0`).
        path: String,
        /// The backend's own error message, preserved for display but
        /// not pattern-matched on.
        reason: String,
    },
    /// A read/write/feature-report call failed after a successful open
    /// (device unplugged mid-session, a genuine I/O error).
    #[error("HID I/O failed: {reason}")]
    Io {
        /// The backend's own error message.
        reason: String,
    },
    /// A non-blocking or timed-out read found no data available. Not
    /// necessarily an error condition for the caller — polling code is
    /// expected to match on this specifically.
    #[error("no data available within the requested timeout")]
    WouldBlock,
}

/// An open channel to exactly one HID interface, able to exchange
/// Input/Output/Feature reports with it. Says nothing about what any
/// report's bytes *mean* — see `docs/architecture/domain-model.md`'s
/// `Transport`/`Protocol` split.
///
/// Implementations open one `Identity`'s `Interface::path`; a physical
/// device with multiple relevant interfaces (common — see
/// `docs/architecture/discovery.md`'s Findings) needs one `Transport`
/// per interface.
///
/// `open()` is deliberately not part of this trait — a trait object
/// (`Box<dyn Transport>`) can't have a constructor returning `Self`.
/// Each implementation exposes its own inherent constructor instead
/// (e.g. `opm_transport::HidTransport::open`).
pub trait Transport: Send {
    /// Sends an Output report. `report_id` is `0` for interfaces that
    /// don't use numbered reports (see `Identity::Interface::report_ids`).
    /// `data` is the report payload *without* the Report ID byte —
    /// implementations handle that framing internally. Returns the
    /// number of payload bytes written.
    fn write_output(&self, report_id: u8, data: &[u8]) -> Result<usize, Error>;

    /// Reads the next queued Input report into `buf`, waiting per
    /// `timeout`. Unlike Output/Feature reports, an Input report is
    /// pushed by the device whenever it has one, not requested by ID —
    /// there is no way to ask for a *specific* report ID, only to read
    /// whatever comes next. Returns `(report_id, payload_len)`: which
    /// report ID the device tagged this one with (`0` for unnumbered
    /// reports) and how many payload bytes were written to `buf` (the
    /// Report ID byte itself is not included, mirroring `write_output`).
    fn read_input(&self, buf: &mut [u8], timeout: ReadTimeout) -> Result<(u8, usize), Error>;

    /// Requests a Feature report by `report_id` over the control
    /// endpoint. Returns the number of payload bytes written to `buf`
    /// (the Report ID byte itself is not included).
    fn get_feature(&self, report_id: u8, buf: &mut [u8]) -> Result<usize, Error>;

    /// Sends a Feature report over the control endpoint. `data` is the
    /// report payload without the Report ID byte, mirroring
    /// `write_output`.
    fn set_feature(&self, report_id: u8, data: &[u8]) -> Result<(), Error>;
}

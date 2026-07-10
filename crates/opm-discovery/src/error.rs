//! Errors `opm-discovery` can produce.

/// Something went wrong while enumerating or inspecting HID devices.
///
/// Enumeration failing entirely (e.g. no HID subsystem available) is the
/// only case that should ever reach a caller as an error — a single
/// device being unreadable or inaccessible is reported inline as data
/// (see [`crate::Discovered`]), never as an `Err`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The `hidapi` backend itself failed to initialize.
    #[error("failed to initialize the HID backend: {0}")]
    Backend(#[from] hidapi::HidError),
    /// Reading sysfs (a report descriptor, or an interface's parent USB
    /// device, for topology-based grouping) failed.
    #[error("failed to read {path}: {source}")]
    Sysfs {
        /// The sysfs path that failed to read.
        path: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// `hidreport` failed to parse an interface's report descriptor.
    #[error("failed to parse report descriptor: {0}")]
    ParseDescriptor(#[from] hidreport::ParserError),
}

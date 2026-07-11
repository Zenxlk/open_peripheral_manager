//! The top-level `Error` type shared across `Driver`/`Device`/
//! `Capability`. See `docs/architecture/driver-model.md`'s
//! `opm_core::error::Error` section for the design.

/// Everything that can go wrong probing, opening, or using a device
/// through `opm-core`'s traits.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No registered driver's [`crate::driver::Driver::probe`]
    /// recognized this [`crate::identity::Identity`].
    #[error("no registered driver recognizes this device")]
    Unsupported,
    /// A [`crate::transport::Transport`] operation failed while opening
    /// or using a device.
    #[error(transparent)]
    Transport(#[from] crate::transport::Error),
    /// A driver-specific failure not covered by [`Error::Transport`]
    /// (e.g. a response the driver can parse enough to reject).
    #[error("{0}")]
    Driver(String),
}

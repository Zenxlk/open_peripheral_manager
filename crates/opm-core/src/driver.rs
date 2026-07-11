//! The `Driver` trait: the contract each manufacturer/device crate under
//! `drivers/` implements so it can be discovered and used without
//! `opm-core`, `opm-cli`, or the future GUI knowing anything
//! vendor-specific. See `docs/architecture/driver-model.md`'s
//! "`Driver`: stateless, `probe`/`open` split".

use crate::device::Device;
use crate::error::Error;
use crate::identity::Identity;

/// A stateless matcher/factory: given an [`Identity`], decides whether
/// it recognizes the device, and if so, opens it.
///
/// A driver crate implements this on a typically zero-sized unit struct
/// (e.g. `pub struct AjazzAk820Driver;`), not something constructed with
/// configuration.
pub trait Driver: Send + Sync {
    /// A short, human-readable name for logging/CLI display (e.g.
    /// `"Ajazz AK820"`), not necessarily unique.
    fn name(&self) -> &str;

    /// Does this driver recognize the device described by `identity`?
    /// Pure inspection — no I/O, never opens anything. Safe to call
    /// against every registered driver for every discovered `Identity`.
    fn probe(&self, identity: &Identity) -> bool;

    /// Opens the device, returning a live [`Device`]. Only ever called
    /// after [`Driver::probe`] returned `true` for the same `Identity`.
    /// Does real I/O (via a `Transport`, from inside the driver crate —
    /// `opm-core` itself never touches a transport library).
    fn open(&self, identity: &Identity) -> Result<Box<dyn Device>, Error>;
}

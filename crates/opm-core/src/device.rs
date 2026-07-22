//! The `Device` trait: a handle to an already-open, communicating
//! peripheral, independent of vendor and transport. See
//! `docs/architecture/driver-model.md`'s "`Device`: identity plus
//! capability accessors".
//!
//! Returned by [`crate::driver::Driver::open`]. Front-ends interact with
//! a device exclusively through this trait plus whichever optional
//! capability traits ([`crate::capability`]) it chooses to expose.

use crate::capability::{Battery, Lighting, Profiles, Rgb};
use crate::identity::Identity;

/// A live handle to an opened peripheral.
///
/// Every accessor defaults to `None`, so a concrete `Device` only
/// overrides the ones it actually supports — see `driver-model.md` for
/// why this shape was chosen over `dyn Any` downcasting.
pub trait Device: Send {
    /// What this device is — the same [`Identity`] its [`crate::driver::Driver`]
    /// matched.
    fn identity(&self) -> &Identity;

    /// `Some` if this device supports RGB lighting control.
    fn rgb(&self) -> Option<&dyn Rgb> {
        None
    }
    /// `Some` if this device reports a battery level.
    fn battery(&self) -> Option<&dyn Battery> {
        None
    }
    /// `Some` if this device supports switching between stored profiles.
    fn profiles(&self) -> Option<&dyn Profiles> {
        None
    }
    /// `Some` if this device supports animated lighting effects beyond
    /// a single solid color (see [`crate::capability::Lighting`]).
    fn lighting(&self) -> Option<&dyn Lighting> {
        None
    }
}

//! Will hold the `Driver` trait: the contract each manufacturer/device
//! crate under `drivers/` implements so it can be discovered and used
//! without `opm-core`, `opm-cli`, or the future GUI knowing anything
//! vendor-specific.
//!
//! Conceptually a driver is a small factory: given information about a
//! connected peripheral, it decides whether it knows how to talk to it,
//! and if so, produces a [`crate::device::Device`].
//!
//! Not yet designed — start here when you're ready to design the trait.

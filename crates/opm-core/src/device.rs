//! Will hold the `Device` trait: a handle to an already-open, communicating
//! peripheral, independent of vendor and transport.
//!
//! A `Device` is what a driver's `open()` call is expected to hand back to
//! the caller (see [`crate::driver`]). Front-ends such as `opm-cli` will
//! interact with a device exclusively through this trait, plus whichever
//! optional capability traits (see [`crate::capability`]) the concrete
//! device chooses to expose.
//!
//! Not yet designed — start here when you're ready to design the trait.

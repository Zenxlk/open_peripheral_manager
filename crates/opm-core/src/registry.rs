//! Will hold `DriverRegistry`: the in-memory collection of available
//! [`crate::driver::Driver`] implementations that front-ends query to find
//! out which connected peripherals are supported and to open them.
//!
//! Each driver crate under `drivers/` is expected to register itself here
//! (exact mechanism — explicit list, `inventory`-style linkme registration,
//! or something else — is an open design question, see
//! `docs/architecture/driver-model.md`).
//!
//! Not yet designed — start here when you're ready to design the registry.

//! Core abstractions for the Open Peripheral Manager.
//!
//! `opm-core` will define the vendor-agnostic contracts that connect
//! device drivers (implemented in separate crates under `drivers/`) with
//! front-ends such as the CLI (`opm-cli`) and, eventually, a graphical
//! application. Both front-ends are meant to depend on this crate and
//! nothing else to talk to a device.
//!
//! Deliberately **out of scope** for this crate, now and for the
//! foreseeable future:
//! - HID/USB transport and device enumeration (belongs in a future
//!   transport crate or in individual driver crates).
//! - Any vendor- or model-specific protocol logic (belongs in a driver
//!   crate under `drivers/`).
//!
//! The modules below are placeholders. Their real design — the `Device`
//! and `Driver` traits, the driver registry, and the capability-detection
//! pattern used for optional features such as RGB lighting or profile
//! switching — is tracked in `docs/architecture/driver-model.md` and is
//! deliberately left for later, hands-on design work rather than decided
//! upfront here.

pub mod capability;
pub mod device;
pub mod driver;
pub mod error;
pub mod registry;

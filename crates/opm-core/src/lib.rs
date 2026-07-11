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
//! - Actually talking to a transport library (`hidapi`, `libusb`, ...).
//!   [`transport`] defines the `Transport` *trait* only — the
//!   `hidapi`-backed implementation lives in `opm-transport` (see
//!   `docs/architecture/transport.md`, ADR 0003). Device enumeration
//!   similarly lives in `opm-discovery` (ADR 0002).
//! - Any vendor- or model-specific protocol logic (belongs in a driver
//!   crate under `drivers/`).
//!
//! [`driver`]/[`device`]/[`capability`]/[`registry`] hold the `Driver`,
//! `Device`, and capability traits, and `DriverRegistry` — see
//! `docs/architecture/driver-model.md` for the design. No real driver
//! crate exists yet (Phase 4); these traits are validated in
//! [`registry`]'s tests against fake, in-memory devices.

pub mod capability;
pub mod device;
pub mod driver;
pub mod error;
pub mod identity;
pub mod registry;
pub mod transport;

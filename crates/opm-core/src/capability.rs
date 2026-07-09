//! Will hold the pattern used to expose *optional* per-device features —
//! RGB lighting, profile switching, battery level, polling-rate control,
//! and so on — without forcing every [`crate::device::Device`] to
//! implement every possible feature.
//!
//! This is the main extensibility knob for supporting many devices with
//! different feature sets through one shared `Device` trait, so it is
//! worth deliberately designing rather than guessing upfront. Candidate
//! approaches to evaluate (see `docs/architecture/driver-model.md`):
//! downcasting via `std::any::Any`, one accessor method per known
//! capability on `Device`, or an external crate such as `downcast-rs`.
//!
//! Not yet designed — start here when you're ready to design the
//! capability pattern.

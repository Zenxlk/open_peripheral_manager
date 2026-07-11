//! Builds the `DriverRegistry` every device-aware subcommand shares.
//!
//! Explicit registration, per
//! `docs/architecture/driver-model.md`'s "`DriverRegistry`: explicit
//! registration" — `opm-cli` lists every driver crate it links against
//! here. Only one exists so far ([`opm_driver_ajazz_ak820`]).

use opm_core::registry::DriverRegistry;

/// Builds a registry with every driver crate `opm-cli` links against.
pub fn build() -> DriverRegistry {
    let mut registry = DriverRegistry::new();
    registry.register(Box::new(opm_driver_ajazz_ak820::AjazzAk820Driver));
    registry
}

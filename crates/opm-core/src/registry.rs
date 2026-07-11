//! `DriverRegistry`: the in-memory collection of available
//! [`Driver`] implementations that front-ends query to find out which
//! connected peripherals are supported and to open them. See
//! `docs/architecture/driver-model.md`'s "`DriverRegistry`: explicit
//! registration".

use crate::device::Device;
use crate::driver::Driver;
use crate::error::Error;
use crate::identity::Identity;

/// A collection of registered [`Driver`]s, queried to find which one (if
/// any) recognizes a given [`Identity`].
///
/// Front-ends (`opm-cli`'s `main.rs` today) list every driver crate they
/// link against explicitly — see `overview.md`'s "Later" item on
/// auto-registration.
#[derive(Default)]
pub struct DriverRegistry {
    drivers: Vec<Box<dyn Driver>>,
}

impl DriverRegistry {
    /// An empty registry with no drivers registered.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a driver to the registry.
    pub fn register(&mut self, driver: Box<dyn Driver>) {
        self.drivers.push(driver);
    }

    /// The first registered driver whose [`Driver::probe`] recognizes
    /// `identity`, if any. No I/O.
    pub fn find(&self, identity: &Identity) -> Option<&dyn Driver> {
        self.drivers
            .iter()
            .find(|driver| driver.probe(identity))
            .map(Box::as_ref)
    }

    /// Finds and opens a device in one call.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unsupported`] if no registered driver recognizes
    /// `identity`, or whatever the matching [`Driver::open`] returns.
    pub fn open(&self, identity: &Identity) -> Result<Box<dyn Device>, Error> {
        self.find(identity)
            .ok_or(Error::Unsupported)?
            .open(identity)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::capability::{Battery, Profiles, Rgb, RgbColor};

    fn fake_identity(vendor_id: u16, product_id: u16) -> Identity {
        Identity {
            vendor_id,
            product_id,
            manufacturer: None,
            product: None,
            serial: None,
            interfaces: Vec::new(),
        }
    }

    // A fake keyboard: RGB + profiles, no battery (wired).
    struct FakeKeyboard {
        identity: Identity,
        color: Cell<RgbColor>,
        profile: Cell<u8>,
    }

    impl Device for FakeKeyboard {
        fn identity(&self) -> &Identity {
            &self.identity
        }
        fn rgb(&self) -> Option<&dyn Rgb> {
            Some(self)
        }
        fn profiles(&self) -> Option<&dyn Profiles> {
            Some(self)
        }
    }

    impl Rgb for FakeKeyboard {
        fn get_color(&self) -> Result<RgbColor, Error> {
            Ok(self.color.get())
        }
        fn set_color(&self, color: RgbColor) -> Result<(), Error> {
            self.color.set(color);
            Ok(())
        }
    }

    impl Profiles for FakeKeyboard {
        fn active_profile(&self) -> Result<u8, Error> {
            Ok(self.profile.get())
        }
        fn set_active_profile(&self, profile: u8) -> Result<(), Error> {
            self.profile.set(profile);
            Ok(())
        }
    }

    struct FakeKeyboardDriver;

    impl Driver for FakeKeyboardDriver {
        fn name(&self) -> &str {
            "Fake Keyboard"
        }
        fn probe(&self, identity: &Identity) -> bool {
            identity.vendor_id == 0x1111 && identity.product_id == 0x0001
        }
        fn open(&self, identity: &Identity) -> Result<Box<dyn Device>, Error> {
            Ok(Box::new(FakeKeyboard {
                identity: identity.clone(),
                color: Cell::new(RgbColor { r: 0, g: 0, b: 0 }),
                profile: Cell::new(0),
            }))
        }
    }

    // A fake headset: battery only, no RGB, no profiles.
    struct FakeHeadset {
        identity: Identity,
        battery_percent: Cell<u8>,
    }

    impl Device for FakeHeadset {
        fn identity(&self) -> &Identity {
            &self.identity
        }
        fn battery(&self) -> Option<&dyn Battery> {
            Some(self)
        }
    }

    impl Battery for FakeHeadset {
        fn level_percent(&self) -> Result<u8, Error> {
            Ok(self.battery_percent.get())
        }
    }

    struct FakeHeadsetDriver;

    impl Driver for FakeHeadsetDriver {
        fn name(&self) -> &str {
            "Fake Headset"
        }
        fn probe(&self, identity: &Identity) -> bool {
            identity.vendor_id == 0x2222 && identity.product_id == 0x0002
        }
        fn open(&self, identity: &Identity) -> Result<Box<dyn Device>, Error> {
            Ok(Box::new(FakeHeadset {
                identity: identity.clone(),
                battery_percent: Cell::new(87),
            }))
        }
    }

    fn registry_with_both_fakes() -> DriverRegistry {
        let mut registry = DriverRegistry::new();
        registry.register(Box::new(FakeKeyboardDriver));
        registry.register(Box::new(FakeHeadsetDriver));
        registry
    }

    #[test]
    fn finds_the_driver_that_recognizes_the_identity() {
        let registry = registry_with_both_fakes();
        let keyboard_identity = fake_identity(0x1111, 0x0001);

        let driver = registry.find(&keyboard_identity).expect("should match");
        assert_eq!(driver.name(), "Fake Keyboard");
    }

    #[test]
    fn find_returns_none_for_an_unrecognized_identity() {
        let registry = registry_with_both_fakes();
        let unknown_identity = fake_identity(0x9999, 0x9999);

        assert!(registry.find(&unknown_identity).is_none());
    }

    #[test]
    fn open_fails_with_unsupported_for_an_unrecognized_identity() {
        let registry = registry_with_both_fakes();
        let unknown_identity = fake_identity(0x9999, 0x9999);

        assert!(matches!(
            registry.open(&unknown_identity),
            Err(Error::Unsupported)
        ));
    }

    #[test]
    fn keyboard_exposes_rgb_and_profiles_but_not_battery() {
        let registry = registry_with_both_fakes();
        let identity = fake_identity(0x1111, 0x0001);

        let device = registry.open(&identity).expect("should open");
        assert!(device.rgb().is_some());
        assert!(device.profiles().is_some());
        assert!(device.battery().is_none());

        let rgb = device.rgb().unwrap();
        rgb.set_color(RgbColor { r: 255, g: 0, b: 0 }).unwrap();
        assert_eq!(rgb.get_color().unwrap(), RgbColor { r: 255, g: 0, b: 0 });
    }

    #[test]
    fn headset_exposes_only_battery() {
        let registry = registry_with_both_fakes();
        let identity = fake_identity(0x2222, 0x0002);

        let device = registry.open(&identity).expect("should open");
        assert!(device.battery().is_some());
        assert!(device.rgb().is_none());
        assert!(device.profiles().is_none());

        assert_eq!(device.battery().unwrap().level_percent().unwrap(), 87);
    }
}

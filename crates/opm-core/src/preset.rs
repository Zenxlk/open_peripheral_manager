//! Host-side, file-backed presets: a named snapshot of a
//! [`LightingEffect`]/[`SleepTime`], saved locally and re-applied on
//! demand by calling the already-real [`Lighting`]/[`SleepTimer`]
//! capabilities. **Not** the device-stored [`crate::capability::Profiles`]
//! capability — see
//! `docs/architecture/decisions/0006-host-side-presets-not-onboard-profiles.md`
//! for why these are kept separate. Storage location (where the JSON
//! file lives) is a front-end concern and lives in `opm-cli`; this
//! module only owns the data shape and how to apply it.

use serde::{Deserialize, Serialize};

use crate::capability::{Lighting, LightingEffect, SleepTime, SleepTimer};
use crate::device::Device;
use crate::error::Error;

/// A named snapshot of lighting/sleep-timer settings. Either field may
/// be absent — a preset that only touches lighting doesn't need to
/// carry a sleep-timer value, and vice versa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preset {
    /// The lighting effect to apply, if this preset sets one.
    #[serde(default)]
    pub lighting: Option<LightingEffect>,
    /// The sleep-timer setting to apply, if this preset sets one.
    #[serde(default)]
    pub sleep_time: Option<SleepTime>,
}

impl Preset {
    /// Applies every field this preset carries to `device`, in order
    /// (lighting, then sleep timer). Fails fast on the first capability
    /// the device doesn't support or the first call that errors —
    /// already-applied fields are not rolled back.
    pub fn apply(&self, device: &dyn Device) -> Result<(), Error> {
        if let Some(effect) = self.lighting {
            let lighting: &dyn Lighting = device
                .lighting()
                .ok_or_else(|| Error::Driver("device does not support lighting".to_owned()))?;
            lighting.set_effect(effect)?;
        }
        if let Some(sleep_time) = self.sleep_time {
            let sleep_timer: &dyn SleepTimer = device
                .sleep_timer()
                .ok_or_else(|| Error::Driver("device does not support a sleep timer".to_owned()))?;
            sleep_timer.set_sleep_time(sleep_time)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::capability::{Direction, LightingMode, RgbColor};
    use crate::identity::Identity;

    struct FakeDevice {
        identity: Identity,
        applied_effect: Arc<Mutex<Option<LightingEffect>>>,
        applied_sleep_time: Arc<Mutex<Option<SleepTime>>>,
        supports_lighting: bool,
        supports_sleep_timer: bool,
    }

    impl Default for FakeDevice {
        fn default() -> Self {
            Self {
                identity: Identity {
                    vendor_id: 0x0000,
                    product_id: 0x0000,
                    manufacturer: None,
                    product: None,
                    serial: None,
                    interfaces: Vec::new(),
                },
                applied_effect: Arc::default(),
                applied_sleep_time: Arc::default(),
                supports_lighting: false,
                supports_sleep_timer: false,
            }
        }
    }

    impl Device for FakeDevice {
        fn identity(&self) -> &Identity {
            &self.identity
        }

        fn lighting(&self) -> Option<&dyn Lighting> {
            self.supports_lighting.then_some(self as &dyn Lighting)
        }

        fn sleep_timer(&self) -> Option<&dyn SleepTimer> {
            self.supports_sleep_timer.then_some(self as &dyn SleepTimer)
        }
    }

    impl Lighting for FakeDevice {
        fn set_effect(&self, effect: LightingEffect) -> Result<(), Error> {
            *self.applied_effect.lock().unwrap() = Some(effect);
            Ok(())
        }
    }

    impl SleepTimer for FakeDevice {
        fn set_sleep_time(&self, time: SleepTime) -> Result<(), Error> {
            *self.applied_sleep_time.lock().unwrap() = Some(time);
            Ok(())
        }
    }

    fn sample_effect() -> LightingEffect {
        LightingEffect {
            mode: LightingMode::Spectrum,
            color: RgbColor { r: 1, g: 2, b: 3 },
            brightness: 5,
            speed: 2,
            direction: Direction::Right,
        }
    }

    #[test]
    fn apply_calls_both_capabilities_when_both_fields_are_set() {
        let device = FakeDevice {
            supports_lighting: true,
            supports_sleep_timer: true,
            ..Default::default()
        };
        let preset = Preset {
            lighting: Some(sample_effect()),
            sleep_time: Some(SleepTime::FiveMinutes),
        };

        preset.apply(&device).expect("apply should succeed");

        assert_eq!(
            *device.applied_effect.lock().unwrap(),
            Some(sample_effect())
        );
        assert_eq!(
            *device.applied_sleep_time.lock().unwrap(),
            Some(SleepTime::FiveMinutes)
        );
    }

    #[test]
    fn apply_skips_absent_fields() {
        let device = FakeDevice {
            supports_lighting: true,
            supports_sleep_timer: true,
            ..Default::default()
        };
        let preset = Preset {
            lighting: Some(sample_effect()),
            sleep_time: None,
        };

        preset.apply(&device).expect("apply should succeed");

        assert!(device.applied_effect.lock().unwrap().is_some());
        assert!(device.applied_sleep_time.lock().unwrap().is_none());
    }

    #[test]
    fn apply_fails_when_device_lacks_lighting() {
        let device = FakeDevice {
            supports_lighting: false,
            supports_sleep_timer: true,
            ..Default::default()
        };
        let preset = Preset {
            lighting: Some(sample_effect()),
            sleep_time: None,
        };

        let err = preset
            .apply(&device)
            .expect_err("should fail: no Lighting capability");
        assert!(matches!(err, Error::Driver(_)));
    }

    #[test]
    fn apply_fails_when_device_lacks_sleep_timer() {
        let device = FakeDevice {
            supports_lighting: false,
            supports_sleep_timer: false,
            ..Default::default()
        };
        let preset = Preset {
            lighting: None,
            sleep_time: Some(SleepTime::Never),
        };

        let err = preset
            .apply(&device)
            .expect_err("should fail: no SleepTimer capability");
        assert!(matches!(err, Error::Driver(_)));
    }

    #[test]
    fn preset_round_trips_through_json() {
        let preset = Preset {
            lighting: Some(sample_effect()),
            sleep_time: Some(SleepTime::OneMinute),
        };
        let json = serde_json::to_string(&preset).expect("serialize");
        let decoded: Preset = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, preset);
    }
}

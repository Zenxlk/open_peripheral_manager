//! `pmctl preset` — saves/applies a named [`Preset`] (a host-side
//! snapshot of a lighting effect and/or sleep-timer setting), for
//! devices whose `opm-core` driver exposes `Lighting`/`SleepTimer`.
//! **Not** onboard device storage — see
//! `docs/architecture/decisions/0006-host-side-presets-not-onboard-profiles.md`.

use std::fs;
use std::path::PathBuf;

use opm_core::capability::{Direction, LightingEffect, LightingMode, SleepTime};
use opm_core::preset::Preset;

use super::rgb::parse_hex_color;
use super::{device, registry};

/// `pmctl preset`'s arguments.
#[derive(Debug, clap::Args)]
pub struct Args {
    /// Which device to target, as `VID:PID` hex (e.g. `0c45:800a`).
    /// Required only when more than one supported device is detected.
    /// Ignored by `list`, which doesn't need a device.
    #[arg(long, value_name = "VID:PID")]
    device: Option<String>,

    #[command(subcommand)]
    action: Action,
}

/// `pmctl preset`'s subcommands.
#[derive(Debug, clap::Subcommand)]
enum Action {
    /// Saves a preset — at least one of the lighting or sleep-timer
    /// flags must be given.
    Save {
        /// The preset's name — becomes `<name>.json` under the presets
        /// directory (see `preset_dir`).
        name: String,
        /// Lighting mode name — see `pmctl lighting modes`. Omit to
        /// leave lighting untouched by this preset.
        #[arg(long)]
        mode: Option<String>,
        /// `RRGGBB` hex color. Only meaningful together with `--mode`.
        #[arg(long, default_value = "ffffff")]
        color: String,
        /// Brightness, device-specific range (0-5 on the AK820).
        #[arg(long, default_value_t = 5)]
        brightness: u8,
        /// Animation speed, device-specific range (0-5 on the AK820).
        #[arg(long, default_value_t = 3)]
        speed: u8,
        /// Direction, for modes that support one: left, down, up, right.
        #[arg(long, default_value = "left")]
        direction: String,
        /// Sleep-timer preset name — see `pmctl sleep presets`. Omit to
        /// leave the sleep timer untouched by this preset.
        #[arg(long)]
        sleep: Option<String>,
    },
    /// Applies a previously saved preset.
    Apply {
        /// The preset's name.
        name: String,
    },
    /// Lists every saved preset name.
    List,
}

/// Runs the `preset` subcommand.
pub fn run(args: Args) {
    match &args.action {
        Action::Save { .. } => run_save(args),
        Action::List => run_list(),
        Action::Apply { .. } => run_apply(args),
    }
}

fn run_save(args: Args) {
    let Action::Save {
        name,
        mode,
        color,
        brightness,
        speed,
        direction,
        sleep,
    } = args.action
    else {
        unreachable!("run_save is only called for Action::Save");
    };

    let lighting = match mode {
        Some(mode) => match build_effect(&mode, &color, brightness, speed, &direction) {
            Ok(effect) => Some(effect),
            Err(message) => {
                eprintln!("{message}");
                std::process::exit(device::EXIT_USAGE);
            }
        },
        None => None,
    };

    let sleep_time = match sleep {
        Some(sleep) => match SleepTime::from_name(&sleep) {
            Some(time) => Some(time),
            None => {
                eprintln!("unknown sleep preset {sleep:?} — see `pmctl sleep presets`");
                std::process::exit(device::EXIT_USAGE);
            }
        },
        None => None,
    };

    if lighting.is_none() && sleep_time.is_none() {
        eprintln!("nothing to save — pass --mode and/or --sleep");
        std::process::exit(device::EXIT_USAGE);
    }

    let preset = Preset {
        lighting,
        sleep_time,
    };
    let path = preset_path(&name);
    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("failed to create {}: {err}", parent.display());
            std::process::exit(device::EXIT_FAILED);
        }
    }
    let json =
        serde_json::to_string_pretty(&preset).expect("serializing a Preset should never fail");
    if let Err(err) = fs::write(&path, json) {
        eprintln!("failed to write {}: {err}", path.display());
        std::process::exit(device::EXIT_FAILED);
    }

    println!("saved preset {name:?} to {}", path.display());
    std::process::exit(device::EXIT_OK);
}

fn run_list() {
    let dir = presets_dir();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            std::process::exit(device::EXIT_OK);
        }
        Err(err) => {
            eprintln!("failed to read {}: {err}", dir.display());
            std::process::exit(device::EXIT_FAILED);
        }
    };

    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension().and_then(|ext| ext.to_str()) == Some("json"))
                .then(|| path.file_stem()?.to_str().map(str::to_owned))
                .flatten()
        })
        .collect();
    names.sort();
    for name in names {
        println!("{name}");
    }
    std::process::exit(device::EXIT_OK);
}

fn run_apply(args: Args) {
    let Action::Apply { name } = &args.action else {
        unreachable!("run_apply is only called for Action::Apply");
    };
    let path = preset_path(name);
    let json = fs::read_to_string(&path).unwrap_or_else(|err| {
        eprintln!("failed to read preset {name:?} ({}): {err}", path.display());
        std::process::exit(device::EXIT_USAGE);
    });
    let preset: Preset = serde_json::from_str(&json).unwrap_or_else(|err| {
        eprintln!("failed to parse preset {name:?}: {err}");
        std::process::exit(device::EXIT_USAGE);
    });

    let registry = registry::build();
    let supported = device::supported_devices(&registry);
    let found = device::select_device(&supported, args.device.as_deref());
    let opened = device::open_or_exit(&registry, found);

    let outcome = preset.apply(opened.as_ref()).map_err(|err| {
        (
            format!("failed to apply preset {name:?}: {err}"),
            device::EXIT_FAILED,
        )
    });

    // Drop the device (and its Transport) explicitly before exiting —
    // see rgb.rs's `run` for why (ADR 0004, LibusbTransport's Drop
    // re-attaching the kernel HID driver).
    drop(opened);

    match outcome {
        Ok(()) => {
            println!("applied preset {name:?}");
            std::process::exit(device::EXIT_OK);
        }
        Err((message, code)) => {
            eprintln!("{message}");
            std::process::exit(code);
        }
    }
}

fn build_effect(
    mode: &str,
    color: &str,
    brightness: u8,
    speed: u8,
    direction: &str,
) -> Result<LightingEffect, String> {
    let mode = LightingMode::from_name(mode)
        .ok_or_else(|| format!("unknown mode {mode:?} — see `pmctl lighting modes`"))?;
    let color = parse_hex_color(color).map_err(|err| format!("invalid color {color:?}: {err}"))?;
    let direction = Direction::from_name(direction).ok_or_else(|| {
        format!("unknown direction {direction:?} — expected left, down, up, or right")
    })?;
    Ok(LightingEffect {
        mode,
        color,
        brightness,
        speed,
        direction,
    })
}

/// Where saved presets live: `$XDG_CONFIG_HOME/opm/presets/`, falling
/// back to `~/.config/opm/presets/`. No new dependency for this —
/// `opm-cli` doesn't yet need full cross-platform config-dir handling
/// (see `docs/roadmap.md`'s "Later, cross-cutting": Windows/macOS
/// support isn't implemented anywhere else in this project either).
fn presets_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("opm/presets");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| {
        eprintln!("could not determine the home directory (HOME is unset)");
        std::process::exit(device::EXIT_FAILED);
    });
    PathBuf::from(home).join(".config/opm/presets")
}

fn preset_path(name: &str) -> PathBuf {
    presets_dir().join(format!("{name}.json"))
}

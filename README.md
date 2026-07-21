# Open Peripheral Manager

A generic, vendor-agnostic peripheral manager for Linux (with Windows and
macOS as future targets), built around the idea that support for each
device lives in its own independent driver crate behind a shared
interface. Development starts with the **Ajazz AK820** keyboard, but the
AK820 is the first supported device, not the point of the project.

Status: **first device partly working end-to-end.** `pmctl` can
discover, list, and inspect HID devices, and open a real driver's
capabilities — for the one driver that exists so far
(`opm-driver-ajazz-ak820`), solid-color RGB control is validated
against physical hardware; everything else (reading state back,
profiles, a second device/vendor) is still ahead. See
[`docs/roadmap.md`](docs/roadmap.md) for exactly what's done per phase,
and a driver crate's own `docs/protocols/<vendor>-<model>/README.md`
for anything specific to running it against real hardware.

## Why

Most Linux peripheral tools are single-device, single-vendor scripts.
This project instead aims for a small, well-designed core library that
any number of manufacturer-specific drivers can plug into, reusable by
both a CLI and, eventually, a GUI. It's also the author's project for
learning Rust with an emphasis on doing it properly rather than quickly.

## Layout

```
crates/
  opm-core/       # library: vendor-agnostic device/driver abstractions
  opm-discovery/  # library: HID enumeration/grouping/classification
  opm-transport/  # library: Transport implementations (hidapi, libusb)
  opm-cli/        # binary `pmctl`: command-line front-end
drivers/          # one crate per supported device
  opm-driver-ajazz-ak820/  # the first one — see its own docs/protocols/ note
docs/             # architecture decisions, protocol notes, roadmap, devlog
```

See [`docs/architecture/overview.md`](docs/architecture/overview.md) for
the reasoning behind this layout.

## The CLI

```
pmctl discover              # every HID device on the system, supported or not
pmctl list                  # detected, supported peripherals
pmctl info [--device VID:PID]
pmctl rgb get|set <RRGGBB>
pmctl profile get|set <N>
```

All five subcommands are wired up and run against real hardware.
`discover`/`list`/`info` need no driver-specific protocol knowledge —
see [`docs/architecture/discovery.md`](docs/architecture/discovery.md).
`rgb`/`profile` depend entirely on what the matched driver actually
implements: against `opm-driver-ajazz-ak820` today, `rgb set` really
works, everything else in those two subcommands still returns "not yet
implemented" (`opm-driver-ajazz-ak820`'s own protocol notes explain
what that needs and, for some devices, extra setup like a permissive
udev rule or `sudo`).

## Building

```
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all
```

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). This project follows the
[Contributor Covenant](CODE_OF_CONDUCT.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE),
at your option — the Rust ecosystem convention.

# Open Peripheral Manager

A generic, vendor-agnostic peripheral manager for Linux (with Windows and
macOS as future targets), built around the idea that support for each
device lives in its own independent driver crate behind a shared
interface. Development starts with the **Ajazz AK820** keyboard, but the
AK820 is the first supported device, not the point of the project.

Status: **early scaffolding.** `pmctl discover` enumerates and classifies
HID devices for real (see below) — but there is no protocol
implementation or working driver yet — see
[`docs/roadmap.md`](docs/roadmap.md). This repository currently exists to
get the architecture and tooling right before any device-specific code is
written.

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
  opm-cli/        # binary `pmctl`: command-line front-end
drivers/          # one crate per supported device (empty for now)
docs/             # architecture decisions, protocol notes, roadmap, devlog
```

See [`docs/architecture/overview.md`](docs/architecture/overview.md) for
the reasoning behind this layout.

## The CLI

```
pmctl discover     # every HID device on the system, supported or not — implemented
pmctl list         # detected, supported peripherals — not implemented yet
pmctl info         # details about one — not implemented yet
pmctl rgb          # RGB lighting control — not implemented yet
pmctl profile      # device profile management — not implemented yet
```

`discover` works today: it needs no driver and no protocol knowledge,
only a connected HID device. See
[`docs/architecture/discovery.md`](docs/architecture/discovery.md).

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

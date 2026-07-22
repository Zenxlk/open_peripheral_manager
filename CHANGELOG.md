# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-07-21

Phase 6a/6c: the AK820's full lighting-mode vocabulary and its sleep
timer, both validated against physical hardware. No breaking changes —
every addition is a new, optional `Capability`.

### Added

- Phase 6a (lighting modes & animations): `opm_core::capability::Lighting`
  (`set_effect`), a new capability alongside `Rgb` for animated effects
  — mode, color, brightness, speed, direction — beyond a single solid
  color. `LightingMode` (all 20 AK820 modes), `Direction`, and
  `LightingEffect` live in `opm-core` as shared vocabulary; see
  [ADR 0005](docs/architecture/decisions/0005-lighting-capability-and-shared-effect-vocabulary.md)
  for why this is a separate capability and why the vocabulary isn't a
  guessed-at cross-vendor generalization.
  - `AjazzAk820Device::set_effect` reuses the already-validated
    `START`/`MODE`/data/`FINISH` transaction; `Rgb::set_color` is now a
    thin wrapper around it (`mode: Static`, still substituted for
    `Breath(speed=0)` on the wire).
  - `pmctl lighting set --mode <name> [--color/--brightness/--speed/
    --direction]` and `pmctl lighting modes`.
  - Validated against real hardware across a representative sample of
    modes — see `docs/protocols/ajazz-ak820/findings.md`'s 2026-07-21
    entries.
- Phase 6c (sleep timer): `opm_core::capability::SleepTimer`
  (`set_sleep_time`)/`SleepTime`, same design pattern as `Lighting`.
  - `AjazzAk820Device::set_sleep_time`, ported directly from
    gohv/EPOMAKER-Ajazz-AK820-Pro's own already-decoded sleep timer —
    no new capture needed. Its transaction shape differs from
    lighting's: a `byte2` field on `control_packet` lighting never
    needed, and no `FINISH` packet at the end.
  - `pmctl sleep set <preset>` and `pmctl sleep presets`.
  - Validated against real hardware.
- Reviewed a third community reference,
  [`wsclx/ak820pro-modder`](https://github.com/wsclx/ak820pro-modder),
  for Phase 6's remaining scope (per-key RGB, keymap, macros, profiles,
  TFT). Its wire protocol targets a different PID (`0x8009`) with a
  fundamentally different transport than this project's validated
  `0x800a` protocol, so nothing from it is ported directly — used only
  to prioritize `docs/roadmap.md`'s remaining Phase 6 sub-phases
  (6b-6h) and to confirm that none of the three known community
  references (gohv, TaxMachine, ak820pro-modder) have onboard profile
  switching decoded, ahead of Phase 6d.

## [0.1.0] - 2026-07-20

First tagged release: workspace scaffolding, device discovery, a
working HID transport layer, the `Driver`/`Device`/`Capability`
architecture, and the first real driver (`opm-driver-ajazz-ak820`)
with solid-color RGB control validated against physical hardware.

### Added

- Initial workspace scaffolding: `opm-core` and `opm-cli` (`pmctl`)
  crates, `drivers/` convention, quality tooling (rustfmt, clippy lints,
  CI), and documentation structure. No device support yet.
- Phase 1 (device discovery), designed in `docs/architecture/discovery.md`
  and validated against a real Ajazz AK820 Pro:
  - `Identity`/`Interface`/`UsagePair` domain types in `opm-core`
    (`identity.rs`), transport-free per
    [ADR 0002](docs/architecture/decisions/0002-discovery-lives-outside-opm-core.md).
  - New `opm-discovery` crate: `hidapi` enumeration adapter, sysfs
    topology resolution, path-dedupe/topology-based grouping, a
    keyboard/configurable-keyboard/vendor-only classification heuristic,
    HID report-ID extraction via `hidreport`, and a zero-I/O
    accessibility check. 10 unit tests, all pure functions over
    serializable data.
  - `pmctl discover`: one-line-per-device default output, `--verbose`
    for the per-interface breakdown, `--export json` for a portable,
    shareable report (host context + full device data, serial numbers
    redacted unless `--include-serial`), and the exit-code table from
    `discovery.md`.
  - `docs/inventory/` — a durable, cross-device catalog (`devices.md`,
    `captures/`), seeded with the AK820 Pro's first capture.
  - `docs/architecture/domain-model.md` — names the five facets
    (`Identity`, `Transport`, `Capabilities`, `Driver`, `Protocol`)
    every driver crate will need, and how the roadmap's phases map onto
    them.
- Phase 2 (HID transport), designed in `docs/architecture/transport.md`
  and validated against the real Ajazz AK820 Pro:
  - `Transport` trait, `Error`, and `ReadTimeout` in `opm-core`
    (`transport.rs`), transport-library-free per
    [ADR 0003](docs/architecture/decisions/0003-transport-trait-in-core-impl-in-opm-transport.md).
  - New `opm-transport` crate: `HidTransport`, a `hidapi`-backed
    implementation opening one HID interface at a time and exchanging
    Input/Output/Feature reports with it.
  - Ran a real `get_feature` round trip against the AK820's vendor
    configuration channel (interface 3, `/dev/hidraw4`) after installing
    a udev rule (`TAG+="uaccess"`) to grant access — the first confirmed
    real I/O exchange with the device's proprietary channel, and the
    permission gap `discovery.md` had already documented as expected.
- Phase 3 (driver system), designed in `docs/architecture/driver-model.md`:
  - `Driver` (stateless, `probe`/`open` split), `Device` (identity plus
    `Option<&dyn Trait>` capability accessors, defaulting to `None`),
    and `DriverRegistry` (explicit registration, first-match-wins) in
    `opm-core`.
  - Three capability traits — `Rgb`, `Battery`, `Profiles` — and
    `opm_core::error::Error`, composing `transport::Error` via `#[from]`.
  - Validated with two fake, non-overlapping in-memory devices (a
    keyboard with RGB+profiles, a headset with only battery): 5 unit
    tests, all passing. No real driver crate exists yet — that's
    Phase 4.
- Phase 4 (first driver): `drivers/opm-driver-ajazz-ak820`, the first
  real `Driver`/`Device` implementation, validated against the real
  Ajazz AK820 Pro:
  - `AjazzAk820Driver` matches the real VID:PID and opens the vendor
    interface confirmed reachable in Phase 2 (`0xff13/0x01`).
  - `AjazzAk820Device` exposes `Rgb`/`Profiles` (not `Battery` — wired
    keyboard); every capability method is a stub returning
    `Error::Driver`, except `get_color()`, which does a real read-only
    `get_feature` round trip first to prove the transport stays alive.
  - Ran the whole chain for real: `opm-discovery` → `DriverRegistry`
    (matches only the AK820 among the machine's other HID devices) →
    a real `HidTransport::open` → capability stubs. Not yet linked into
    `pmctl` (Phase 5). 5 unit tests, all passing, no hardware required.
- Phase 5 (CLI): `pmctl`'s subcommands wired to a real, explicitly-
  registered `DriverRegistry`, validated against the real Ajazz AK820
  Pro:
  - `list` — enumerates via the same `opm_discovery::discover()` call
    `discover` itself uses, filtered to supported devices.
  - `info [--device VID:PID]` — opens the device, prints identity plus
    which capabilities it exposes.
  - `rgb get`/`set <RRGGBB>`, `profile get`/`set <N>` — open the device,
    call the capability if present; `--device VID:PID` disambiguates
    when more than one supported device is detected. Against the AK820
    today these correctly surface Phase 4's stub "not yet implemented"
    error with exit code 1 — expected until Phase 6.
  - Ran every subcommand and error path (malformed/non-matching
    `--device`, invalid hex color) against the real device; correct
    exit codes (`0`/`1`/`2`) throughout.
- Phase 6 (protocol reverse-engineering), solid-color RGB: `pmctl rgb
  set` genuinely changes the real Ajazz AK820 Pro's lighting.
  - Decoded the vendor `SET_REPORT` command via real Wireshark/USBPcap
    captures against the vendor's official Windows software
    (`docs/protocols/ajazz-ak820/findings.md`).
  - New `opm-transport::LibusbTransport` (`rusb`-backed), added
    alongside `HidTransport` per
    [ADR 0004](docs/architecture/decisions/0004-libusb-transport-for-kernel-driver-interference.md):
    this device's Feature-report writes are silently swallowed by
    Linux's `hid-generic` kernel driver over `hidraw`, and only work
    with the kernel driver explicitly detached.
  - `AjazzAk820Device::set_color` reimplemented against
    `LibusbTransport`, sending the `START`/`START_MODE`/data/`FINISH`
    transaction gohv/EPOMAKER-Ajazz-AK820-Pro and
    TaxMachine/ajazz-keyboard-software-linux (credited in full in
    `findings.md`) had already reverse-engineered for the closely
    related PID `0x8009`. New `drivers/opm-driver-ajazz-ak820/src/
    protocol.rs` carries gohv's full `LightingMode`/`Direction`/
    `SleepTime` vocabulary, not just what `set_color` currently uses.
  - Fixed three more bugs surfaced only by real-hardware testing:
    missing inter-packet delays, a `get_feature` `wLength` off-by-one
    (found via an independent second-opinion review), and `pmctl`'s
    `std::process::exit()` skipping the `Drop` that re-attaches the
    kernel HID driver (fixed in `rgb`/`profile`/`info`).
  - Verified with four real colors and back-to-back runs with no
    manual intervention. `get_color`, profiles, sleep timer, and
    running without `sudo` remain open (see `findings.md`'s known
    gaps).

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project intends to adhere to [Semantic Versioning](https://semver.org/)
once it has a first release.

## [Unreleased]

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

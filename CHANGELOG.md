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

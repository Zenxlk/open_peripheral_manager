# Roadmap

No dates — this is a spare-time, learning-driven project. Organized as a
sequence of phases, each one a prerequisite for the next. Revisited as
reality dictates.

## Why this order

Earlier drafts of this roadmap put "reverse-engineer the AK820's
protocol" right after discovery — the intuitive order: find the device,
then learn how to talk to it. It's reordered so that every
architecture-shaping decision (the transport abstraction, the
`Driver`/`Device`/`Capability` traits) gets designed against the general
problem first, using only what's available *before* any protocol is
known — see [`architecture/domain-model.md`](architecture/domain-model.md)
for the vocabulary this depends on. The AK820's actual byte-level
protocol — the messiest, most vendor-specific part of the whole project —
is deliberately the second-to-last phase, not the first. Building the
driver system and CLI before knowing the protocol also forces those
phases to be designed around information genuinely available at
discovery time, not shaped around whatever the AK820 happens to look
like.

## Phase 0 — Project architecture

**Status: done.** Cargo workspace, crate-per-driver convention
([ADR 0001](architecture/decisions/0001-cargo-workspace-with-crate-per-driver.md)),
quality tooling, CI, docs structure. See
[`architecture/overview.md`](architecture/overview.md).

## Phase 1 — Device discovery

**Status: fully implemented** as designed in `discovery.md`. See
[`architecture/discovery.md`](architecture/discovery.md),
[ADR 0002](architecture/decisions/0002-discovery-lives-outside-opm-core.md),
and [`inventory/`](inventory/).

- [x] Get the AK820 enumerating via `hidapi` (throwaway script is fine)
      and capture the raw output — the first real test of
      `discovery.md`'s heuristics. Every heuristic held up unchanged; see
      `discovery.md`'s Findings section.
- [x] Record the AK820 in `inventory/devices.md`.
- [x] Implement the `opm-discovery` crate (`crates/opm-discovery`):
      `Identity` in `opm-core`, the `hidapi`/`hidreport` adapters,
      pure grouping/classification with unit tests, and `pmctl discover`
      wired up and run against the real AK820 (see its devlog entry).
- [x] `--export`, `--verbose`, and the exit-code table from
      `discovery.md` — implemented and run against the real AK820.

## Phase 2 — HID abstraction (Transport)

**Status: fully implemented** as designed in `transport.md`. See
[`architecture/transport.md`](architecture/transport.md) and
[ADR 0003](architecture/decisions/0003-transport-trait-in-core-impl-in-opm-transport.md):
a `Transport` trait (Input/Output/Feature reports, per-interface, no
`hidapi` dependency) lives in `opm-core`; a `hidapi`-backed
implementation lives in the new `opm-transport` crate.

- [x] Design the trait against Phase 1's real, validated `Identity`
      shape and `hidapi` 2.6.6's actual API — see `transport.md`.
- [x] Implement `opm_core::transport` (trait, `Error`, `ReadTimeout`).
- [x] Implement `opm-transport`'s `HidTransport`. 0 `clippy -D warnings`
      findings, `cargo fmt --check` clean, matching every other crate's
      gates.
- [x] Open a real AK820 interface and exchange a first report. Needed a
      udev rule (`TAG+="uaccess"`) first — the exact permission gap
      `discovery.md` predicted, confirmed then unblocked. A `get_feature`
      round trip against interface 3 (`/dev/hidraw4`, one of the AK820's
      vendor channels) succeeded for real. See `transport.md`'s Findings
      and the 2026-07-10 devlog.

## Phase 3 — Driver system

**Status: fully implemented** as designed in `driver-model.md`. The
`Driver`/`Device`/`Capability` traits and `DriverRegistry` — how a
driver crate declares "I can handle this `Identity`" and hands back a
`Device` exposing `Capabilities`. See
[`architecture/driver-model.md`](architecture/driver-model.md) and
[`architecture/domain-model.md`](architecture/domain-model.md).

- [x] Decide the `Capability` pattern (explicit `Device` accessors, not
      `dyn Any` downcasting — see `driver-model.md`'s reasoning),
      `Driver`'s stateless `probe`/`open` split, `DriverRegistry`'s
      explicit-registration shape, and fill in `opm_core::error::Error`.
- [x] Prototype against two fake, non-overlapping devices (a keyboard
      with RGB+profiles, a headset with only battery) as in-crate unit
      tests — 5 tests, all passing, 0 `clippy -D warnings` findings.
- [ ] Phase 4: a real driver crate for the AK820, validating
      `Driver::open()` against `opm-transport` and real hardware for the
      first time — these traits have only been proven against fakes so
      far.

## Phase 4 — First driver (Ajazz AK820)

**Status: fully implemented, validated against real hardware.**
`drivers/opm-driver-ajazz-ak820` (`AjazzAk820Driver`/`AjazzAk820Device`)
matches the real AK820's `Identity`, opens its vendor `Transport`, and
exposes `Rgb`/`Profiles` (not `Battery` — wired keyboard) with stub
implementations, per this phase's original goal.

- [x] `probe()` matches the real VID:PID (`0x0c45:0x800a`); `open()`
      finds and opens the vendor interface confirmed reachable in
      Phase 2 (usage `0xff13/0x01`, `/dev/hidraw4`). 5 unit tests
      (no hardware needed) cover `probe`/`open`'s error path.
- [x] Ran the real thing: `opm-discovery` → `DriverRegistry::find` (only
      matches the AK820, not the mouse/touchpad also on the machine) →
      `DriverRegistry::open` (a real `HidTransport::open` against
      `/dev/hidraw4`) → `rgb()`/`profiles()` return `Some`, `battery()`
      returns `None` → `get_color()` does a real, already-validated
      `get_feature(0, ..)` read (proving the transport stays alive) then
      correctly reports "protocol unknown" rather than fabricating a
      color. See the 2026-07-10 devlog.
- Known gaps, carried forward rather than silently accepted: VID:PID
  matching alone can't rule out a different Sonix-based rebrand sharing
  `0x0c45:0x800a`; only one of the AK820's three vendor usage pages is
  opened (interfaces 1's shared `0xffff/0x01` and 2's `0xff68/0x61` are
  unused — Phase 6 may need them too); `pmctl` doesn't link this driver
  crate yet, so `pmctl discover`'s own "unsupported" output is still
  accurate until Phase 5 wires it in.

## Phase 5 — CLI

**Status: fully implemented, validated against real hardware.**
`pmctl`'s subcommands are wired to a real, explicitly-registered
`DriverRegistry` (`opm-cli/src/commands/registry.rs`) — exactly as
Phase 4 predicted, commands run and devices are opened before a single
byte of the AK820's real protocol is known.

- [x] `list` — enumerates via the same `opm_discovery::discover()`
      `discover` itself calls, filtered to `driver status == supported`
      (see `discovery.md`'s now-resolved "Relationship to `list` and
      `info`").
- [x] `info [--device VID:PID]` — opens the device, prints identity plus
      which capabilities it exposes (`rgb`/`battery`/`profiles`).
- [x] `rgb get`/`rgb set <RRGGBB>`, `profile get`/`profile set <N>` —
      open the device, call the capability if present. Against the
      AK820 today these always fail with the Phase 4 stub's "not yet
      implemented" message and exit code 1 — expected, not a bug in
      this wiring; `docs/roadmap.md`'s Phase 6 is what changes that.
      `--device VID:PID` disambiguates when more than one supported
      device is present (untested against real hardware — only one
      AK820 unit exists to test with, same category of gap Phase 1
      flagged for topology grouping).
- [x] Ran every subcommand for real against the real AK820: `list`,
      `info`, `info --device 0c45:800a`, `rgb get`/`set`, `profile get`,
      plus the error paths (`--device` malformed, `--device` matching
      nothing, an invalid hex color) — all correct exit codes (`0`/`1`/
      `2`). See the 2026-07-10 devlog.

## Phase 6 — Protocol reverse-engineering

**Status: not started.** Figure out the AK820's actual vendor protocol
(RGB, profiles, whatever else it turns out to expose), turning Phase 4's
stub `Capability` implementations into real ones. Lives under
[`protocols/ajazz-ak820/`](protocols/ajazz-ak820/); the driver-internal
`Protocol` concept from `architecture/domain-model.md`.

## Phase 7 — GUI

**Status: not started, no toolkit chosen.** A GUI crate depending on
`opm-core` exactly like `opm-cli` does, once there's a stable enough
trait surface to build one against (candidates: egui, Tauri, iced).

## Later, cross-cutting

- Second device/vendor, to pressure-test that the driver abstraction
  (and the `Identity`/`Transport`/`Capabilities`/`Driver`/`Protocol`
  vocabulary) actually generalizes and isn't secretly AK820-shaped.
- Windows/macOS transport and discovery support.
- Publish `opm-core` (and stable drivers) to crates.io.

## Explicitly not planned right now

- Broad "support every keyboard" ambitions before the architecture has
  been proven against at least two real, different devices.
- A plugin/dynamic-loading system for drivers — static linking via the
  workspace is simpler and sufficient while the driver count is small.
- Auto-generating a driver crate's scaffolding from a `pmctl discover
  --export` report. A plausible future direction once Phase 1's report
  format has proven itself useful for humans first (see
  `architecture/discovery.md`) — not a near-term deliverable.

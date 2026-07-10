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

**Status: designed and validated against real hardware; crate not
implemented yet.** How to enumerate and classify HID devices without
knowing their protocol; design of `pmctl discover`, including its role
as a shareable report generator for community-driven device support. See
[`architecture/discovery.md`](architecture/discovery.md),
[ADR 0002](architecture/decisions/0002-discovery-lives-outside-opm-core.md),
and [`inventory/`](inventory/).

- [x] Get the AK820 enumerating via `hidapi` (throwaway script is fine)
      and capture the raw output — the first real test of
      `discovery.md`'s heuristics. Every heuristic held up unchanged; see
      `discovery.md`'s Findings section.
- [x] Record the AK820 in `inventory/devices.md`.
- [ ] Implement the `opm-discovery` crate for real and wire up
      `pmctl discover`, including `--export`.

## Phase 2 — HID abstraction (Transport)

**Status: not started, not yet designed.** A vendor-agnostic Rust
abstraction over the OS's HID layer (open/read/write, on top of whatever
the discovery crate already uses to enumerate) that driver crates depend
on instead of every driver calling `hidapi` directly. Deliberately
designed *after* Phase 1 is validated against real hardware, so its
shape is informed by what discovery actually needed and what opening/
writing to a real device actually requires — not guessed at in advance.
No document yet; gets its own architecture doc when its turn comes.

## Phase 3 — Driver system

**Status: sketched, not decided.** The `Driver`/`Device`/`Capability`
traits and `DriverRegistry` — how a driver crate declares "I can handle
this `Identity`" and hands back a `Device` exposing `Capabilities`. See
[`architecture/driver-model.md`](architecture/driver-model.md) and
[`architecture/domain-model.md`](architecture/domain-model.md).

## Phase 4 — First driver (Ajazz AK820)

**Status: not started, blocked on phases 1-3.** Build
`drivers/opm-driver-ajazz-ak820` against the real traits: match its
`Identity`, open its `Transport`, expose whatever `Capabilities` are
anticipated for it — even before those capabilities do anything real.
The goal of this phase is proving the architecture end-to-end for one
device (discovery → transport → driver → capability shape), *before*
any protocol reverse-engineering. Capability implementations are
expected to be stubs/no-ops at the end of this phase.

## Phase 5 — CLI

**Status: stubs only** (`list`/`info`/`rgb`/`profile` print
"not implemented"). Wire `pmctl`'s subcommands to real
`Driver`/`Device`/`Capability` calls. Since Phase 4 only produces stub
capabilities, this phase can be finished — commands run, devices are
listed and opened — before a single byte of the AK820's real protocol is
known.

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

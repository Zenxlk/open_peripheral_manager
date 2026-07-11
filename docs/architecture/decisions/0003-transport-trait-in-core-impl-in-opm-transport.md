# 0003. `Transport` trait lives in `opm-core`, `hidapi`-backed impl lives in `opm-transport`

Date: 2026-07-10

Status: Accepted

## Context

`docs/architecture/transport.md` designs `Transport` — Phase 2's
abstraction over opening one HID interface and exchanging Input/Output/
Feature reports with it, the layer Phase 3's `Driver`/`Device` and Phase
6's real AK820 protocol will be built on. ADR 0002 already established
`opm-core`'s real constraint: never depend on a transport library
(`hidapi`, `libusb`, `udev`, ...), not "zero dependencies forever." That
constraint applies here exactly as it did to `Identity`: `Device`
(Phase 3, in `opm-core`) needs to hold onto *something* that can read and
write reports, but `opm-core` cannot name a concrete `hidapi`-backed type
without depending on `hidapi`.

## Decision

The `Transport` trait, its `Error` type, and the `ReadTimeout` enum are
defined **in `opm-core`** (`opm-core::transport`), as plain signatures
with no transport-library dependency — mirroring ADR 0002's treatment of
`Identity`. `opm-core`'s future `Device` (Phase 3) holds a
`Box<dyn Transport>` (or similar) directly, with no indirection needed.

The `hidapi`-backed implementation lives in a new crate, `opm-transport`
(`crates/opm-transport`), following the same `opm-<role>` naming
convention as `opm-core`/`opm-discovery`/`opm-cli`. It **depends on
`opm-core`** (to implement its `Transport` trait) and on `hidapi`
internally — not the other way around. `opm-core` still never depends on
`hidapi`, `libusb`, `udev`, or any other transport-specific crate.

`opm-transport` does not depend on `opm-discovery`, and `opm-discovery`
does not depend on `opm-transport` — they solve different problems
(zero-I/O enumeration vs. sustained I/O) against the same underlying
`hidapi` library. Both are expected to wrap `hidapi::HidApi`/`open_path`
independently; this is accepted duplication, not a reason to merge the
crates (see `transport.md`'s reasoning).

## Consequences

- `opm-core` stays embeddable in a future GUI, or in a context with no
  HID access at all, without ever pulling in `hidapi` — extending the
  same property ADR 0002 established for `Identity` to `Transport`.
- `Driver`/`Capability` code (Phase 3) and `Protocol` code (Phase 6) can
  be written and tested against a fake, in-memory `Transport`
  implementation with no physical device attached — the concrete payoff
  of the trait living somewhere `hidapi`-free, since a test helper only
  needs to depend on `opm-core`, never on `opm-transport` or `hidapi`.
- Driver crates will depend on `opm-core` (for the trait/`Device`
  contract), `opm-discovery` (if they run discovery themselves, e.g. for
  `Driver::probe()`), and `opm-transport` (to actually open a device) —
  three separate dependency edges, none of which loop back through
  `opm-core`.
- `opm-transport`'s `HidTransport` cannot be meaningfully unit-tested
  without real hardware, same limitation `opm-discovery`'s `raw.rs`
  adapter already has — accepted for the same reason (see
  `discovery.md`'s "Testing strategy" and `transport.md`'s equivalent
  section).

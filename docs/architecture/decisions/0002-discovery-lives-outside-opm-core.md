# 0002. Discovery logic lives outside `opm-core`

Date: 2026-07-09

Status: Accepted

## Context

`docs/architecture/discovery.md` designs how OPM enumerates and
classifies connected HID devices before any driver or protocol code
exists. Doing that requires a transport-level library (`hidapi` or
equivalent) to actually talk to the OS's HID layer. ADR 0001 and
`docs/architecture/overview.md` already established that `opm-core` must
never depend on a transport library or know about any specific vendor —
not that it must have zero dependencies of any kind — and flagged "where
does HID access live" as an open question to resolve once a real need
forced it. Discovery is that forcing need.

The first draft of this ADR left the dependency direction between
`opm-core` and the discovery crate unstated ("`opm-core`'s trait
definition only names the type, it doesn't implement discovery itself"),
which doesn't hold up in Rust: naming a type in a trait signature
requires depending on the crate that defines it. Left as originally
written, `opm-core` would have ended up depending on the discovery crate
— and transitively on `hidapi` — the exact thing this ADR exists to
prevent. This revision fixes that.

## Decision

The plain-data type describing a detected-but-unopened HID interface
(vendor/product id, serial, interface topology, usage pages, ... — the
`Identity` facet in `docs/architecture/domain-model.md`) is defined
**in `opm-core`**, as a dependency-free data type (deriving `serde`'s
traits is fine — `serde` is a general-purpose crate, not a transport
library; see `overview.md`). `opm-core`'s future `Driver` trait names
this type directly, with no indirection needed.

Discovery (enumeration, physical-device grouping, classification) lives
in its own crate, separate from `opm-core`, and **depends on
`opm-core`** to produce values of that type — using `hidapi` internally
to do so. `opm-cli`'s `pmctl discover` depends on the discovery crate
directly, without going through `opm-core`'s `DriverRegistry` —
discovery must work with zero drivers registered. `opm-core` itself
still never depends on `hidapi`, `libusb`, `udev`, or any other
transport-specific crate.

The crate is named `opm-discovery`, at `crates/opm-discovery` — following
`overview.md`'s `opm-<role>` convention alongside `opm-core`/`opm-cli`,
and living under `crates/` rather than `drivers/` since it isn't
vendor-specific.

## Consequences

- `opm-core` stays embeddable in a future GUI, or in a context with no
  HID access at all (e.g. pure protocol analysis of a saved capture),
  without ever pulling in `hidapi` — while still being free to take on
  ordinary general-purpose dependencies (`serde`, an error-handling
  crate, ...) as real needs arise.
- Discovery can be developed, tested, and used (via `pmctl discover`)
  before a single `Driver` is written, since it only depends on
  `opm-core`'s data types, not on the registry or trait design in
  `driver-model.md` being finished.
- Driver crates will depend on both `opm-core` (for the trait contract)
  and the discovery crate (if they need to run discovery themselves,
  e.g. for `Driver::probe()`) — but the dependency edge that matters,
  `opm-core` never depending on discovery, is the one this ADR protects.
- The `Identity` type is a second, narrower vocabulary from `opm-core`'s
  `Device` — a detected `Identity` is not yet a `Device` until some
  driver opens it. Anyone reading the two types side by side needs this
  distinction spelled out (see `discovery.md`'s "Where this lives in the
  architecture").

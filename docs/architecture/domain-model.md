# Domain model: the five facets of a device

Status: design notes, not decided — like `driver-model.md`, this is a
working notebook. It exists so that the vocabulary used across
`docs/roadmap.md`'s phases 1-6 is settled *before* any of those phases is
implemented, so a second vendor's driver doesn't force renaming
everything the AK820's driver happened to call things.

## Why this document exists

Without a shared vocabulary, it's tempting to organize the project around
the first device it supports: a `drivers/ajazz` module, an `ak820.rs`
somewhere, code that "happens to" only make sense for one keyboard. The
project has already committed to one crate per driver
([ADR 0001](decisions/0001-cargo-workspace-with-crate-per-driver.md)), but
that ADR is about crate *boundaries*, not what's organized *inside* each
one. This document names the concepts every driver crate will need,
independent of any vendor, so that when the second device arrives its
driver crate has the same internal shape as the first, instead of a
bespoke structure invented under deadline.

## The five facets

A `Device` (the handle a front-end holds, per `driver-model.md`) is
composed of, or associated with, five distinct concepts:

```
Device
├── Identity      — who is it?
├── Transport      — how do you talk to it?
├── Capabilities   — what can it do?
├── Driver         — what matched it and opened it?
└── Protocol       — what does "talk to it" actually mean, in bytes?
```

| Facet | Question it answers | Owning phase | Owning document | Public in `opm-core`? |
|---|---|---|---|---|
| **Identity** | Vendor, product, serial, human-readable name, *and* declared interface topology (usage pages/usages, report-ID counts) — enough for a `Driver` to decide "this is mine" without opening anything | Phase 1 (Discovery) | `discovery.md` | Yes — defined in `opm-core` itself (see [ADR 0002](decisions/0002-discovery-lives-outside-opm-core.md)); the discovery crate depends on `opm-core` to produce it, using `hidapi` internally |
| **Transport** | How bytes actually move: HID over USB today, conceivably HID over Bluetooth or something else later | Phase 2 | `transport.md` | Yes — the `Transport` trait, defined in `opm-core` (see [ADR 0003](decisions/0003-transport-trait-in-core-impl-in-opm-transport.md)); the `hidapi`-backed implementation lives in `opm-transport`, using `hidapi` internally |
| **Capabilities** | The optional feature surface a `Device` exposes (RGB, profiles, battery, ...) | Phase 3 | `driver-model.md` | Yes — the capability trait pattern |
| **Driver** | The matcher/factory: given an `Identity`, decides "mine", opens a `Transport`, returns a `Device` with some `Capabilities` | Phase 3 | `driver-model.md` | Yes — the `Driver` trait and `DriverRegistry` |
| **Protocol** | The vendor-specific byte-level meaning of reports sent/received over a `Transport` — how a `Capability` is actually implemented for one device | Phase 6 | `docs/protocols/<vendor>-<model>/` | **No** — internal to each driver crate, never exposed outside it |

`Protocol` is deliberately the odd one out: every other facet is part of
the shared vocabulary a front-end or `opm-core` itself might reason
about. Protocol never is — it's the private implementation detail inside
one driver crate that makes that driver's `Capability` implementations
actually work. Nothing outside `drivers/opm-driver-ajazz-ak820` should
ever need to know the AK820 speaks in, say, 65-byte feature reports with
a two-byte command prefix; that fact lives entirely inside that crate,
informed by `docs/protocols/ajazz-ak820/findings.md`.

## How this maps onto phase ordering

The phase order in `roadmap.md` is, not coincidentally, this table read
top to bottom: `Identity` first (discovery doesn't need to open anything),
then `Transport` (needs to actually move bytes, but not know what they
mean), then `Driver`/`Capabilities` together (the matching and
feature-surface machinery — designed against *shapes* of capabilities,
not real ones), then a first driver proving that machinery against real
hardware with stub capabilities, then the CLI wired against those stubs,
and only last, `Protocol` — filling in what the stubs actually do.

## Non-goals of this document

- Designing `Transport`'s actual API in depth — that's `transport.md`'s
  job (Phase 2, now designed; see there for the trait itself).
- Finalizing the `Capability` mechanism (`dyn Any` downcasting vs. an
  accessor per known trait vs. `downcast-rs`) — still open, see
  `driver-model.md`.
- Changing ADR 0001. Crate-per-driver stands; this document is about
  internal module shape, not crate boundaries.

## `Identity` is deliberately HID-shaped, for now

`driver-model.md` cautions against designing `Driver::probe()`'s input
"against HID specifics", since the transport layer wasn't decided yet.
`Identity`, once actually designed in `discovery.md`, *is* HID-shaped
(usage pages, interface topology) — a deliberate, narrower choice than
that caution suggests, justified by `discovery.md`'s own non-goals: OPM
targets HID-class devices only for the foreseeable future, and a
maximally transport-neutral `Identity` designed against a hypothetical
non-HID device would be speculative generality with nothing real to
validate it against. If a non-HID transport is ever pursued, `Identity`
gets revisited then — not designed around in advance.

## Related documents

- [`discovery.md`](discovery.md) — designs `Identity` in depth (Phase 1).
- [`driver-model.md`](driver-model.md) — designs `Driver`/`Capabilities`
  (Phase 3).
- `docs/protocols/ajazz-ak820/` — where `Protocol` gets filled in for the
  first device (Phase 6).
- [`decisions/0001-cargo-workspace-with-crate-per-driver.md`](decisions/0001-cargo-workspace-with-crate-per-driver.md)
  — the crate-boundary decision this document deliberately doesn't touch.

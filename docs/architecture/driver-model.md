# Driver model (design notes, not yet decided)

This document tracks the open design questions for `opm-core`'s central
traits. Nothing here is implemented yet — see the placeholder modules in
`crates/opm-core/src/`. Treat this as a working notebook, not a spec: cross
out and rewrite as decisions are actually made.

## The core question

How does `opm-cli` (and later a GUI) talk to a keyboard, a mouse, and a
headset — three devices with almost nothing in common — through one
shared set of types, while each device's actual protocol lives in its own
isolated driver crate?

## Sketch of the pieces (names are placeholders)

- **`Driver`** — implemented once per device by its driver crate.
  Given information about a connected peripheral, decides whether it can
  handle it, and if so, opens a `Device`. Questions to resolve:
  - What does a driver need to *see* to decide "this is mine"? (USB
    vendor/product id? Something more?) This depends on decisions not yet
    made about the transport layer, so don't design it against HID
    specifics.
  - Is a `Driver` stateless (a pure matcher/factory), or does it own
    long-lived resources?

- **`Device`** — returned by `Driver::open`. The thing front-ends hold
  onto. Should expose only what is common to *every* device (identity,
  display name, maybe connection status) plus a way to reach optional
  features.

- **Capability pattern** — how a `Device` exposes the features it
  *does* have (RGB, profiles, battery, ...) without `Device` growing one
  method per feature that only some devices implement. Candidates worth
  evaluating hands-on:
  1. `dyn Any` downcasting: `device.as_capability::<dyn Rgb>()`.
  2. One `Option<&dyn Trait>` accessor per known capability directly on
     `Device` (simpler, but `Device` must know every capability that will
     ever exist).
  3. An existing crate such as `downcast-rs` to reduce boilerplate for
     option 1.
  
  This is the single most consequential design decision in `opm-core` —
  worth prototyping against at least two imagined devices (e.g. a
  keyboard with RGB + profiles, and a headset with only battery level)
  before committing.

- **`DriverRegistry`** — where drivers become discoverable. Open
  question: does `opm-cli`'s `main.rs` explicitly list every driver crate
  it links against (simplest, fully explicit, but means adding a driver
  requires editing `opm-cli`), or is there some auto-registration
  mechanism (e.g. the `inventory` or `linkme` crates)? Explicit listing is
  the safer starting point for a young project.

## Non-goals for `opm-core`, even long-term

- Talking to hardware directly (HID/USB reads and writes).
- Anything specific to one vendor's byte-level protocol.
- Anything specific to a front-end (no CLI formatting, no GUI widgets).

## Related documents

- [`overview.md`](overview.md) — why the crates are split the way they are.
- [`decisions/`](decisions/) — ADRs for decisions that have actually been
  made and are expected to stick.
- `docs/protocols/ajazz-ak820/` — the first real device this model will be
  tested against.

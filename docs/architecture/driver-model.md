# RFC: `Driver`/`Device`/`Capability` (Phase 3)

Status: Accepted, implemented, validated with fake in-crate devices (no
real driver exists yet — that's Phase 4).

Date: 2026-07-10

## Summary

`opm-discovery` (Phase 1) produces an `Identity`. `opm-transport`
(Phase 2) can open one HID interface and move bytes. Neither knows what
a keyboard, a mouse, or a headset actually *is* as a concept a front-end
can hold onto and call `.set_rgb_color(...)` on. This document designs
the three traits that close that gap: `Driver` (decides "is this mine,
and if so, open it"), `Device` (the handle a front-end holds), and the
`Capability` pattern (how `Device` exposes optional features like RGB or
battery level without knowing about every device that will ever exist).

## Motivation

`docs/roadmap.md` deliberately delayed this until Phases 1 and 2 were
both validated against real hardware — designing `Driver::probe()`'s
input against a real, working `Identity`, and `Driver::open()`'s
plumbing against a real, working `Transport`, rather than guessing at
either. Both are now real: `crates/opm-core/src/identity.rs` and
`crates/opm-core/src/transport.rs`. This is the last piece before Phase
4 can build the actual AK820 driver.

## Goals

- Decide `Driver`: what it needs to see to say "mine" (now knowable:
  it's an `&Identity`), whether it's stateless, and how "probe" (no
  I/O, just inspection) and "open" (real I/O, via `Transport`) relate.
- Decide `Device`: what's common to every device, and how it exposes
  optional features.
- Decide the `Capability` pattern concretely — the decision
  `driver-model.md`'s original draft flagged as "the single most
  consequential design decision in `opm-core`."
- Decide `DriverRegistry`'s shape.
- Fill in `opm-core::error`'s placeholder with a real `Error` type
  shared across all three traits.
- Prototype against at least two imagined devices with non-overlapping
  capability sets (a keyboard with RGB + profiles, a headset with only
  battery) — the check the original notebook asked for, done here as
  in-crate unit tests rather than left for later.

## Non-goals

- A real driver for the AK820. That's Phase 4, built *against* what
  this document decides, not designed here.
- `Protocol` — the byte-level meaning of any report. Stays entirely
  inside a driver crate (Phase 6), invisible to `opm-core`.
- Hotplug / reconnect. `Device` losing its `Transport` mid-session
  surfaces as an `Err` on the next call, same non-goal `transport.md`
  already carries.
- An auto-registration mechanism (`inventory`/`linkme`-style) for
  `DriverRegistry`. Explicit registration is decided below as the
  starting point; revisit only once there are enough driver crates for
  hand-listing them to actually hurt.

## Decisions

### `Driver`: stateless, `probe`/`open` split

A `Driver` implementation is a stateless, typically zero-sized matcher/
factory — a driver crate exposes a unit struct (e.g.
`pub struct AjazzAk820Driver;`) implementing this trait, not something
constructed with configuration.

`probe` and `open` are deliberately two separate methods, not one:

```rust
pub trait Driver: Send + Sync {
    /// A short, human-readable name for logging/CLI display (e.g.
    /// `"Ajazz AK820"`), not necessarily unique.
    fn name(&self) -> &str;

    /// Does this driver recognize the device described by `identity`?
    /// Pure inspection — no I/O, never opens anything. Safe to call
    /// against every registered driver for every discovered `Identity`,
    /// with zero side effects, e.g. to answer `pmctl discover`'s
    /// "supported/unsupported" column.
    fn probe(&self, identity: &Identity) -> bool;

    /// Opens the device, returning a live `Device`. Only ever called
    /// after `probe` returned `true` for the same `Identity`. Does real
    /// I/O (via `Transport`, from inside the driver crate — `opm-core`
    /// itself never touches a transport library, per ADR 0002/0003).
    fn open(&self, identity: &Identity) -> Result<Box<dyn Device>, Error>;
}
```

This mirrors the `Identity`/`Transport` split `discovery.md`/
`transport.md` already established: "detected" (`probe`, cheap,
side-effect-free, safe to run against every device on the machine) is a
different operation from "opened" (`open`, real I/O, only done when a
front-end actually wants to use one specific device). `pmctl discover`
already needs the first without the second — it must work with a
`DriverRegistry` holding real drivers without opening a single one of
the devices it lists.

### `Device`: identity plus capability accessors

```rust
pub trait Device: Send {
    /// What this device is — the same `Identity` its `Driver` matched.
    fn identity(&self) -> &Identity;

    /// `Some` if this device supports RGB lighting control.
    fn rgb(&self) -> Option<&dyn Rgb> {
        None
    }
    /// `Some` if this device reports a battery level.
    fn battery(&self) -> Option<&dyn Battery> {
        None
    }
    /// `Some` if this device supports switching between stored profiles.
    fn profiles(&self) -> Option<&dyn Profiles> {
        None
    }
}
```

Every accessor defaults to `None`, so a concrete `Device` only overrides
the ones it actually supports — a headset's `impl Device` doesn't
mention `rgb()`/`profiles()` at all, it just gets the default `None` for
free. See "The `Capability` pattern" below for why this shape was chosen
over the alternatives `driver-model.md`'s original draft listed.

### The `Capability` pattern

The original notebook listed three candidates: `dyn Any` downcasting,
one `Option<&dyn Trait>` accessor per known capability, or the
`downcast-rs` crate to reduce boilerplate for the first option.
**Decision: option 2 — explicit accessor methods on `Device`, shown
above.**

This needs justifying, because the `Any`-downcasting approach initially
looks more extensible (`Device` wouldn't need to change when a new
capability is invented). It doesn't hold up as cleanly as it first
appears:

- `std::any::Any::downcast_ref::<T>()` only ever succeeds against the
  *exact concrete, `Sized`, `'static` type* a value was originally
  stored as — it cannot downcast an opaque value to an unrelated trait
  object like `dyn Rgb`, because `dyn Rgb` is unsized and has no single
  `TypeId` of its own. Getting `Option<&dyn Rgb>` out of a type-erased
  `Device` via `Any` alone isn't the one-line trick it looks like; doing
  it for real needs either a wrapper-struct-per-capability trick (store
  a `Sized` newtype around `Box<dyn Rgb>` and downcast to *that*) or
  nightly-only trait-upcasting machinery. Either is real, working Rust —
  but it's meaningfully more machinery than this project's current
  handful of capabilities (`Rgb`, `Battery`, `Profiles`) justifies.
- The stated cost of the accessor approach — "`Device` must know every
  capability that will ever exist" — is real but small in practice:
  adding a new capability means adding one more default-`None` method to
  a trait already in `opm-core`. Default trait methods don't break
  existing implementors (a `Device` that doesn't override the new
  accessor keeps compiling, just answers `None`), so this is a small,
  additive, git-diffable change, not a breaking one.
- `Capabilities` are explicitly meant to be **public, shared vocabulary
  in `opm-core`** (`domain-model.md`'s table), not driver-private like
  `Protocol` — so `opm-core` "knowing about" `Rgb`/`Battery`/`Profiles`
  isn't a layering violation, it's the point: a GUI can render "this
  device has RGB" by calling `device.rgb().is_some()` without depending
  on any specific driver crate.

**Rejected for now, not forever:** if the number of capability kinds
grows large enough that editing `opm-core` for each new one becomes a
real bottleneck (dozens of narrow capabilities, frequent additions),
revisit with the `Any`-wrapper-newtype trick or `downcast-rs` — this
decision is about the right tool for a handful of capabilities today,
not a permanent rejection of type erasure.

Capability traits live in `opm-core::capability`, alongside `Device`'s
accessor methods that reference them:

```rust
/// An RGB color, 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

/// Devices that can report and change a solid RGB color.
///
/// Deliberately minimal for Phase 3/4 — per-key color, effects/
/// animations, and brightness are real AK820 features but are Phase 6
/// protocol questions, not part of proving the trait shape end-to-end.
/// Extend once a real driver's `Protocol` work needs more.
pub trait Rgb: Send {
    /// Reads the device's current color.
    fn get_color(&self) -> Result<RgbColor, Error>;
    /// Sets the device's color.
    fn set_color(&self, color: RgbColor) -> Result<(), Error>;
}

/// Devices that report a battery level (wireless keyboards, mice, ...).
pub trait Battery: Send {
    /// Battery level, 0-100.
    fn level_percent(&self) -> Result<u8, Error>;
}

/// Devices that store multiple configuration profiles switchable at
/// runtime.
pub trait Profiles: Send {
    /// The currently active profile's index.
    fn active_profile(&self) -> Result<u8, Error>;
    /// Switches to a different stored profile.
    fn set_active_profile(&self, profile: u8) -> Result<(), Error>;
}
```

Every capability method returns `Result<_, Error>` — talking to real
hardware can always fail (unplugged mid-call, a malformed response), so
none of these can honestly be infallible.

### `DriverRegistry`: explicit registration, `Vec<Box<dyn Driver>>`

```rust
pub struct DriverRegistry {
    drivers: Vec<Box<dyn Driver>>,
}

impl DriverRegistry {
    pub fn new() -> Self { .. }
    /// Adds a driver. Front-ends (`opm-cli`'s `main.rs` today) list
    /// every driver crate they link against explicitly — see
    /// `overview.md`'s "Later" item on auto-registration.
    pub fn register(&mut self, driver: Box<dyn Driver>) { .. }
    /// The first registered driver whose `probe` recognizes `identity`,
    /// if any. No I/O.
    pub fn find(&self, identity: &Identity) -> Option<&dyn Driver> { .. }
    /// Finds and opens a device in one call. `Error::Unsupported` if no
    /// driver recognizes it.
    pub fn open(&self, identity: &Identity) -> Result<Box<dyn Device>, Error> { .. }
}
```

"First match wins" — acceptable for now since no two drivers are
expected to claim the same `Identity`; revisit (e.g. require drivers to
disagree loudly, or rank by specificity) only once a real collision is
observed, not speculatively.

### `opm_core::error::Error`

The placeholder in `error.rs` ("start here when you're ready to design
error handling") is now real:

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No registered driver's `probe` recognized this `Identity`.
    #[error("no registered driver recognizes this device")]
    Unsupported,
    /// A `Transport` operation failed while opening or using a device.
    #[error(transparent)]
    Transport(#[from] crate::transport::Error),
    /// A driver-specific failure not covered by `Transport` (e.g. a
    /// malformed response it can parse enough to reject).
    #[error("{0}")]
    Driver(String),
}
```

`Transport`'s `Error` composes into this one via `#[from]` rather than
being duplicated — both live in the same crate (`opm-core`), so no new
dependency is needed for this, unlike `Identity`/`Transport` themselves
which needed ADRs 0002/0003 to resolve a real crate-boundary tension.
`Driver`/`Device`/`Capability` have no such tension: nothing about them
needs a transport library, so they stay directly in `opm-core` with no
new crate and no new ADR.

## Testing strategy

No real hardware and no real driver crate exist to test against yet
(Phase 4). Per the original notebook's own suggestion — "worth
prototyping against at least two imagined devices... a keyboard with RGB
+ profiles, and a headset with only battery level" — `opm-core`'s own
test suite includes two fake, in-memory `Driver`/`Device` pairs with
non-overlapping capability sets, proving:

- `DriverRegistry::find`/`open` correctly route to the matching driver
  and reject an `Identity` neither recognizes.
- A capability accessor returns `Some` exactly for the capabilities a
  concrete `Device` overrides, `None` for the rest — validating that the
  "`Device` must know every capability" cost really is as small as
  claimed: the headset's `impl Device` doesn't mention `rgb()`/
  `profiles()` at all.
- Capability methods on the fake devices use interior mutability
  (`Cell`) to simulate real device state across calls, proving the
  `&self`-only method signatures (matching `Transport`'s own convention)
  are workable without `&mut self` plumbed through `Box<dyn Device>`.

## Risks and open questions

- **The accessor-based `Capability` pattern doesn't scale indefinitely**
  — see "Rejected for now, not forever" above. Watch this once a second
  real driver (the "Later, cross-cutting" roadmap item) exists with a
  meaningfully different capability set than the AK820's.
- **"First driver wins" in `DriverRegistry` is unvalidated** against a
  real collision (two drivers both claiming one `Identity`) — no
  evidence yet that this matters, flagged so it isn't forgotten.
- **The three capability traits (`Rgb`/`Battery`/`Profiles`) are guesses
  at what's useful, not derived from the AK820's actual protocol** —
  Phase 6 (protocol reverse-engineering) may reveal the AK820 needs a
  richer `Rgb` (per-key, effects) or a capability not listed here at all
  (macros?). Adding one is the same small, additive change discussed
  above, not a redesign.
- **No real `Driver::open()` has ever run** — the fake devices in this
  document's tests never touch `opm-transport`/`hidapi` at all. The
  `Driver`/`Transport` integration (a real driver's `open()` actually
  calling `HidTransport::open`) is unverified until Phase 4.

## Next steps (feed into `docs/roadmap.md`)

- [x] Decide and implement `Driver`, `Device`, the `Capability` pattern,
      `DriverRegistry`, and `opm_core::error::Error`.
- [x] Prototype against two fake, non-overlapping devices (in-crate unit
      tests) — see "Testing strategy".
- [ ] Phase 4: build `drivers/opm-driver-ajazz-ak820` against these real
      traits, with stub `Capability` implementations, and validate
      `Driver::open()` against the real AK820 via `opm-transport`.

## Related documents

- [`overview.md`](overview.md) — why the crates are split the way they
  are; `Driver`/`Device`/`Capability` need no new crate or ADR, unlike
  `Identity`/`Transport`.
- [`domain-model.md`](domain-model.md) — the five-facet vocabulary this
  document fills in `Driver`/`Capabilities` for.
- [`decisions/`](decisions/) — ADRs 0002/0003, the crate-boundary
  decisions this document's `Error` type builds on without needing one
  of its own.
- `docs/protocols/ajazz-ak820/` — the first real device this model will
  be tested against, in Phase 4.

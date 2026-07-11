# 2026-07-10 — Phase 3: `Driver`/`Device`/`Capability` designed and implemented

Same day as the Transport RFC, continued straight into Phase 3 now that
both `Identity` (Phase 1) and `Transport` (Phase 2) are real and
validated against the AK820. `driver-model.md`'s original notebook
deliberately left `Driver`/`Device`/`Capability` undesigned as a
hands-on Rust learning exercise for the maintainer — asked directly
whether to keep that plan or have this session propose concrete traits,
same as it did for `Transport`; the maintainer chose the latter again.

Rewrote `docs/architecture/driver-model.md` from a "working notebook"
into a decided RFC, same shape as `discovery.md`/`transport.md`. Key
decisions, each with reasoning recorded in the document:

- **`Driver` is stateless**, a typically zero-sized unit struct per
  driver crate, with `probe`/`open` deliberately split into two methods
  — mirrors the `Identity`/`Transport` split `discovery.md`/
  `transport.md` already established: `probe` is cheap, side-effect-
  free inspection (safe to run against every device for `pmctl
  discover`'s "supported" column); `open` is real I/O, only run when a
  front-end actually wants to use one specific device.
- **The `Capability` pattern is explicit `Option<&dyn Trait>` accessor
  methods on `Device`** (`rgb()`, `battery()`, `profiles()`, each
  defaulting to `None`), not `dyn Any` downcasting — the option
  `driver-model.md`'s original notebook flagged as "the single most
  consequential design decision." Worked through why the `Any` route
  doesn't hold up as cleanly as it first looks: `downcast_ref::<T>()`
  only matches an exact concrete `Sized` type, and `dyn Rgb` isn't one —
  getting `Option<&dyn Rgb>` out of a type-erased `Device` via `Any`
  alone needs either a `Sized` newtype-wrapper-per-capability trick or
  nightly trait-upcasting, meaningfully more machinery than three
  capability traits justify today. Recorded as "rejected for now, not
  forever" — revisit if the capability count grows large.
- **Three capability traits for now**: `Rgb` (get/set a solid color —
  deliberately not per-key/effects, that's a Phase 6 protocol question),
  `Battery` (percent), `Profiles` (get/set active profile index). Named
  in `opm-core`, not driver-private, since `domain-model.md` already
  marked `Capabilities` as public/shared vocabulary — a GUI should be
  able to ask `device.rgb().is_some()` without depending on any driver
  crate.
- **`DriverRegistry` is explicit registration**, `Vec<Box<dyn Driver>>`,
  first-match-wins — matches `driver-model.md`'s original recommendation
  ("explicit listing is the safer starting point for a young project")
  and `overview.md`'s deferred auto-registration question.
- **Filled in `opm_core::error::Error`**, the placeholder left since the
  initial scaffold ("start here when you're ready to design error
  handling"). Composes `transport::Error` via `#[from]` — no new
  dependency needed since both live in the same crate — plus
  `Unsupported` (no driver matched) and a catch-all `Driver(String)` for
  driver-specific failures.

Unlike `Identity`/`Transport`, none of this needed a new crate or ADR:
`Driver`/`Device`/`Capability` have no transport-library dependency at
all, so they stay directly in `opm-core` — ADRs 0002/0003 exist
specifically because `Identity`/`Transport` had a real crate-boundary
tension to resolve; this phase doesn't.

Implemented all five files for real: `error.rs`, `capability.rs`,
`device.rs`, `driver.rs`, `registry.rs`. `cargo build`/
`clippy -D warnings`/`fmt --check` all clean. One clippy-shaped
correction along the way: an early test used `Result::unwrap_err()`,
which requires the `Ok` type to implement `Debug` — `Box<dyn Device>`
doesn't (trait objects aren't `Debug` unless the trait requires it), so
rewrote that assertion as `matches!(registry.open(...), Err(Error::Unsupported))`
instead of chasing a `Debug` bound onto `Device` for one test's sake.

Followed `driver-model.md`'s own testing strategy — its original
notebook asked for prototyping against "at least two imagined devices...
a keyboard with RGB + profiles, and a headset with only battery level"
before committing to the capability pattern. Wrote exactly that as
in-crate unit tests in `registry.rs`: a `FakeKeyboard` (overrides
`rgb()`/`profiles()`, using `Cell` for interior mutability under
`&self`-only methods — the same convention `Transport` already
established) and a `FakeHeadset` (overrides only `battery()`), each with
its own zero-sized `Driver`. Five tests, all passing: driver matching
finds the right one, an unrecognized `Identity` returns `None`/
`Error::Unsupported` correctly, and each fake device's capability
accessors return `Some`/`None` exactly as expected — concretely proving
the "`Device` must know every capability" cost really is as small as
`driver-model.md` claims: the headset's `impl Device` simply never
mentions `rgb()`/`profiles()`, and the trait's default `None` bodies
handle the rest.

Nothing here touched real hardware — that's explicitly Phase 4's job.
The fakes prove the trait *shapes* compile and compose correctly; a real
driver's `open()` actually calling `opm_transport::HidTransport::open`
against the AK820 is still unverified. `driver-model.md`'s "Risks"
section says this plainly.

Updated `domain-model.md` (Capabilities/Driver rows, no longer vague
about what "the capability trait pattern" means concretely),
`overview.md` (the `opm-core` section was stale — it still said "zero
dependencies right now," no longer true since `serde`/`thiserror`),
`roadmap.md` (Phase 3 marked fully implemented, Phase 4 marked
unblocked), and `docs/README.md` to stay in sync, same discipline every
phase so far has followed.

Next: Phase 4, `drivers/opm-driver-ajazz-ak820` — the first real driver,
with stub `Capability` implementations, validated by actually calling
`Driver::open()` against the real AK820 through `opm-transport`.

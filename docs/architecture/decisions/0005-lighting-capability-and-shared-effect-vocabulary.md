# 0005. Add a `Lighting` capability, with `LightingMode`/`Direction` living in `opm-core`

Date: 2026-07-21

Status: Accepted

## Context

Phase 6a (`docs/roadmap.md`) needs to expose the AK820's full 20-mode
lighting vocabulary — not just the solid color `Rgb::set_color` already
covers — through `pmctl`. The vocabulary itself
(`LightingMode`/`Direction`) already exists, ported from gohv/
EPOMAKER-Ajazz-AK820-Pro into
`drivers/opm-driver-ajazz-ak820/src/protocol.rs` back in Phase 6's
solid-color work, but it's private to that driver crate and only ever
used internally for the `Static` → `Breath(speed=0)` substitution.

Two questions had to be answered before writing any code:

1. Does animated-effect control belong on the existing `Rgb` trait, or
   is it a new capability? `Rgb`'s own doc comment already anticipated
   this moment: "effects/animations... are real AK820 features but are
   Phase 6 protocol questions... extend once a real driver's Protocol
   work needs more."
2. If it's a new capability, where does its vocabulary
   (`LightingMode`/`Direction`) live? A `Capability` trait lives in
   `opm-core` (`driver-model.md`'s decision), which cannot depend on any
   specific driver crate — so if the trait's methods take these enums as
   arguments, the enums have to live in `opm-core` too, not stay private
   to `drivers/opm-driver-ajazz-ak820`.

## Decision

**New capability, not an `Rgb` extension.** Added `pub trait Lighting:
Send { fn set_effect(&self, effect: LightingEffect) -> Result<(),
Error>; }` to `opm_core::capability`, alongside `Rgb`/`Battery`/
`Profiles`. `Rgb` stays exactly as it is (`get_color`/`set_color`) —
some devices this project might support later (a plain single-zone
mouse, say) may have a solid color without any animated-mode concept
at all, and conflating "can show one color" with "can run twenty
different animations with speed/direction" would make `Rgb` an
awkward-to-implement trait for the simple case.
`driver-model.md`'s own cost/benefit for the accessor pattern already
covers this: "adding a new capability means adding one more
default-`None` method to a trait already in `opm-core`... a small,
additive, git-diffable change."

**`LightingMode`, `Direction`, and `LightingEffect` move into
`opm_core::capability`, unapologetically shaped around what the AK820
(this project's only real driver) actually needs right now**, rather
than a guessed-at, hypothetically cross-vendor-generic effect
vocabulary. This is a deliberate, narrower choice than it might look:

- The alternative — inventing a small "lowest common denominator" set
  of generic effect categories (e.g. `Off`/`Static`/`Breathing`/
  `ColorCycle`) with an escape hatch for vendor-specific modes — was
  considered and rejected. It would force a lossy mapping from the
  AK820's real mode names (`Glittering`, `Explode`, `Shuttle`, ...) onto
  generic buckets, directly working against Phase 6a's actual goal
  (`docs/roadmap.md`: "apply the keyboard's different animations and
  modes" — plural, specific, by name), for a generalization this
  project can't yet validate against a second real device anyway (see
  roadmap's "Explicitly not planned right now": broad multi-vendor
  ambitions before two real devices exist).
- `driver-model.md`'s "Capability" section already establishes that
  `opm-core` "knowing about" a piece of shared vocabulary isn't a
  layering violation as long as it's public and shared, not
  driver-private — exactly `Rgb`'s `RgbColor` today, extended the same
  way here.

`LightingEffect { mode, color, brightness, speed, direction }`
deliberately does **not** normalize `brightness`/`speed` to a 0-100
percentage — they stay raw `u8`, device-specific range (0-5 today,
matching the AK820's own `MAX_BRIGHTNESS`/`MAX_SPEED`), documented as
such. Normalizing now would be designing for a second vendor's needs
that don't exist yet; `RgbColor`'s own `r`/`g`/`b: u8` fields already
set this precedent of "raw, un-normalized, driver clamps."

`drivers/opm-driver-ajazz-ak820/src/protocol.rs` keeps everything that
really is wire-specific: `CONTROL_REPORT_ID`, `CMD_START`/`CMD_MODE`/
`CMD_FINISH`/`CMD_SLEEP`, `MAX_BRIGHTNESS`/`MAX_SPEED`, and the
`control_packet`/`mode_data_packet` byte-layout builders — it now takes
`opm_core::capability::{LightingMode, Direction}` as input instead of
defining its own copies. `SleepTime` stays driver-private for now; Phase
6c (sleep timer) hasn't been designed yet and moving it prematurely
would be the same mistake this ADR just argued against, in the other
direction.

## Consequences

- `pmctl lighting set --mode <name> [--color RRGGBB] [--brightness N]
  [--speed N] [--direction <name>]` becomes possible, exposing all 20
  AK820 modes by name (`pmctl lighting modes` lists them), not just
  solid color.
- `Rgb::set_color` becomes a thin wrapper around `Lighting::set_effect`
  with `mode: LightingMode::Static` (which the driver still internally
  substitutes for `Breath(speed=0)`, per the existing, still-unexplained
  hardware quirk) — no behavior change, confirmed by the existing
  `set_color` unit test passing unchanged.
- If a second real driver ever needs lighting modes that don't fit this
  enum (a different mode set entirely, or a genuinely continuous
  parameter this discrete enum can't express), `LightingMode` stops
  being an honest "shared, generic" capability vocabulary and needs
  rework — accepted here as a real, not hypothetical, future cost, in
  exchange for not blocking today's actual, concrete need on a
  generalization nobody can validate yet.
- Every other AK820 mode beyond `Static`/`Breath` is still going out
  **unvalidated against real hardware** at the time this ADR is
  written — see `docs/protocols/ajazz-ak820/findings.md`'s 2026-07-21
  entry for what's confirmed versus still assumed.

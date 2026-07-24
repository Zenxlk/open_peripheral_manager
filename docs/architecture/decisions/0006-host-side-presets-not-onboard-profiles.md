# 0006. Host-side lighting/sleep-timer presets, distinct from onboard `Profiles`

Date: 2026-07-23

Status: Accepted

## Context

Phase 6d (`docs/roadmap.md`) set out to reverse-engineer the AK820's
onboard profile switching — the `Profiles` capability (`opm-core`,
Phase 3) has existed since before any protocol was known, stubbed ever
since. Two real captures were taken (see
`docs/protocols/ajazz-ak820/findings.md`'s 2026-07-23 entry) of the
vendor software's profile-switch UI. The result was ambiguous rather
than decoded: every observed switch emits the exact same two-command
sequence (`0x11` then `0x27`, wrapped in the same `START`/preamble/
`FINISH` shape already known from lighting and sleep), regardless of
which of the three configured profiles was the source or destination.
Nothing in the captured bytes varies with the target profile. Two
explanations remain open — a fixed two-step handshake unrelated to the
specific profile number, or an encoding this project hasn't found yet
— and resolving it would need more captures, including one against the
still-untested third profile.

Separately, the maintainer proposed a pragmatic alternative: since
solid-color/animated lighting (`Lighting`) and the sleep timer
(`SleepTimer`) are already real, validated capabilities, why not let
`pmctl` remember a named combination of their settings in a local file
and re-apply it on demand — sidestepping the onboard-profile protocol
question entirely for now.

## Decision

Add a **host-side, file-backed preset** feature — `pmctl preset save/
apply/list` — that is explicitly **not** the `Profiles` capability and
does not claim to be onboard storage. A preset is a named snapshot of
a `LightingEffect` and/or a `SleepTime`, written as JSON under
`$XDG_CONFIG_HOME/opm/presets/<name>.json` (falling back to
`~/.config/opm/presets/` — no new dependency; `opm-cli` already links
`serde`/`serde_json`), and applying one just calls the already-real
`Lighting::set_effect`/`SleepTimer::set_sleep_time` capability methods.
`Preset` itself (the data shape, `apply(&self, device: &dyn Device)`)
lives in `opm_core::preset`, not `opm-cli`, so a future GUI front-end
gets it for free — the same reasoning `driver-model.md` already applies
to `Capability`.

This is deliberately **not** a `Capability` and does **not** reuse the
`Profiles` trait, for a semantic reason, not just a naming one:
`Profiles`/`driver-model.md` describes something the *device* stores
and switches, which — per `ak820pro-modder`'s own protocol docs for
the same chip family (`GET_DEVICE_INFO.currentProfile`,
`GET_MACRO`/`SET_MACRO`'s in-device address space) — plausibly persists
on the keyboard itself, surviving a reboot or a different host with no
software installed at all. A host-side JSON file provides none of
that; conflating the two would make `Device::profiles().is_some()`
lie about what the device can actually do on its own. Phase 6d (real
onboard profile switching) stays open, explicitly parked rather than
abandoned, in `docs/roadmap.md`.

Presets only cover `Lighting`/`SleepTimer` for now — not macros or
keymap remapping, which the maintainer flagged as a plausible future
extension but explicitly unexplored territory (no capture, no known
wire format, no design yet).

## Consequences

- `pmctl preset` works today, independent of ever resolving Phase 6d's
  ambiguity — real, shippable value now instead of continued guessing
  at bytes that may not even encode what's being looked for.
- A preset applied on one machine has no effect anywhere else unless
  this project's config directory is copied along with it — explicitly
  weaker than what "onboard profile" implies, and documented as such
  everywhere this feature is mentioned so nobody mistakes one for the
  other.
- If Phase 6d is ever solved for real, `Profiles` gets its own real
  implementation independent of this — the two features coexist
  (`pmctl profile` for onboard switching, `pmctl preset` for host-side
  recall) rather than one replacing the other.
- `opm_core::capability`'s `RgbColor`/`LightingEffect`/`LightingMode`/
  `Direction`/`SleepTime` all need `Serialize`/`Deserialize` now, for
  `Preset` to (de)serialize them — a small, mechanical addition
  (`opm-core` already depends on `serde` for `Identity`, see ADR 0002's
  ancestry / `pmctl discover --export`).

# 2026-07-10 — Phase 5: `pmctl` wired to a real `DriverRegistry`

Fourth phase closed out the same day, same cadence as Phases 1-4. No new
architecture doc — like Phase 4, everything this phase implements was
already decided (`driver-model.md`'s `DriverRegistry`, `discovery.md`'s
already-written "Relationship to `list` and `info`" section); the work
was wiring, not design.

## What changed

- `opm-cli/src/commands/registry.rs` — `build()`, the one place that
  explicitly registers every driver crate `opm-cli` links against (just
  `AjazzAk820Driver` today), per `driver-model.md`'s explicit-
  registration decision.
- `opm-cli/src/commands/device.rs` — shared logic every device-aware
  subcommand needed: `supported_devices` (enumerate + filter to
  `registry.find().is_some()`), `select_device` (an explicit `--device
  VID:PID` selector, or automatic if exactly one supported device is
  present, or a clear "which one?" listing + exit 2 if ambiguous), and
  `open_or_exit`. Exit codes follow `discover`'s own convention (`0`
  normal, `1` a hardware/driver failure, `2` a usage problem) — extended
  here to cover "you need to disambiguate," which isn't something
  `clap` itself can validate.
- `list` — enumerates via the *same* `opm_discovery::discover()` call
  `discover` uses (through `supported_devices`), filtered to supported,
  one line per device. Closes the "should list reuse discover's
  enumeration or duplicate it" question `discovery.md` had left open —
  resolved as reuse, recorded there now.
- `info [--device VID:PID]` — opens the device for real, prints identity
  plus `rgb`/`battery`/`profiles: true/false`.
- `rgb get`/`rgb set <RRGGBB>`, `profile get`/`profile set <N>` — open
  the device, call the capability if the accessor returns `Some`,
  otherwise a clear "this device does not support X" instead of a panic
  or a confusing downstream error.

## Real-hardware validation

Ran the actual `pmctl` binary (not a throwaway probe this time — the
real CLI) against the real AK820:

```
$ pmctl list
AK820 — 0x0c45:0x800a — Ajazz AK820

$ pmctl info
AK820
  manufacturer: SONiX
  VID:PID: 0x0c45:0x800a
  interfaces: 4
  capabilities:
    rgb: true
    battery: false
    profiles: true

$ pmctl rgb get
failed to read color: AK820: Rgb::get_color (protocol unknown) not yet
implemented — see docs/protocols/ajazz-ak820/ (Phase 6, not started)
$ echo $?
1
```

`rgb set ff0000` and `profile get`/`set` behave the same way — real
`Driver::open()`, real capability dispatch, the Phase 4 stub's error
surfacing correctly all the way through the CLI. This is the intended
outcome, not a partial result: Phase 5's job was proving the wiring, and
Phase 4 already decided the capabilities themselves stay stubs until
Phase 6.

Also exercised the error paths directly: `info --device bad` (malformed
selector) and `info --device 9999:9999` (no match) both exit `2` with a
clear message; `info --device 0c45:800a` (a valid, matching selector)
works identically to the no-argument form since only one supported
device exists; `rgb set nothex` exits `2` from the hex-color parser.

## Known gaps for this phase

- **The "more than one supported device" branch of `select_device` is
  unverified against real hardware** — only one AK820 unit exists to
  test with. The code path is simple (list the ambiguous matches, exit
  2) and reviewed, but "reviewed" and "run against two real devices"
  are different things, same distinction `discovery.md`'s own Findings
  insisted on for topology grouping.
- **`rgb`/`profile` always fail today.** Not a defect in this phase —
  Phase 4 deliberately shipped stub capabilities — but worth restating
  plainly: nothing in `pmctl` can control the AK820's lighting or
  profiles yet. That's Phase 6's entire job.
- **No machine-readable output for `list`/`info`** (`discover` has
  `--export json`; these don't). Not requested, not designed — a
  plausible future nicety if scripting against `pmctl` ever comes up,
  not a near-term need.
- **`rgb get`'s real `get_feature` call is now reachable by anyone
  running `pmctl`**, not just this project's own validation scripts.
  Still the same read-only, already-proven-safe exchange from Phase 2's
  Findings — worth knowing it's now part of the public command surface,
  not just an internal check.

## Next

Roadmap phases 0-5 are all done. Phase 6 (protocol reverse-engineering,
`docs/protocols/ajazz-ak820/`) is next — the first phase that requires
genuinely new investigation (capturing the vendor's official software
talking to the AK820) rather than building against an already-decided
design. Phase 7 (GUI) and the "Later, cross-cutting" items (a second
device/vendor, Windows/macOS support) remain further out.

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

**Status: fully implemented** as designed in `discovery.md`. See
[`architecture/discovery.md`](architecture/discovery.md),
[ADR 0002](architecture/decisions/0002-discovery-lives-outside-opm-core.md),
and [`inventory/`](inventory/).

- [x] Get the AK820 enumerating via `hidapi` (throwaway script is fine)
      and capture the raw output — the first real test of
      `discovery.md`'s heuristics. Every heuristic held up unchanged; see
      `discovery.md`'s Findings section.
- [x] Record the AK820 in `inventory/devices.md`.
- [x] Implement the `opm-discovery` crate (`crates/opm-discovery`):
      `Identity` in `opm-core`, the `hidapi`/`hidreport` adapters,
      pure grouping/classification with unit tests, and `pmctl discover`
      wired up and run against the real AK820 (see its devlog entry).
- [x] `--export`, `--verbose`, and the exit-code table from
      `discovery.md` — implemented and run against the real AK820.

## Phase 2 — HID abstraction (Transport)

**Status: fully implemented** as designed in `transport.md`. See
[`architecture/transport.md`](architecture/transport.md) and
[ADR 0003](architecture/decisions/0003-transport-trait-in-core-impl-in-opm-transport.md):
a `Transport` trait (Input/Output/Feature reports, per-interface, no
`hidapi` dependency) lives in `opm-core`; a `hidapi`-backed
implementation lives in the new `opm-transport` crate.

- [x] Design the trait against Phase 1's real, validated `Identity`
      shape and `hidapi` 2.6.6's actual API — see `transport.md`.
- [x] Implement `opm_core::transport` (trait, `Error`, `ReadTimeout`).
- [x] Implement `opm-transport`'s `HidTransport`. 0 `clippy -D warnings`
      findings, `cargo fmt --check` clean, matching every other crate's
      gates.
- [x] Open a real AK820 interface and exchange a first report. Needed a
      udev rule (`TAG+="uaccess"`) first — the exact permission gap
      `discovery.md` predicted, confirmed then unblocked. A `get_feature`
      round trip against interface 3 (`/dev/hidraw4`, one of the AK820's
      vendor channels) succeeded for real. See `transport.md`'s Findings
      and the 2026-07-10 devlog.

## Phase 3 — Driver system

**Status: fully implemented** as designed in `driver-model.md`. The
`Driver`/`Device`/`Capability` traits and `DriverRegistry` — how a
driver crate declares "I can handle this `Identity`" and hands back a
`Device` exposing `Capabilities`. See
[`architecture/driver-model.md`](architecture/driver-model.md) and
[`architecture/domain-model.md`](architecture/domain-model.md).

- [x] Decide the `Capability` pattern (explicit `Device` accessors, not
      `dyn Any` downcasting — see `driver-model.md`'s reasoning),
      `Driver`'s stateless `probe`/`open` split, `DriverRegistry`'s
      explicit-registration shape, and fill in `opm_core::error::Error`.
- [x] Prototype against two fake, non-overlapping devices (a keyboard
      with RGB+profiles, a headset with only battery) as in-crate unit
      tests — 5 tests, all passing, 0 `clippy -D warnings` findings.
- [ ] Phase 4: a real driver crate for the AK820, validating
      `Driver::open()` against `opm-transport` and real hardware for the
      first time — these traits have only been proven against fakes so
      far.

## Phase 4 — First driver (Ajazz AK820)

**Status: fully implemented, validated against real hardware.**
`drivers/opm-driver-ajazz-ak820` (`AjazzAk820Driver`/`AjazzAk820Device`)
matches the real AK820's `Identity`, opens its vendor `Transport`, and
exposes `Rgb`/`Profiles` (not `Battery` — wired keyboard) with stub
implementations, per this phase's original goal.

- [x] `probe()` matches the real VID:PID (`0x0c45:0x800a`); `open()`
      finds and opens the vendor interface confirmed reachable in
      Phase 2 (usage `0xff13/0x01`, `/dev/hidraw4`). 5 unit tests
      (no hardware needed) cover `probe`/`open`'s error path.
- [x] Ran the real thing: `opm-discovery` → `DriverRegistry::find` (only
      matches the AK820, not the mouse/touchpad also on the machine) →
      `DriverRegistry::open` (a real `HidTransport::open` against
      `/dev/hidraw4`) → `rgb()`/`profiles()` return `Some`, `battery()`
      returns `None` → `get_color()` does a real, already-validated
      `get_feature(0, ..)` read (proving the transport stays alive) then
      correctly reports "protocol unknown" rather than fabricating a
      color. See the 2026-07-10 devlog.
- Known gaps, carried forward rather than silently accepted: VID:PID
  matching alone can't rule out a different Sonix-based rebrand sharing
  `0x0c45:0x800a`; only one of the AK820's three vendor usage pages is
  opened (interfaces 1's shared `0xffff/0x01` and 2's `0xff68/0x61` are
  unused — Phase 6 may need them too); `pmctl` doesn't link this driver
  crate yet, so `pmctl discover`'s own "unsupported" output is still
  accurate until Phase 5 wires it in.

## Phase 5 — CLI

**Status: fully implemented, validated against real hardware.**
`pmctl`'s subcommands are wired to a real, explicitly-registered
`DriverRegistry` (`opm-cli/src/commands/registry.rs`) — exactly as
Phase 4 predicted, commands run and devices are opened before a single
byte of the AK820's real protocol is known.

- [x] `list` — enumerates via the same `opm_discovery::discover()`
      `discover` itself calls, filtered to `driver status == supported`
      (see `discovery.md`'s now-resolved "Relationship to `list` and
      `info`").
- [x] `info [--device VID:PID]` — opens the device, prints identity plus
      which capabilities it exposes (`rgb`/`battery`/`profiles`).
- [x] `rgb get`/`rgb set <RRGGBB>`, `profile get`/`profile set <N>` —
      open the device, call the capability if present. Against the
      AK820 today these always fail with the Phase 4 stub's "not yet
      implemented" message and exit code 1 — expected, not a bug in
      this wiring; `docs/roadmap.md`'s Phase 6 is what changes that.
      `--device VID:PID` disambiguates when more than one supported
      device is present (untested against real hardware — only one
      AK820 unit exists to test with, same category of gap Phase 1
      flagged for topology grouping).
- [x] Ran every subcommand for real against the real AK820: `list`,
      `info`, `info --device 0c45:800a`, `rgb get`/`set`, `profile get`,
      plus the error paths (`--device` malformed, `--device` matching
      nothing, an invalid hex color) — all correct exit codes (`0`/`1`/
      `2`). See the 2026-07-10 devlog.

## Phase 6 — Protocol reverse-engineering

**Status: solid-color RGB, full lighting-mode/animation control (6a),
and the sleep timer (6c) done and validated against real hardware.**
`pmctl rgb set`/`pmctl lighting set`/`pmctl sleep set` all genuinely
change the AK820 Pro. `get_color` (6b), per-key RGB (6e), keymap (6f),
macros (6g), and TFT (6h) are still open — see the sub-phase breakdown
below. **6d (onboard profiles) is parked**, its motivating need served
instead by a new, cross-cutting, non-AK820-specific feature: host-side
presets (`pmctl preset save/apply/list/delete`,
[ADR 0006](architecture/decisions/0006-host-side-presets-not-onboard-profiles.md)) —
implemented and validated against real hardware (`pmctl preset apply`
genuinely changed the keyboard's lighting). Figure out the AK820's
actual vendor protocol, turning Phase 4's stub `Capability`
implementations into real ones. Lives under
[`protocols/ajazz-ak820/`](protocols/ajazz-ak820/); the driver-internal
`Protocol` concept from `architecture/domain-model.md`.

- [x] Decide the capture method: a Windows VM (`virt-manager`/`libvirt`/
      QEMU, USB host-device passthrough) with Ajazz's official software
      installed inside the guest, Wireshark + USBPcap. Capturing on the
      Linux host itself via `usbmon` was tried first and abandoned —
      see `protocols/ajazz-ak820/README.md`'s status note for why.
- [x] Ran real captures against the AK820 Pro and decoded the solid-
      color `SET_REPORT` command (interface 3, Feature report 0):
      `01 <R> <G> <B> 00×5 05 03 00×3 AA 55 00×48`, confirmed across
      four separate color changes. See `protocols/ajazz-ak820/
      findings.md`'s first 2026-07-20 entry.
- [x] `AjazzAk820Device::set_color` implemented against the decoded
      layout, unit-tested against a fake `Transport` — **run for real
      against the physical keyboard and confirmed to have no visible
      effect**, despite succeeding at the USB level. Not a bug in this
      driver crate; see the next line.
- [x] **Root cause found**: `opm-transport`'s `HidTransport` (hidapi,
      Linux hidraw backend) can read Feature reports fine but its writes
      are silently swallowed by the kernel's `hid-generic` driver for
      this device — reads and writes need genuinely different transport
      strategies here. Found and verified via two community open-source
      Linux tools for this keyboard family (credited in full in
      `findings.md`); confirmed for real against our own hardware by
      building one of them (`gohv/EPOMAKER-Ajazz-AK820-Pro`, PID patched
      from its native `0x8009` to our `0x800a`) and watching the
      keyboard actually change color. See `protocols/ajazz-ak820/
      findings.md`'s second and third 2026-07-20 entries for the full
      writeup, credits, and the still-open questions (why `mode=Static`
      doesn't work and needs a `Breath(speed=0)` substitution; whether
      `0x800a` and `0x8009` share the *entire* protocol or just lighting).
- [x] **Transport-strategy decision made and implemented**: ADR 0004
      (`architecture/decisions/0004-libusb-transport-for-kernel-driver-interference.md`)
      — a second `Transport` implementation, `LibusbTransport`
      (`opm-transport`, the `rusb` crate), detaches the kernel driver
      and speaks raw USB control transfers. `HidTransport` is
      unchanged and still the default for devices without this quirk.
- [x] **`AjazzAk820Device::set_color` re-implemented and validated for
      real.** Ported gohv/EPOMAKER-Ajazz-AK820-Pro's lighting-mode
      vocabulary into `drivers/opm-driver-ajazz-ak820/src/protocol.rs`
      (credited there); `set_color` now sends the full `START`/
      `START_MODE`/data(`Breath`, speed `0`)/`FINISH` sequence over
      `LibusbTransport`. Getting a genuinely visible result needed three
      more bug fixes beyond the transport switch itself — missing
      inter-packet delays, a `get_feature` `wLength` bug (found by an
      independent Fable 5 review, not by testing), and `pmctl`'s
      `std::process::exit()` skipping the `Drop` that re-attaches the
      kernel driver — see `protocols/ajazz-ak820/findings.md`'s final
      2026-07-20 entry for the full story. `pmctl rgb set` confirmed
      working with four different real colors.
- [ ] `pmctl rgb set`/`profile` still need `sudo` — the new `usb`-
      subsystem udev rule (`protocols/ajazz-ak820/99-ak820-usb.rules`)
      doesn't visibly grant access the way Phase 2's `hidraw` rule does;
      not diagnosed further, see findings.md's known gaps.
- [ ] `LibusbTransport::write_output`/`read_input` (interrupt-endpoint
      I/O) are implemented but never exercised against real hardware.

### Reference reviewed: `wsclx/ak820pro-modder`

Reviewed [`wsclx/ak820pro-modder`](https://github.com/wsclx/ak820pro-modder)
(macOS/Tauri, MIT-licensed) as a third community project for this
keyboard family — much broader feature coverage than gohv/TaxMachine
(per-key RGB, keymap, macros, TFT, system info). **Its wire protocol is
not compatible with ours and must not be ported byte-for-byte**: it
targets PID `0x8009` on macOS firmware 1.07 with a fundamentally
different transport (hidapi output reports + interrupt-IN responses, a
single `0xAA`-magic frame per command) versus our validated PID
`0x800a` protocol (Feature reports over `LibusbTransport`, the
`START`/`MODE`/`data`/`FINISH` four-packet transaction). Its own docs
note gohv/TaxMachine's protocol "is silently ignored by firmware 1.07
on macOS" — the mirror image of our own experience, where gohv's
protocol is exactly what worked. Two firmware/hardware variants of the
same chip, not a bug in either project. See `protocols/ajazz-ak820/
findings.md`'s 2026-07-21 entry for the full comparison and credit.

Used here only as a **feature-scope and code-organization reference**
— the command families below, and the precedent of a per-family module
under `protocol.rs`/a living `PROTOCOL.md`-style doc — not as a source
of opcodes or byte layouts, all of which still need their own capture
+ decode against our real `0x800a` unit.

### Sub-phases, in priority order

Each follows the same discipline as solid-color RGB: capture (Windows
VM + Wireshark, see `protocols/ajazz-ak820/README.md`) → decode and
record in `findings.md` → extend `protocol.rs` → design/extend the
`Capability` trait it needs (own ADR if it's a new trait, per
`driver-model.md`) → wire into the driver and `pmctl` → validate
against the real keyboard → update this roadmap → split commits, own
branch + PR.

- [x] **6a — Lighting modes & animations. Done, validated against real
      hardware.** New `Lighting` capability
      ([ADR 0005](architecture/decisions/0005-lighting-capability-and-shared-effect-vocabulary.md)),
      `pmctl lighting set --mode <name> [--color/--brightness/--speed/
      --direction]` and `pmctl lighting modes`. Reuses the already-
      validated `START`/`MODE`/`data`/`FINISH` transaction; `Rgb::set_color`
      is now a thin wrapper around it. A representative sample of modes
      confirmed working on the physical keyboard, no per-mode quirks
      found beyond `Static`'s known substitution; see
      `protocols/ajazz-ak820/findings.md`'s 2026-07-21 entries for the
      still-open detail (rainbow/color-mode byte unexplored).
- [ ] **6b — `get_color` (read back the current color).** Lower
      confidence — not obviously present in the `GET_REPORT` polling
      traffic captured so far (see Phase 6's findings), and neither
      gohv nor ak820pro-modder read color back either on their own
      protocols. May end up simply unsupported.
- [x] **6c — Sleep timer. Done, validated against real hardware.** New
      `SleepTimer` capability
      (`opm_core::capability::SleepTimer`/`SleepTime`, same pattern as
      `Lighting`/ADR 0005), `pmctl sleep set <preset>`/`pmctl sleep
      presets`. Full packet layout (`sleep_data_packet`, a `byte2`
      field on `control_packet` not needed by lighting) ported from
      gohv's own already-implemented sleep timer — no new capture
      needed, confirming the prediction. Transaction shape differs from
      lighting's: `START`/`SLEEP`-preamble/data, no `FINISH` packet.
- [ ] **6d — Onboard profile switching (`Profiles`). Parked, not
      abandoned.** Two real captures taken (see
      `protocols/ajazz-ak820/findings.md`'s 2026-07-23 entry) — the
      result is ambiguous, not decoded: every observed switch emits the
      exact same two-command sequence regardless of source/target
      profile, out of three configured. Either a fixed handshake
      unrelated to the specific profile, or an encoding not found yet;
      resolving it needs more captures (including the untested third
      profile) that haven't been prioritized since. Meanwhile, **Phase
      6d's motivating need — "let `pmctl` remember and reapply a named
      configuration" — is now served differently**: host-side,
      file-backed presets (`pmctl preset`, see
      [ADR 0006](architecture/decisions/0006-host-side-presets-not-onboard-profiles.md)),
      which don't need the onboard protocol solved at all. Real onboard
      profile switching (persists without this software installed, on
      a different host) stays open for whenever it's worth resuming.
- [ ] **6e — Per-key RGB.** New capture + decode required. High value,
      new `Capability` trait needed (see ak820pro-modder's
      `CustomLedMap`/`SET_CUSTOM_LED_DATA` for the *shape* of the
      problem — 128 LEDs, not the byte layout).
- [ ] **6f — Keymap remapping.** New capture + decode required. Large
      scope (128 slots × base + Fn layers); biggest single feature
      after lighting.
- [ ] **6g — Macros.** New capture + decode required. Large scope,
      lower priority than keymap.
- [ ] **6h — TFT display / clock sync.** Confirmed relevant — this
      unit has the 0.85" display. Deliberately last: ak820pro-modder's
      own TFT work (the most mature public reference available) is
      still marked 🚧 "wire-format decoded, visibility verification in
      progress" after significant effort, so expect this to be the
      hardest and most speculative sub-phase. No capture attempted yet.

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

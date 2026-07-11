# 2026-07-10 — Phase 4: first real driver, validated end-to-end

Third phase closed out in one day, following the same
design-implement-validate cadence as Phases 1-3. With `Driver`/`Device`/
`Capability` designed and implemented (Phase 3) and proven only against
fakes, Phase 4's job was to build the first *real* driver crate and
prove the whole chain — discovery → transport → driver → capability —
against actual hardware, before any protocol reverse-engineering.

Unlike Phases 1-3, this one didn't get its own RFC document: the
roadmap's Phase 4 description already fully specified the goal ("match
its `Identity`, open its `Transport`, expose whatever `Capabilities` are
anticipated, stubs are fine"), and there was no open architectural
question to research — every trait this driver implements was already
decided. Went straight to implementation.

## `drivers/opm-driver-ajazz-ak820`

`AjazzAk820Driver` (stateless, per `driver-model.md`) matches on the
real VID:PID (`0x0c45:0x800a`, from `docs/protocols/ajazz-ak820/`'s
Hardware Identity section) and opens the vendor interface confirmed
reachable in Phase 2's real-hardware validation — usage `0xff13/0x01`,
`/dev/hidraw4` on this machine — rather than guessing at one of the
AK820's other two vendor usage pages that never got a successful
`get_feature` response.

`AjazzAk820Device` exposes `rgb()`/`profiles()` as `Some(self)`, not
`battery()` (a wired keyboard has no battery to report). Every
capability method returns `Error::Driver("... not yet implemented ...")`
— except `get_color()`, which first does a real
`self.vendor.get_feature(0, &mut buf)` call (the exact read-only
exchange already proven safe in Phase 2's Findings) before still
returning the "protocol unknown" error. This wasn't just for flavor: a
`vendor: Box<dyn Transport>` field that no method ever reads triggers
rustc's `dead_code` lint (caught by `clippy -D warnings`), and the
honest fix was to make a stub actually *use* the transport rather than
silence the lint with `#[allow(dead_code)]` — which also means every
call to `get_color()` genuinely re-proves the transport is alive, not
just at `open()` time.

Deliberately did **not** attempt `set_color()`/`set_active_profile()`
against real hardware, even as a "does it work" probe — same reasoning
as Phase 2's validation: reading an already-safe, already-observed
exchange is fine; writing arbitrary bytes to a proprietary channel with
an unknown protocol is Phase 6's job, not something to do incidentally
while proving the driver's plumbing.

5 unit tests (no hardware needed): `probe()` matches the real AK820's
shape and rejects a different VID or PID; `vendor_interface_path` finds
`/dev/hidraw4` among a realistic 4-interface `Identity`; `open()` fails
cleanly with `Error::Driver` (not a panic) when no interface declares
the expected vendor usage pair, exercising the error path without ever
touching `opm-transport`/`hidapi`.

## Real-hardware validation

Wrote a throwaway probe (outside the repo, same convention as every
previous phase's hardware check) wiring a real `DriverRegistry` with
`AjazzAk820Driver` registered, run against real `opm-discovery` output
alongside the mouse and touchpad already on the machine:

```
0c45:800a -> matched driver "Ajazz AK820"
17ef:608d -> no driver (classification: UnknownHid)
04f3:3140 -> no driver (classification: UnknownHid)
```

`DriverRegistry::find` correctly matched only the AK820. `open()`
succeeded (the udev rule from Phase 2 is still installed).
`rgb()`/`profiles()` returned `Some`, `battery()` returned `None`.
`get_color()`'s real `get_feature(0, ..)` call succeeded at the
transport level (no error), and the method still correctly reported
"protocol unknown" instead of fabricating a color from bytes nobody has
decoded yet. `active_profile()` returned the plain stub error, as
designed — it doesn't touch the transport at all.

## Known gaps / risks for this phase (carried forward, not silently dropped)

- **VID:PID matching alone can't rule out a rebrand collision.** `probe`
  only checks `0x0c45:0x800a`; any other Sonix-based keyboard sharing
  that exact pair (unconfirmed to exist, but `discovery.md` already
  flagged the underlying gray-market VID reuse risk as real) would be
  misidentified as an AK820. No stronger signal (interface count, exact
  usage-pair set) is checked yet — would need a second, different
  Sonix-based keyboard to know whether this actually matters.
- **Only one of three vendor usage pages is opened.** Interface 1's
  usage pair shared with keyboard/consumer/mouse usages (`0xffff/0x01`)
  and interface 2's dedicated `0xff68/0x61` are untouched. If Phase 6
  finds the AK820's real RGB/profile/macro commands split across more
  than one channel, `AjazzAk820Device` will need more than one
  `Transport` field — a small, additive change, not a redesign, but
  worth remembering before assuming one channel is enough.
- **`pmctl` doesn't link this driver crate yet.** `opm-cli`'s `main.rs`
  has no `DriverRegistry` at all — `pmctl discover`'s "unsupported"
  column for the AK820 is still technically correct today, even though
  a driver now exists on disk. Wiring it in is explicitly Phase 5's job,
  not something this phase should reach ahead and do.
- **The `get_color()` transport-liveness check reads report ID `0`
  arbitrarily** — same caveat `transport.md`'s Findings already
  recorded: nothing confirms `0` is the "right" report to read for
  anything RGB-related; it's simply the one already known to respond.
  Expect this to change completely once Phase 6 has real answers.
- **No reconnect/retry logic** if the device is unplugged between
  `probe()` and `open()`, or mid-session — same non-goal `transport.md`
  already carries forward; `Driver::open()` and every capability method
  just surface whatever `opm-transport` returns.

## Next

Phase 5: wire `pmctl`'s subcommands (`list`, `info`, `rgb`, `profile`)
to a real `DriverRegistry` with `AjazzAk820Driver` registered — the
first time a front-end, not a throwaway probe script, exercises this
whole chain. `list`'s relationship to `discover` (see `discovery.md`'s
"Relationship to `list` and `info`") becomes concrete at that point.

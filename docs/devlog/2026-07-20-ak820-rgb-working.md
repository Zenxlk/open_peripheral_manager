# 2026-07-20 — Phase 6: solid-color RGB working end-to-end on the real AK820

The longest single day of work on this project so far, and the first
time any of Phase 4's stub `Capability` implementations actually did
something real to the physical keyboard. Full technical detail lives in
`docs/protocols/ajazz-ak820/findings.md` (six entries, all dated today)
and [ADR 0004](../architecture/decisions/0004-libusb-transport-for-kernel-driver-interference.md);
this is the narrative version.

## The capture

Picked up the Phase 6 capture rig from the prior session (Windows VM,
Wireshark + USBPcap) and actually ran it: three separate capture
sessions, four deliberate solid-color changes in the vendor software,
one via a preset swatch and three via the R/G/B sliders. Decoded a
clean `SET_REPORT` command — interface 3, Feature report 0, a 64-byte
payload with the color bytes exactly where a slider-driven change
landed them, plus a fixed, unexplained footer. See findings.md's first
entry for the byte table.

## Two dead ends before the real blocker

First guess: maybe a plain color write needs a separate "activate
Static mode" command first. Captured what looked like one (a distinct
opcode when switching modes in the vendor software), implemented it,
ran it for real — no visible effect. Second entry in findings.md; fully
superseded by what came next, kept for the record rather than deleted.

The actual blocker: `opm-transport`'s `HidTransport` (hidraw) can read
Feature reports fine but its *writes* are silently swallowed by Linux's
`hid-generic` kernel driver for this device. Every byte-exact replay —
through `pmctl`, a throwaway Rust example, an independent C
reproduction — succeeded at the USB level and did nothing to the
keyboard.

## Finding, and verifying, prior art

Rather than keep guessing, searched for existing Linux tooling for this
keyboard family and found two: TaxMachine/ajazz-keyboard-software-linux
(C++, first to reverse-engineer the transaction shape, but its own
`setColor()` is an admitted stub) and gohv/EPOMAKER-Ajazz-AK820-Pro
(Rust, a real working CLI+GUI). Didn't just read the source — cloned
both, patched their hardcoded PID from `0x8009` (a closely related
variant) to this project's actual `0x800a`, and built them. TaxMachine's
GUI: no effect, consistent with its stub. gohv's `ak820-ctl test`
(`sudo`, its own built-in validation command): **the keyboard's lighting
genuinely changed**, red → green → blue, live. First outside
confirmation that the two PIDs share enough protocol to matter, and
that the real problem was the kernel driver, not a byte-layout
difference this project had gotten wrong.

## Building it in this project's own architecture

ADR 0004: a second `Transport` implementation, `LibusbTransport`
(`opm-transport`, the `rusb` crate) — detaches the kernel driver,
speaks raw USB control transfers, implements the exact same `Transport`
trait `HidTransport` does, no trait changes needed. Ported gohv's
`LightingMode`/`Direction`/`SleepTime` vocabulary into a new
`drivers/opm-driver-ajazz-ak820/src/protocol.rs` (its own doc comment
credits both source repos and notes neither has a `LICENSE` file
GitHub recognizes — this is an independent re-expression of the
protocol facts, not copied source). Rewrote `set_color` to send the
full `START`/`START_MODE`/data/`FINISH` transaction gohv's code
sends, substituting `Breath(speed=0)` for `Static` per gohv's own
empirically-found workaround.

First real run: still nothing. Three more bugs, found in this order:

1. **No delay between packets.** gohv sleeps 10ms after every packet,
   100ms after the full transaction. This project's first attempt had
   no delays at all.
2. **`get_feature` requested one byte too many.** Copied
   `HidTransport`'s hidraw-specific "strip a synthetic leading Report
   ID byte" convention onto `LibusbTransport`, where it doesn't apply —
   raw `libusb` control transfers have no such byte. Sent the wrong
   `wLength` to a device already known to be picky about this exact
   exchange. **Caught by an independent second-opinion review** (asked
   a Claude Fable 5 agent to compare the two implementations line by
   line after two more real-hardware attempts had both failed for
   non-obvious reasons) — not found by staring at the code longer.
3. **`std::process::exit()` skips `Drop`.** `pmctl`'s `rgb`/`profile`/
   `info` commands all called it directly after using an opened device,
   which meant `LibusbTransport::drop()` — the code that re-attaches
   the kernel driver — never ran. Every single `pmctl` invocation left
   interface 3 permanently orphaned (confirmed via
   `/sys/bus/usb/devices/3-2:1.3/driver` showing no driver bound, even
   surviving a full physical unplug/replug — only a manual
   `usbhid` rebind fixed it). Restructured those three commands to
   compute their outcome, explicitly `drop()` the device, then exit
   once at the end.

After all three: `pmctl rgb set` confirmed working with four different
colors, and — the real test of fix #3 — two runs back-to-back with no
manual intervention between them, both successful.

## What's still open

Recorded in full in findings.md's known gaps and `roadmap.md`'s Phase 6
checklist: running `pmctl` without `sudo` (the new `usb`-subsystem udev
rule doesn't visibly take effect on this machine, unlike Phase 2's
`hidraw` one); `get_color`; profiles, sleep timer, and clock sync (the
vocabulary exists in `protocol.rs`, nothing's wired to a `Capability`
yet); `LibusbTransport::write_output`/`read_input` untested against
real hardware; and the structural fragility ADR 0004 now documents —
any future call site that skips `Drop` will reproduce the orphaned-
interface bug fix #3 solved, with no code-level guarantee against it.

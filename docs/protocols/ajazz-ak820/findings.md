# Findings

Suggested format per entry:

```
## YYYY-MM-DD — short title

What was tested, what capture it came from (see `captures/`), what was
learned, what's still unknown.
```

## 2026-07-20 — RGB solid color: `SET_REPORT` command decoded

Captured inside the Windows VM (Wireshark + USBPcap) against the real
Ajazz AK820 Pro, three separate sessions, four deliberate color changes
total (one via a preset swatch, three via the R/G/B sliders). Filters
used, refined over the session:

```
usb.device_address == <N> && usb.transfer_type == 0x02
  && usb.bmRequestType == 0x21 && usbhid.setup.bRequest == 0x09
```

Note: Wireshark's USB-HID dissector renames the raw `usb.setup.*` fields
to `usbhid.setup.*` once it recognizes a control transfer as HID class —
filtering on `usb.setup.bRequest` (as originally planned in this
directory's README) silently matches nothing once that happens; use
`usbhid.setup.bRequest` instead.

**Transport-level match:** every capture confirms `wIndex == 3`
(interface 3, the `0xff13/0x01` vendor channel — the same one
`transport.md` and Phase 4's driver already open), Report Type `3`
(Feature), Report ID `0`. This is exactly `Transport::set_feature(0, ..)`.

**Payload (64 bytes), four real examples, slider-driven:**

| RGB set (screenshot) | Captured bytes 0-15 (hex) |
|---|---|
| 254, 185, 115 | `01 FE B9 73 00 00 00 00 00 05 03 00 00 00 AA 55` |
| 118, 254, 211 | `01 76 FE D3 00 00 00 00 00 05 03 00 00 00 AA 55` |
| 206, 253, 85  | `01 CE FD 55 00 00 00 00 00 05 03 00 00 00 AA 55` |
| 219, 149, 254 | `01 DB 95 FE 00 00 00 00 00 05 03 00 00 00 AA 55` |

Bytes 1-3 match the set R/G/B exactly in all four, in that byte order.
Bytes 16-63 are all `0x00` in every capture.

**Decoded layout:**

```
byte 0      = 0x01      opcode: set solid color
byte 1      = R
byte 2      = G
byte 3      = B
bytes 4-8   = 0x00      reserved/padding, unvarying
bytes 9-10  = 0x05 0x03 fixed, unvarying — meaning unknown
bytes 11-13 = 0x00      reserved/padding, unvarying
bytes 14-15 = 0xAA 0x55 fixed, unvarying — likely a footer/magic marker
bytes 16-63 = 0x00      padding to the 64-byte Feature report
```

**One preset-swatch capture doesn't fit this layout exactly:** clicking
the red preset swatch (not dragging sliders) produced
`01 FF 00 00 00 00 00 00 00 01 05 03 00 00 00 AA 55 ...` — one extra
byte (`0x01`) inserted right before the `05 03` footer, shifting
everything after it by one position. Hypothesis, not confirmed: that
byte may be a preset index, present only when a swatch is used rather
than a manual RGB value. Not investigated further yet — the slider path
is unambiguous and sufficient for `Rgb::set_color`, which is now
implemented against it (`drivers/opm-driver-ajazz-ak820/src/lib.rs`).

**Implemented:** `AjazzAk820Device::set_color` builds this exact 64-byte
report and calls `set_feature(0, &report)`. Unit-tested against a fake
`Transport` (asserts the exact byte layout, not run against real
hardware in CI) — **not yet re-validated by actually running `pmctl rgb
set` against the physical keyboard**, see this entry's "Known gaps".

**`GET_REPORT` (the constant background polling) does not obviously
mirror the current color.** Checked one response right after a confirmed
color change (`DB 95 FE`) — its payload (`04 02 00 01 76 03 00 ...`)
doesn't contain that RGB anywhere obvious, and starts with the same
`04 02` tag seen in unrelated heartbeat `SET_REPORT` commands sent
throughout every capture regardless of user action. Reading the color
back (`Rgb::get_color`) is still unimplemented and still an open
question — likely a different report ID or a different vendor interface
(interface 1's `0xffff/0x01` or interface 2's `0xff68/0x61`, both
unopened by this driver) rather than something decodable from this
polling traffic.

## 2026-07-20 — `set_color` alone doesn't light up the keyboard: a separate "activate Static mode" command exists

Real-hardware test: after implementing `set_color` per the entry above,
ran `pmctl rgb set` against the real AK820 Pro (handed back from the
Windows VM to the Linux host first). Command succeeded (`color set`,
exit 0, `set_feature` returned `Ok`) but **the keyboard's LEDs did not
visibly change** — the write reaches the device but has no effect.

Asked the maintainer to confirm the vendor software's UI: there's a
separate **Modes tab**, and Static color only takes effect after
explicitly selecting the "Static" mode there — a distinct action from
picking R/G/B. Captured that action in isolation (switching from
another mode to Static, without touching color afterward):

```
02 FF 00 00 00 00 00 00 00 01 05 03 00 00 00 AA 55 00×47
```

- `byte 0 = 0x02` — a different opcode from `set_color`'s `0x01`.
- `bytes 1-3 = FF 00 00` (red) — likely the mode's default/last color,
  not something the maintainer chose; unconfirmed whether the firmware
  actually reads these bytes as a color or ignores them for this opcode.
- `byte 9 = 0x01` — an extra byte not present in the plain `01 <R> <G>
  <B>` slider-driven command, shifting the `05 03 ... AA 55` footer one
  position later (footer now at bytes 10-11 and 15-16 instead of 9-10
  and 14-15). The same extra byte appeared in the one preset-swatch
  color-set capture noted in the entry above — working hypothesis: this
  byte marks "value chosen from a fixed list/index" (mode, or a preset
  swatch) as opposed to a continuous slider value, though this is
  unconfirmed, not tested as its own variable.

**Hypothesis, not yet validated against real hardware:** `set_color`
needs to send *two* Feature reports — this "activate Static mode"
command (opcode `0x02`) with the target color substituted for the
captured `FF 00 00`, followed by the existing `0x01` color-update
command — to reliably light the keyboard regardless of whatever effect
mode it was last in. Implemented this way in
`drivers/opm-driver-ajazz-ak820/src/lib.rs`; substituting the caller's
RGB into the `0x02` command's bytes 1-3 is a guess (only ever observed
carrying the mode's default red, never a chosen color) and needs a real
`pmctl rgb set` run to confirm it doesn't misbehave.

**Known gaps, carried forward:**
- `set_color` has not been run against the real keyboard yet — only
  unit-tested with a fake transport. Needs a real `pmctl rgb set` run
  before this counts as validated, matching every other phase's
  discipline.
- `get_color` protocol is still unknown; stub unchanged.
- The `05 03` / `AA 55` fixed bytes are unexplained — could be a
  command-family tag, checksum, or protocol version marker. Not
  investigated since color-setting works without understanding them.
- The preset-swatch variant's extra byte is unexplained and untested as
  its own code path.
- Profiles, brightness, and effect-mode commands (mentioned in the
  README's suggested capture order) were not captured this round — only
  solid-color changes.

**Superseded by the entry below:** the "activate Static mode" opcode-`0x02`
hypothesis above turned out to be the wrong lead entirely. The real
blocker wasn't a missing command — see 2026-07-20's "root cause" entry.

## 2026-07-20 — Root cause found: Linux's kernel HID driver blocks Feature-report writes; needs libusb + kernel-driver detach (with credits)

**The actual blocker had nothing to do with byte layout.** Every
byte-exact replay of real captured commands failed on real hardware —
`pmctl rgb set` (via `opm-transport`'s `hidapi`/hidraw backend), a
hand-written throwaway example replaying the full `START`/`START_MODE`/
`data`/`FINISH` sequence with our own captured bytes, and an independent
minimal C program using `hidapi-hidraw` directly — all three reported
success at the USB level (no error, feature reports accepted) but
produced **no visible change on the keyboard**, across many attempts.

Found and tested two community, MIT-licensed(-ish — see below)
open-source Linux tools for this exact keyboard family:

- **[TaxMachine/ajazz-keyboard-software-linux](https://github.com/TaxMachine/ajazz-keyboard-software-linux)**
  (C++, targets PID `0x8009`) — first to reverse-engineer the
  `START`/`START_MODE`/`data`/`FINISH` four-command transaction shape
  and the `ModePacket` struct layout (`mode`, `r`, `g`, `b`, `rainbow`,
  `brightness`, `speed`, `direction`, `0xAA55` delimiter) that this
  directory's earlier entries independently rediscovered from our own
  captures. Its own `setColor()` is an empty `// TODO reverse engineer
  custom RGB data` stub — only `setMode()` is implemented, and even that
  wasn't confirmed working by us (see below).
- **[gohv/EPOMAKER-Ajazz-AK820-Pro](https://github.com/gohv/EPOMAKER-Ajazz-AK820-Pro)**
  (Rust, targets PID `0x8009`) — a more complete port (CLI + `egui` GUI)
  building on TaxMachine's protocol work, crediting it explicitly in its
  own README. This is the one that unblocked us. Two critical,
  independently-earned discoveries in its `src/usb.rs`:
  1. **The Linux kernel's `hid-generic` driver interferes with Feature
     report writes on this device.** Its code comment: *"Uses rusb
     (libusb) to detach the kernel driver and send SET_REPORT directly.
     The kernel's hid-generic driver interferes with feature reports, so
     we must bypass it via libusb."* The repo actually ships three
     backends (`hidapi`, raw `hidraw` ioctl, and raw `libusb`), but only
     wires the **libusb** one (`src/usb.rs`) into `main.rs` — the other
     two exist in the tree but aren't what `ak820-ctl`/`ak820-gui`
     actually use.
  2. **Requesting "Static" must not send `mode = Static (0x01)`.**
     `set_lighting()` silently substitutes `(LightingMode::Breath, 0)`
     (mode `0x07`, speed forced to `0`) whenever the caller asks for
     `Static` — an empirically-found workaround, not documented as to
     why `mode = 0x01` doesn't work.

**Verified against our real hardware, not just read:** cloned both
repos, changed their hardcoded `PRODUCT_ID` from `0x8009` to our real
`0x800a` (nothing else touched), and built:
- TaxMachine's full GUI (CMake, GTK3/OpenGL/GLFW/imgui) — ran it, no
  visible change from any action, same as our own attempts. Consistent
  with its `setColor()` being an admitted stub, and with it defaulting
  to the `hidapi-libusb` backend via `hid_open(vid, pid, nullptr)`
  (no explicit interface selection or kernel-driver handling called out
  in its `openHandle()` — unlike gohv's explicit detach step).
- gohv's `ak820-ctl` (`cargo build --release --bin ak820-ctl`, no GUI
  deps needed) — ran `sudo ak820-ctl test` (its own built-in validation
  command: Breath red → wait 5s → Breath green → wait 5s → Breath blue).
  **The keyboard's lighting genuinely changed for all three colors.**
  First real, external confirmation that PID `0x800a` speaks close
  enough to the same protocol as `0x8009`, and that the blocker really
  was kernel-driver interference, not a byte-layout difference.

**License note:** gohv's README states "MIT" but neither repo has an
actual `LICENSE` file (`GET /repos/.../license` returns `null` for
both) — noted here for accuracy, not acted on; nothing from either repo
is vendored into this codebase, only the protocol knowledge, credited
above.

**What this means for `opm-transport`/`opm-core`'s `Transport` trait —
an open design question, not yet decided:** every existing `Transport`
implementation (`HidTransport` in `opm-transport`) goes through
`hidapi`'s hidraw backend, validated in Phase 2 for **reads**
(`get_feature`) but now shown insufficient for **writes** on this
device, at least for the multi-step lighting transaction. Delivering a
real `Rgb::set_color` needs a write path that detaches the kernel HID
driver and speaks raw USB control transfers (`rusb`/libusb) — a
materially different transport strategy than what ADR 0003 chose for
Phase 2. Options not yet weighed: add a second `Transport` impl
alongside `HidTransport`; change `HidTransport` itself to detach/use
libusb only for feature-report writes; or something else. Needs its own
decision (and probably its own ADR) before `AjazzAk820Device::set_color`
can be re-implemented for real — the current implementation still uses
`opm_transport::HidTransport` and is now known not to work.

**Known gaps, carried forward (superseded by the entry below — kept for
history):**
- `opm-driver-ajazz-ak820`'s `set_color` (as currently implemented)
  does not work against real hardware — confirmed broken, not just
  unvalidated. Needs the transport-strategy decision above before
  re-implementing.
- The `Static → Breath(speed=0)` substitution is unexplained (why
  `mode=0x01` doesn't work) — accepted as an empirical workaround, not
  understood.
- Whether `0x800a` and `0x8009` share the *entire* protocol (profiles,
  sleep timer, LCD/clock sync, brightness-only changes) or only the
  lighting-mode transaction is untested — only the `test` command's
  Breath color cycle was verified.
- `get_color` still unimplemented; neither reference project reads
  color back either (gohv's `AK820Device` structs don't expose a getter).

## 2026-07-20 — `set_color` works for real: `LibusbTransport` implemented, three more bugs found and fixed

Following the "root cause" entry's decision (ADR 0004), implemented
`opm-transport::LibusbTransport` (`rusb`-backed, kernel-driver-detach,
same `Transport` trait as `HidTransport`) and rewrote
`AjazzAk820Driver`/`AjazzAk820Device` to use it, porting
gohv/EPOMAKER-Ajazz-AK820-Pro's `LightingMode`/`Direction`/`SleepTime`
vocabulary into a new `drivers/opm-driver-ajazz-ak820/src/protocol.rs`
(credited there; see that module's docs — an independent re-expression
of their protocol facts, not a copy of their source, since neither
source repo has a recognized `LICENSE` file). `set_color` sends
`START` → `START_MODE` → (`Breath` mode, speed `0`, target color) →
`FINISH`, each a `Transport::set_feature` call.

**First real run: still no visible effect**, despite byte-identical
packets to the now-confirmed-working `gohv` binary. Three more bugs
found, in order of discovery:

1. **No delay between packets.** `gohv`'s `send_feature` sleeps 10ms
   after every packet and an extra 100ms after the full 4-packet
   transaction ("small delay for device to process" / "extra delay
   after full transaction for device processing") — this project's
   first `LibusbTransport`/driver had no delays at all. Added
   `INTER_PACKET_DELAY`/`POST_TRANSACTION_DELAY` constants in
   `drivers/opm-driver-ajazz-ak820/src/lib.rs`, matching gohv's timing
   exactly.
2. **`LibusbTransport::get_feature` requested one byte too many.**
   Copied `HidTransport`'s "allocate `buf.len() + 1`, strip a leading
   Report-ID byte" convention without noticing it's a `hidraw`-ioctl-
   specific convention (the Linux kernel always synthesizes that byte
   for `HIDIOCGFEATURE` regardless of the device's own framing) — raw
   `libusb` control transfers have no such synthetic byte; the DATA
   stage is exactly what the device sends. Requesting `wLength = 65`
   instead of gohv's exact `64` sent the wrong request to a device
   already known to be picky about this specific exchange (recall
   TaxMachine's own comment: "No GET_REPORT — it corrupts EP0 state on
   this device"). Fixed to read exactly the caller's buffer length,
   with no byte-stripping. **Found by an independent second-opinion
   review (a Claude Fable 5 agent), not by direct testing** — asked for
   one specifically because two prior real-hardware attempts (byte-exact
   replay, then a from-scratch libusb reimplementation) had both failed
   for reasons not obvious from staring at the code longer.
3. **`std::process::exit()` skips `Drop`, so the kernel driver was
   never re-attached.** `pmctl`'s command implementations
   (`rgb.rs`/`profile.rs`/`info.rs`) all called `std::process::exit()`
   directly after using an opened device — which, on real hardware,
   left interface 3 permanently detached from Linux's `hid-generic`
   driver after every single run (confirmed via
   `udevadm`/`/sys/bus/usb/devices/3-2:1.3/driver` showing no driver
   bound, even after a full physical unplug/replug — only a manual
   `echo -n "3-2:1.3" | sudo tee /sys/bus/usb/drivers/usbhid/bind`
   brought it back). Fixed by restructuring `rgb.rs`/`profile.rs`/
   `info.rs` to compute the outcome first, explicitly `drop()` the
   opened device, *then* call `std::process::exit()` — `list.rs`/
   `discover.rs` don't open a device at all and don't have this problem.

**Confirmed working after all three fixes**, running the real
`pmctl rgb set` binary against the physical AK820 Pro (`sudo`, no udev
workaround needed for the *device* handling anymore — see the next
paragraph for the *permission* story): `feb973` (orange), `00ff00`
(green), `0000ff` (blue), `ff00ff` (magenta) all visibly changed the
keyboard's lighting. Two consecutive `rgb set` calls with no manual
intervention between them both succeeded — confirming fix #3 actually
solved the recurring breakage, not just the one run it was tested on.

**Permissions: a second, broader udev rule is needed**, beyond Phase
2's `hidraw`-only one — `docs/protocols/ajazz-ak820/99-ak820-usb.rules`
(`SUBSYSTEM=="usb" ... TAG+="uaccess"`). Installed it, but the real
`uaccess` ACL never actually appeared on the device node even after
reloading rules and a real physical replug (`getfacl` showed no
`user:...` entry) — cause not diagnosed, not worth blocking on. `pmctl
rgb set` was validated with `sudo` throughout; running it as a normal
user is still an open gap, tracked below, separate from the interface-
orphaning bug fix #3 solved.

**Known gaps, carried forward:**
- `pmctl rgb set` (and `profile`) still need `sudo` in practice — the
  `uaccess` udev rule for the `usb` subsystem doesn't visibly take
  effect on this machine/session, unlike the `hidraw`-only rule from
  Phase 2, which does. Not diagnosed further.
- `get_color` still unimplemented; the `05 03`/`AA 55`-family footer
  bytes are still unexplained; profiles/sleep-timer/clock-sync/LCD are
  still unported despite the vocabulary now existing in `protocol.rs`.
- Whether `0x800a` and `0x8009` share the *entire* protocol or just the
  lighting-mode transaction is still untested beyond solid colors.
- `LibusbTransport::write_output`/`read_input` (interrupt-endpoint I/O,
  needed for e.g. the LCD image upload gohv also implements) are
  written but have never been exercised against real hardware — only
  `get_feature`/`set_feature` (control transfers) have.
- If a `LibusbTransport` caller's process ever exits without running
  `Drop` (a panic that unwinds past the caller but still enters `Drop`
  is fine; a hard crash, `SIGKILL`, or another `std::process::exit()`
  call site added later without the same care is not), interface 3
  will need the same manual `usbhid` rebind used throughout this
  investigation to recover — this is a structural fragility of the
  kernel-driver-detach approach itself (ADR 0004's "Consequences"),
  not fully eliminated, only worked around at every currently-known
  call site.

## 2026-07-21 — Reviewed `wsclx/ak820pro-modder`: a third community project, incompatible wire protocol

Cloned and read [`wsclx/ak820pro-modder`](https://github.com/wsclx/ak820pro-modder)
(MIT license, `LICENSE` file present and GitHub-recognized — unlike
gohv/TaxMachine) to scope out what's left of this project's Phase 6:
lighting modes/animations, per-key RGB, keymap, macros, sleep timer,
profiles, TFT. It's the most feature-complete of the three community
projects found so far — a Tauri 2 (Rust + React) app targeting macOS,
with a pure-Rust `ak820-protocol` crate organized one module per
command family (`lighting.rs`, `keymap.rs`, `macros.rs`,
`per_key_rgb.rs`, `sleep.rs`, `system.rs`, `tft.rs`, `clock.rs`) and a
living `docs/PROTOCOL.md` documenting every decoded command — a similar
discipline to this directory's own `findings.md`/`README.md` split.

**Its wire protocol is a different family from gohv/TaxMachine's (the
one this project has already validated against real `0x800a`
hardware), not a variant of it.** Confirmed by reading its own
`docs/PROTOCOL.md`, which documents this explicitly under "Where the
upstream Linux ports went wrong": it targets PID `0x8009` against the
official AJAZZ driver on macOS firmware 1.07, and found that
gohv/TaxMachine's entire framing — Feature reports, `0x04` control
report ID, `START`(0x18)/`MODE`(0x13)/`FINISH`(0xf0), the 64-byte
mode-data payload — "is silently ignored by strict firmware 1.07 on
macOS". Its replacement format uses plain HID **output** reports
(report ID `0`, no Feature reports, no kernel-driver detachment needed
on macOS), a single `0xAA`-magic frame per command
(`[0xAA, cmd, len, addr_lo, addr_hi, opt0, opt1/last-flag, opt2,
payload…]`), and a completely different command byte table
(`SET_LED_EFFECT = 0x23`, `GET_LED_EFFECT = 0x13` — notably the same
numeric value TaxMachine used for `CMD_MODE`, apparently coincidence:
different command, different framing, different transport).

This is the mirror image of this project's own experience — where
gohv/TaxMachine's protocol is exactly what worked, validated with four
real color changes on our physical `0x800a` unit (see the two entries
above). Neither side's project is wrong; **two firmware/hardware
variants of the same Sonix SN32F299-based board genuinely speak
different wire protocols**, distinguished at minimum by PID (`0x8009`
vs `0x800a`) and possibly by OS-side transport differences (macOS
IOHIDDevice vs Linux hidraw/libusb) that haven't been isolated from the
firmware difference.

**Conclusion: nothing from `ak820pro-modder` gets ported byte-for-byte
into this project.** Its value here is as a **feature-scope and
code-organization reference** — confirms which command families exist
on this keyboard family in general (lighting modes, per-key RGB,
keymap incl. Fn layer, macros, sleep timer, onboard profiles, TFT
upload), and its per-family module split is a reasonable shape to
mirror in `protocol.rs` as this project's own Phase 6 sub-phases grow —
but every opcode and byte offset still needs its own capture against
our real `0x800a` unit. `docs/roadmap.md`'s Phase 6 sub-phases (6a-6h)
are prioritized and scoped from this review.

**Known gaps, carried forward:**
- Whether the protocol difference is purely firmware-version-driven
  (i.e. a `0x800a` unit could theoretically be re-flashed to speak
  `ak820pro-modder`'s protocol, or vice versa) or tied to the PID/
  hardware revision itself is unconfirmed and not worth investigating —
  out of scope, this project targets the real unit's protocol as-is.
- `ak820pro-modder`'s TFT work (`docs/PROTOCOL.md`'s `SET_TFT_USER_ANIMATION`
  section) is the most detailed public documentation of an AK820-family
  TFT upload found so far, but the project's own README marks it 🚧
  "wire-format decoded, visibility verification in progress" even after
  substantial effort — a signal that Phase 6h (TFT) should be scoped as
  the hardest sub-phase, not a template to copy directly.

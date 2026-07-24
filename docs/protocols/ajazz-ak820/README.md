# Ajazz AK820 — protocol notes

Status: solid-color RGB control works for real, validated against
physical hardware (`pmctl rgb set`). `get_color`, profiles, sleep
timer, and clock sync are still open — see `findings.md`'s known gaps
and `docs/roadmap.md`'s Phase 6 checklist.

## Using RGB control

```
pmctl rgb set ff8800   # RRGGBB hex
```

This needs raw USB access beyond a normal user's default permissions,
because of a device-specific quirk (see `findings.md`'s "root cause"
entries): Linux's `hid-generic` kernel driver silently swallows this
device's Feature-report *writes*, so `opm-transport`'s `LibusbTransport`
detaches the kernel driver and talks raw `libusb` control transfers
instead (see [ADR 0004](../../architecture/decisions/0004-libusb-transport-for-kernel-driver-interference.md)).
This is **not** a general requirement of this project or of `pmctl`
itself — it's this one device's firmware being picky. A driver for a
better-behaved device would just work over the normal `hidraw` path
(`HidTransport`), no `sudo`, no udev rule beyond Phase 2's.

**Permissions**, in the order to try them:
1. Install [`99-ak820-usb.rules`](99-ak820-usb.rules) (`sudo cp` into
   `/etc/udev/rules.d/`, `sudo udevadm control --reload-rules`,
   `sudo udevadm trigger`, then unplug/replug the keyboard) — grants
   non-root access to the raw USB device node. On the machine this was
   developed on, the resulting `uaccess` ACL never actually appeared
   (`getfacl` showed nothing), for reasons not diagnosed — the rule is
   still correct and worth trying on a fresh system.
2. If that doesn't work, run `pmctl rgb set`/`profile` with `sudo`.

**If a run leaves the vendor interface broken** (symptom: `pmctl
discover --verbose` shows only 3 AK820 interfaces instead of 4, or
`pmctl rgb set` fails with "no interface declares the expected vendor
usage pair"): a process that opened `LibusbTransport` exited without
running its `Drop` (a crash, a `SIGKILL`, or — the bug this project hit
and fixed — `std::process::exit()` called before the device was
dropped), leaving interface 3 detached from the kernel driver with
nothing to re-attach it. Fix:
```
# find the interface path — usually <bus>-<port>:1.3 for the AK820;
# confirm with: for i in /sys/bus/usb/devices/*:1.3; do echo "$i"; done
echo -n "<bus>-<port>:1.3" | sudo tee /sys/bus/usb/drivers/usbhid/bind
```
A physical unplug/replug does *not* reliably fix this on its own (seen
in practice) — the manual rebind above does.

## Using lighting effects

```
pmctl lighting modes                                       # list every mode name
pmctl lighting set --mode spectrum
pmctl lighting set --mode breath --color 7c5cff --speed 4
pmctl lighting set --mode rolling --direction right --speed 2
```

Same transport, same permissions caveats as "Using RGB control" above
(`sudo`, or the udev rule). `pmctl rgb set` is now a thin wrapper around
`pmctl lighting set --mode static` — both go through the same
`Static`→`Breath(speed=0)` substitution documented in `findings.md`.

Validated against real hardware across a representative sample of
modes — see `findings.md`'s 2026-07-21 entries.

## Using the sleep timer

```
pmctl sleep presets       # list every preset name
pmctl sleep set 5m
```

Same transport/permissions as above. Ported from gohv's own already-
decoded sleep timer (no new capture needed — see `findings.md`'s
2026-07-21 6c entry) and validated against real hardware.

## Using presets (host-side, not onboard profiles)

```
pmctl preset save gaming --mode spectrum --speed 5 --sleep never
pmctl preset save chill --mode breath --color 7c5cff --brightness 2
pmctl preset list
pmctl preset apply gaming
```

**This is not the keyboard's onboard profile switching** — that's
still unresolved (Phase 6d, parked — see `findings.md`'s 2026-07-23
entry and [ADR 0006](../../architecture/decisions/0006-host-side-presets-not-onboard-profiles.md)).
A preset is a JSON file under `$XDG_CONFIG_HOME/opm/presets/` (falls
back to `~/.config/opm/presets/`) that remembers a lighting effect
and/or sleep-timer setting and re-applies it by calling `pmctl
lighting`/`pmctl sleep` under the hood. It does **not** persist on the
keyboard itself — plug it into a machine without this project
installed and the preset has no effect. `pmctl preset apply` needs the
same device permissions as `rgb`/`lighting`/`sleep`. **Not yet run
against real hardware** — `save`/`list` and the error paths are
tested; `apply` only unit-tested against a fake device so far.

[`examples/presets/`](../../../examples/presets/) has three ready-made
presets (`gaming.json`, `focus.json`, `off.json`) in the exact format
`pmctl preset save` writes — copy one in directly instead of building
it via flags:

```
mkdir -p ~/.config/opm/presets
cp examples/presets/gaming.json ~/.config/opm/presets/
pmctl preset apply gaming
```

## Layout

- `captures/` — raw USB/HID captures (`usbmon`, Wireshark/`usbpcap`
  dumps, etc.). Ignored by git (see root `.gitignore`) because these are
  large binary artifacts; only the analysis derived from them belongs in
  version control.
- `findings.md` — running notes on what's been figured out: report
  descriptors, command byte layouts, known opcodes, open questions.
- `99-ak820-usb.rules` — the udev rule from "Using RGB control" above.

## Capture workflow (Windows VM + Wireshark/USBPcap)

How the byte-level protocol below was actually figured out — useful
reference if a future capability (profiles, sleep timer) needs a fresh
capture, not something you need to redo just to use `pmctl rgb set`.

Capture rig: a Windows VM (via `virt-manager`/`libvirt`/QEMU, on the
same Linux host `opm-core` is developed on) with the AK820 passed
through via USB host-device passthrough, and Ajazz's official
configuration software installed inside the guest.

Considered and rejected: capturing on the Linux host itself via
`usbmon`, which would have let this session's own tooling drive the
capture directly. Hit real friction getting `/dev/usbmon*` device nodes
to appear (`usbmon` kernel module present but not creating them; root
access needed to debug further, which this session's tools can't
provide interactively) — abandoned in favor of the standard, already-
proven-elsewhere approach: capture inside the guest, same as the
original bare-metal-Windows plan this document had before the machine
setup changed to a VM. The trade-off: the maintainer runs the capture
himself inside the guest and brings the decoded bytes back, rather than
this session capturing live.

Decided 2026-07-10, informed by the AK820's real interface shape
already confirmed in Phase 1/2's Findings (see "Hardware identity"
below) rather than guessed at generically.

1. **Pass the AK820 through to the Windows guest**, in `virt-manager`:
   *Add Hardware → USB Host Device*, select the device by its VID:PID
   (`0c45:800a` — it'll show under whatever generic name the local USB-
   ID database resolves that VID to, e.g. `Microdia`/`Vivitar`; that
   name is meaningless noise, same gray-market-VID caveat
   `discovery.md` already documented, not a different device). This
   unbinds the device from the host and hands it to the guest — once
   done, the host no longer sees it as a HID device at all.
2. **Inside the guest: install Wireshark with USBPcap** (Wireshark's
   Windows installer offers USBPcap as a checkbox; no separate
   download) and Ajazz's official configuration software, if not
   already present.
3. **Capture on every `USBPcapN` interface** rather than figuring out in
   advance which root hub the AK820 landed on inside the guest — filter
   after the fact instead.
4. **One isolated action at a time**, ~2-3s of idle between each,
   noting wall-clock time + what was done in a separate notes file.
   Pure solid, saturated test colors (255,0,0 / 0,255,0 / 0,0,255) make
   the RGB bytes unambiguous once decoded. Suggested order: open the
   software (captures startup/handshake traffic) → solid red → solid
   green → solid blue → brightness up one step → change effect mode →
   switch profile → switch back. Start with color + profile — that's
   what `Rgb`/`Profiles` (Phase 3/4) already have real trait shapes for.
5. **Save as `.pcapng`** into `captures/` (git-ignored, stays local —
   and stays inside the guest's filesystem unless deliberately copied
   out, e.g. via a shared folder or USB drive, back to the host where
   this repo lives).
6. **Filter to the AK820**: `usb.idVendor == 0x0c45 && usb.idProduct ==
   0x800a`. If that filter only matches the enumeration/descriptor
   packet (not the later transfer packets), note the `usb.device_address`
   shown there and filter by `usb.device_address == <N>` instead for
   everything downstream.
7. **Read the filtered packets as HID class control requests** — the
   vendor channel almost certainly uses `SET_REPORT`/`GET_REPORT` over
   the control endpoint (the same operations `opm-transport`'s
   `Transport::set_feature`/`get_feature` already implement):
   `bmRequestType 0x21`/`bRequest 0x09` for `SET_REPORT`
   (`bmRequestType 0xA1`/`bRequest 0x01` for `GET_REPORT`), `wValue =
   (report_type << 8) | report_id` (`report_type`: 1=Input, 2=Output,
   3=Feature), `wIndex` = interface number, payload = the report bytes.
   Wireshark's own USB-HID dissector should label these fields by name.
   `URB_INTERRUPT` packets instead of `URB_CONTROL` mean the interrupt
   endpoint is used (`write_output`/`read_input`'s territory), not
   control.
8. **Bring the decoded bytes back**, not the capture file itself: for
   each isolated action, the interface, report ID, and payload hex,
   written up in `findings.md` — the capture file never needs to leave
   the guest if copying it out is inconvenient, only the write-up does.

## Hardware identity

Captured via a throwaway `hidapi`/`hidreport` probe (see
[`docs/architecture/discovery.md`](../../architecture/discovery.md)'s
Findings section for the full analysis; raw report descriptors are not
yet in this directory's `captures/` — only the discovery-level JSON
report lives in
[`docs/inventory/captures/ajazz-ak820-2026-07-09.json`](../../inventory/captures/ajazz-ak820-2026-07-09.json)).

- USB VID:PID: `0x0c45:0x800a` (`0x0c45` is Sonix Technology — the OEM
  controller vendor, not Ajazz; `manufacturer_string`/`product_string`
  report `"SONiX"`/`"AK820"`, not the Ajazz brand).
- 4 HID interfaces on one composite USB device:
  - Interface 0 — boot-protocol keyboard only (`usage 0x01/0x06`).
  - Interface 1 — five top-level usage pairs sharing one interface:
    consumer control (`0x0c/0x01`), system control (`0x01/0x80`), a
    second keyboard usage (`0x01/0x06`), mouse (`0x01/0x02`), and a
    vendor-defined channel (`0xffff/0x01`); multiplexes report IDs
    `1, 2, 3, 5, 6`.
  - Interface 2 — dedicated vendor channel, `usage 0xff68/0x61`.
  - Interface 3 — dedicated vendor channel, `usage 0xff13/0x01`.
- No usable serial number (`Some("")`, empty string).
- `/dev/hidraw0`-`3` are root-only by default on a stock Arch install —
  actual protocol work against this device will need a udev rule, not
  just discovery-level enumeration (which needs no permissions at all).

Three distinct vendor usage pages (one shared with interface 1, two
dedicated) was the headline surprise here — likely separate command
channels for different features. Interface 3 (`0xff13/0x01`) is the one
lighting control uses, decoded in `findings.md`; what interfaces 1 and
2's vendor channels do (macros? the LCD gohv's tool also controls on
the closely related `0x8009` variant?) is still open.

# 0004. Add a `libusb`-backed `Transport` implementation alongside `HidTransport`

Date: 2026-07-20

Status: Accepted

## Context

`docs/protocols/ajazz-ak820/findings.md`'s 2026-07-20 "root cause" entry
documents a real-hardware finding that ADR 0003 didn't anticipate:
`opm-transport::HidTransport` (`hidapi`, Linux `hidraw` backend) can
**read** Feature reports from the AK820 Pro's vendor interface just
fine (Phase 2's original finding still holds), but its **writes** are
silently accepted at the USB level and then never acted on by the
device's firmware. Linux's `hid-generic` kernel driver, still bound to
the interface, is the interference — confirmed by building two
independent, real, community-authored Linux tools for this keyboard
family (credited in full in `findings.md`) and observing that the one
which explicitly **detaches the kernel driver and speaks raw USB
control transfers via `libusb`** is the one that actually changes the
keyboard's lighting; the ones going through the kernel's `hidraw` node
(this project's own `HidTransport`, and a from-scratch `hidapi-hidraw`
C reproduction) do not.

This is a device/firmware quirk, not evidence that `hidraw` is broken
in general — `HidTransport` stays exactly as ADR 0003 designed it, and
is expected to keep working fine for reads on this device and for
devices without this quirk. The problem is narrower: **this project
currently has no way to express "detach the kernel driver and write
over raw `libusb`"** at all.

## Decision

Add a second `Transport` implementation, `LibusbTransport`, to
`opm-transport`, using the `rusb` crate (`rusb = "0.9"`). It implements
the exact same `opm_core::transport::Transport` trait — **no trait
changes needed**; the existing `set_feature`/`get_feature`/
`write_output`/`read_input` signatures already express what a `libusb`
control-transfer-based implementation needs to do.

`LibusbTransport::open` takes `(vendor_id: u16, product_id: u16,
interface_number: u8)` rather than a `hidraw` path string — `libusb`
addresses devices/interfaces differently than the kernel's `hidraw`
nodes, and `Identity`/`Interface` (from `opm-core::identity`) already
carry `vendor_id`/`product_id`/`interface_number`, so no new discovery
work is needed to supply this. On open, it:
1. Finds the matching USB device by VID:PID (first match — same
   "multiple identical devices" gap already flagged elsewhere in this
   project, e.g. `opm-cli`'s `select_device`; not solved here either).
2. Detaches the kernel driver from `interface_number` if one is
   attached, then claims the interface.
3. On `Drop`, releases the interface and re-attaches the kernel driver
   — leaving the device in the same state a `hidraw`-based tool would
   find it in afterward, rather than leaving it permanently detached.

`set_feature`/`get_feature` build the HID class control request
(`bmRequestType`/`bRequest`/`wValue`/`wIndex` — the same shape
`transport.md`'s Findings already documented from real Wireshark
captures) directly via `rusb::DeviceHandle::write_control`/
`read_control`, with no kernel driver in the way.

`HidTransport` is **not removed or deprecated**. Driver crates choose
whichever `Transport` implementation their device needs;
`opm-driver-ajazz-ak820` switches to `LibusbTransport` for its vendor
channel because it needs writes to work, but a future driver for a
better-behaved device can keep using `HidTransport`.

## Consequences

- `opm-transport` now depends on both `hidapi` and `rusb` — two
  transport-library dependencies instead of one. `opm-core` still
  depends on neither (ADR 0002/0003's constraint is unaffected; only
  `opm-transport`'s own dependency footprint grows).
- Raw `libusb` access needs root or a udev rule granting access to the
  `usb` subsystem for the device's VID:PID — a **different, broader**
  permission than the `hidraw`-only `uaccess` rule Phase 2 already
  documented (`TAG+="uaccess"` on `KERNEL=="hidraw*"` doesn't cover
  `SUBSYSTEM=="usb"` nodes). Needs its own udev rule, not yet written.
- Detaching the kernel driver means, for the duration a `LibusbTransport`
  is open, the OS's normal `hidraw`/`evdev` handling of that specific
  *interface* stops (re-attached on `Drop`). For the AK820, this is
  interface 3 — a vendor-only config channel with no keyboard-input
  role, so no keystrokes are lost; a future driver reusing this pattern
  on an interface that *also* carries input reports would need to
  think about that trade-off explicitly, which this ADR does not need
  to resolve now.
- `LibusbTransport` cannot be meaningfully unit-tested without real
  hardware, the same accepted limitation ADR 0003 already noted for
  `HidTransport`.
- Two `Transport` implementations now exist with genuinely different
  operational trade-offs (kernel-driver-friendly reads-only vs.
  kernel-driver-hostile full read/write) and no guidance yet on when a
  future driver should reach for which. Not resolved here — worth
  revisiting once a second real device/vendor exists (see
  `roadmap.md`'s "Later, cross-cutting").
- **A consequence not anticipated when this ADR was written, found
  during implementation**: `Drop`-based kernel-driver re-attachment
  only runs if `Drop` actually runs. `opm-cli`'s existing commands
  (`rgb`/`profile`/`info`) called `std::process::exit()` directly after
  using an opened device — which skips all destructors, including
  `LibusbTransport::drop()` — leaving interface 3 permanently detached
  after every single `pmctl` invocation until a manual driver rebind
  (`echo -n "<bus>-<port>:1.<iface>" | sudo tee
  /sys/bus/usb/drivers/usbhid/bind`). Fixed by restructuring those three
  commands to drop the opened device explicitly before calling
  `std::process::exit()` — see
  `docs/protocols/ajazz-ak820/findings.md`'s final 2026-07-20 entry.
  This is a structural fragility of the kernel-driver-detach approach
  itself: any future call site that exits, panics past an unwind
  boundary that doesn't run destructors, or is killed outright will
  reproduce the same orphaned-interface symptom, and there is no
  code-level guarantee against it — only discipline at each call site.

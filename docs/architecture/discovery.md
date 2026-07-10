# RFC: hardware discovery

Status: Accepted, validated against real hardware (see Findings below),
fully implemented (`crates/opm-discovery`, `pmctl discover` including
`--export`/`--verbose`/exit codes)

Date: 2026-07-09

## Summary

Before OPM can talk to any device it has to find it. This document designs
that first phase in isolation from any specific device or protocol: how a
running OPM instance enumerates the HID devices attached to the machine,
what it can learn about each one *without* speaking its protocol, how it
tells a plain keyboard apart from one with a vendor-specific configuration
channel, and what a future `pmctl discover` command does with all of that.

This is deliberately not about the Ajazz AK820. The AK820 is the hardware
this design will be validated against first (see
[`docs/protocols/ajazz-ak820/`](../protocols/ajazz-ak820/)), but every
question below is asked at the level of "a HID device", not "this
keyboard". If a heuristic only works because of something AK820-specific,
it belongs in the AK820 driver crate, not here.

## Motivation

`docs/architecture/driver-model.md` designs the `Driver`/`Device` traits —
the contract a driver crate implements *once it already knows* which
device it's talking to and has decided to open it. It deliberately leaves
open "what does a driver need to *see* to decide 'this is mine'?" That
question is this document's problem, and it has to be answered before
`driver-model.md` can be finalized, because the shape of the thing a
`Driver::probe()` gets handed *is* the output of discovery.

Discovery also has a second, independent consumer: a human running
`pmctl discover` on hardware OPM has never seen before, trying to figure
out what it is and whether it's worth reverse-engineering. That use case
needs to work with **zero** registered drivers — it's the tool that
produces the raw material `docs/protocols/<vendor>-<model>/` gets seeded
with, and the tool a future contributor runs on their own keyboard to
start a new driver.

## Goals

- Define what information about a connected HID device is obtainable
  without knowing anything about its wire protocol, and where each piece
  of information comes from (OS, `hidapi`, HID report descriptor).
- Define a heuristic for classifying a device (standard keyboard? a
  keyboard with a vendor-specific configuration channel? something else
  entirely?) using only that information.
- Design the behavior, output, and I/O contract of `pmctl discover`,
  including its role as a portable, shareable **report** — not just a
  terminal listing — so a device OPM's maintainers have never physically
  touched can still be investigated from a report someone else ran.
- Decide where discovery logic lives in the crate layout, given that
  `opm-core` is not allowed to depend on a transport library
  ([`overview.md`](overview.md)).
- Define how discovery results get written down so the project builds a
  durable, cross-device inventory instead of re-deriving this knowledge
  by hand for every new device.
- Establish `Identity` as this phase's contribution to the shared
  vocabulary in [`domain-model.md`](domain-model.md) — the piece every
  later phase (`Transport`, `Driver`, `Capabilities`, `Protocol`) builds
  on.

## Non-goals

- Parsing individual HID report *fields* (sizes, offsets, logical
  ranges) or any byte-level protocol content. That's
  `docs/protocols/<vendor>-<model>/`'s job, and it starts only after a
  device has been classified as worth investigating.
- Opening a device for sustained I/O, sending any report, or reading
  device state (battery, current profile, etc.). Discovery only
  *enumerates*; it never *talks*.
- Hotplug / background watching. `pmctl discover` is a one-shot snapshot,
  like `lsusb`. A `pmctl watch`-style daemon mode is a plausible future
  feature but is out of scope here.
- Non-HID peripherals (e.g. a mouse that only exposes a vendor USB
  bulk/interrupt interface with no HID interface at all). OPM starts with
  HID-class devices; revisit if a real device forces the question.
- Finalizing the `Driver`/`Device` traits themselves — this document
  feeds `driver-model.md`, it doesn't replace it.

## Success criteria

This phase is "done" when:

1. A reader can point at any connected HID device and know, on paper,
   every field OPM could theoretically inspect about it, which layer
   (OS / `hidapi` / report descriptor) it comes from, and how reliable it
   is expected to be.
2. The classification heuristic is written down concretely enough that
   validating it against the real AK820 (next phase) is a matter of
   running `pmctl discover` and checking whether reality matches the
   prediction — not inventing the heuristic on the spot.
3. `pmctl discover`'s behavior, flags, output format, and exit codes are
   specified well enough that someone could implement it without asking
   follow-up design questions (they may still hit *technical* surprises —
   that's expected, see "Risks").
4. The crate-layout question ("where does discovery code live, given
   `opm-core` can't depend on a transport library?") has an answer,
   recorded as an ADR.
5. There's a place and a format for writing down what gets learned once
   real hardware is actually probed — this document defines the
   container; it doesn't fill it, since no hardware has been touched yet.

## Research questions

Each question below is answered as far as it can be from documentation
and first principles (`hidapi`'s own docs, the Linux HID subsystem, the
USB HID Usage Tables spec). None of this has been checked against a real
device yet — that validation is explicitly the next phase, not this one.
Where the answer depends on hardware behavior no reference documents it,
it's marked **open — needs real hardware**.

### How to detect compatible devices?

"Compatible" is a driver-level question, not a discovery-level one.
Discovery produces an `Identity` per candidate device (vendor id,
product id, interfaces, ...); *matching* that `Identity` against a
registered `Driver` to decide "something in this binary knows how to
open this" is what `DriverRegistry` (see `driver-model.md`) does with
discovery's output. Discovery itself has no concept of "supported" —
only "detected".
This split matters: `pmctl discover` must work identically whether zero
or a hundred drivers are linked in.

### How to identify manufacturers?

Two independent, both-unreliable sources, used together:

- **USB-IF vendor ID.** The `vendor_id` field is a lookup key into the
  community-maintained [USB ID
  database](https://www.linux-usb.org/usb-ids.html) (packaged on most
  Linux distros as `/usr/share/misc/usb.ids` on the `usb.ids`/`hwdata`
  package, also available as a Rust crate). Gives a manufacturer name for
  registered vendors. **Known gap:** many small/gray-market keyboard
  vendors (plausibly including Ajazz) don't register their own vendor id
  and instead ship hardware built around a generic OEM controller chip
  (e.g. Sonix, Holtek), so the vendor id in the database may resolve to
  the chip maker, not the brand printed on the keyboard. Confirm which
  case the AK820 is once it's in hand — this changes whether VID alone is
  ever a safe signal for "is this an Ajazz product."
- **`manufacturer_string` / `product_string`.** Free-text USB string
  descriptors the device itself reports. Often accurate for reputable
  vendors, frequently generic ("USB Gaming Keyboard", or empty) for
  budget hardware. Treat as a display hint, never as a matching key.

**Conclusion:** manufacturer identification for this market segment
cannot rely on the vendor-id database alone. The project needs its own
`docs/inventory/` catalog (see below) mapping observed
`(vendor_id, product_id)` pairs to real product names, maintained
empirically rather than looked up.

### How to classify devices?

See "Classification heuristic" below — it's substantial enough to get
its own section.

### What information can I obtain without knowing the protocol?

Everything in the "Layers of information" table below. In short:
identity (vendor/product id, strings, serial), topology (how many HID
interfaces, which ones share a physical device), and *declared* usage
(each interface's usage page/usage and the set of report IDs it defines)
— but never what those reports *mean*.

### How to determine what HID interfaces a device has?

A single physical keyboard commonly enumerates as **multiple** HID
interfaces on one USB composite device. `hidapi`'s `enumerate()` doesn't
even return one entry per interface: since libhidapi 0.13, the Linux
hidraw backend emits one `DeviceInfo` **per top-level collection** (i.e.
per usage pair) in a device's report descriptor — a single interface
that declares two top-level collections (not unusual: a keyboard
interface with a companion consumer-control collection bolted on) yields
two entries sharing the same `path` and `interface_number`. Grouping raw
entries back into "this is one keyboard" is therefore a two-step
problem, resolved as follows (this is a decision, not an open question,
though it should still be checked against the real AK820):

1. **Dedupe by `path`** first. Entries sharing a `path` are the same
   interface with multiple top-level collections, not separate
   interfaces — collapse them into one interface record carrying the
   *set* of usage pairs it declares.
2. **Group interfaces into one physical device by OS topology, not by
   serial number.** On Linux: resolve each hidraw `path`'s sysfs parent
   USB device (`/sys/class/hidraw/hidrawN/device/..`, walked up to the
   enclosing `usb_device` node — a few `read_link`/`canonicalize` calls
   via `std::fs`, no extra crate needed for something this mechanical)
   and use that resolved path as the grouping key. This is the primary
   strategy, not a fallback: it's always available and immune to both
   missing *and duplicated* serial numbers (see next point). Same idea
   on macOS (IOKit registry parent) and Windows (device instance id
   hierarchy), differently shaped — deferred with the rest of
   cross-platform support.
3. **`serial_number` is recorded as descriptive metadata on the
   resulting `Identity`, never used as the grouping key.** It was
   originally considered the reliable signal and topology the fallback;
   that's backwards. Budget HID controllers are known to ship a single
   hardcoded serial string identical across every unit that chip
   produces — using it for grouping would silently *merge* two distinct
   physical devices plugged in at once, which is a worse failure than
   the "no serial" case topology already handles correctly.

**Still open — needs real hardware.** Whether the AK820 exposes a
(unique, or hardcoded, or absent) serial number at all is unknown until
captured — informative for `docs/inventory/`, but no longer something
the grouping algorithm's correctness depends on.

### How to differentiate a standard keyboard from a configurable device?

By interface topology plus usage pages — see "Classification heuristic".
The short version: a device is a **configurable** keyboard, not just a
keyboard, if in addition to the standard boot-keyboard interface it
exposes at least one interface under a vendor-defined usage page. That
extra interface is the channel the vendor's own Windows software uses for
RGB/macros/profiles — and the one a future driver crate would need to
reverse-engineer.

### What information does hidapi provide?

Per enumerated interface (`hidapi::DeviceInfo`, both the Rust crate and
the underlying C API expose the same fields):

| Field | Meaning |
|---|---|
| `vendor_id`, `product_id` | USB VID/PID (u16 each) |
| `serial_number` | Optional; often absent on budget hardware |
| `release_number` | `bcdDevice` — device/firmware version, vendor-defined meaning |
| `manufacturer_string`, `product_string` | Optional free-text USB string descriptors |
| `usage_page`, `usage` | Top-level HID usage of *this interface* — see below |
| `interface_number` | USB interface number (composite devices only; `-1` where the concept doesn't apply) |
| `path` | Opaque, OS-specific handle used to actually open the device later — not stable across reboots on some platforms |
| `bus_type` | `Usb` / `Bluetooth` / `I2c` / `Spi` / `Unknown` (newer `hidapi` versions) |

Critically, `usage_page`/`usage` come from parsing the interface's HID
report descriptor — on Linux this only works through `hidapi`'s
**hidraw** backend, which reads the descriptor straight from the
world-readable `/sys/class/hidraw/hidrawN/device/report_descriptor`
sysfs file (no `ioctl`, no elevated privileges — good news: this means
usage-page/usage data, and report-ID counts below, are available even
on a machine with no udev rules installed at all). The alternative
**libusb** backend leaves both fields zeroed by default, and — the
stronger reason to avoid it, beyond blank fields — *opening* a device
through it detaches the kernel's HID driver first, which stops the
keyboard from typing while OPM holds it open. Decision: pin the
`hidapi` crate (2.x) with its `linux-static-hidraw` feature, which is
already that crate's default on Linux — this needs to be verified to
stay the default across upgrades, not switched to `linux-native` (a
newer, pure-Rust hidraw backend the crate also offers) without
re-checking that it enumerates top-level collections the same way.

### What information does the operating system provide?

Beyond what `hidapi` hands back directly (Linux specifics; macOS/Windows
equivalents are conceptually similar but structurally different and are
themselves **open — needs research when those platforms are targeted**,
see `overview.md`'s "Later" roadmap item):

- **sysfs topology** — `/sys/class/hidraw/hidrawN` symlinks back through
  the USB device tree, which is how sibling interfaces of the same
  physical device get discovered when there's no serial number (see
  above).
- **udev properties** — `ID_VENDOR`, `ID_MODEL`, `ID_VENDOR_ID`,
  `ID_MODEL_ID`, `ID_SERIAL`, `ID_USB_INTERFACE_NUM`. Largely redundant
  with `hidapi`'s own fields, but a second, independently-sourced copy —
  useful for cross-checking and for udev-rule authoring later (device
  permissions, not in scope here).
- **Permissions.** `/dev/hidrawN` is typically not world-readable by
  default; access usually needs a `plugdev`-style group membership or a
  udev rule. `hidapi::enumerate()` itself does not open anything, so
  listing devices works without any special permission — but a `discover`
  run as an unprivileged user with no udev rule installed may still show
  devices it could never actually open. `discover` must report this
  distinction ("detected" vs "detected, not accessible") rather than
  fail or silently omit the device.
- **evdev vs hidraw duality.** A standard boot-keyboard interface is
  bound by the kernel's generic HID driver and shows up *both* as a raw
  `hidraw` node **and** as an `evdev` input device (`/dev/input/eventN`)
  that the desktop environment actually reads keystrokes from. A
  vendor-specific configuration interface (unrecognized usage page) gets
  **no** `evdev` binding — it only exists as `hidraw`. This asymmetry is
  itself a classification signal, independent of usage page: "this
  interface has no evdev sibling" correlates with "this is a vendor
  channel, not a keyboard-typing channel." Worth checking as a
  cross-validation, not the primary signal (usage page is more direct
  and more portable).

### What information belongs to the HID descriptor?

The report descriptor is the boundary this document draws around itself.
It defines, per interface: one or more **collections** (Application /
Logical / Physical), each with a **usage page + usage**; one or more
**report IDs**, each multiplexing a distinct input/output/feature report
on the same interface; and, within each report, a sequence of fields with
bit sizes and logical ranges. Discovery reads only the outermost layer —
the top-level collections' usage pages/usages (which `hidapi` already
surfaces, one `DeviceInfo` entry per collection — see above) and the
*set* of report IDs declared (which it doesn't — that requires parsing
the raw descriptor bytes, read from the same sysfs `report_descriptor`
file `hidapi` itself reads). Decision: depend on the
[`hidreport`](https://crates.io/crates/hidreport) crate (plus its
companion [`hut`](https://crates.io/crates/hut) for human-readable usage
names) to do that parsing, rather than hand-rolling a descriptor scanner
— it's a small, purpose-built, actively maintained parser (minimal
dependencies of its own) for exactly this format, and the report
descriptor grammar has enough edge cases (push/pop, global vs. local
items, delimiters) that re-implementing it for a "just count report IDs"
use case is the wrong place to spend effort. Anything past "how many
report IDs, and how big are they" — i.e. what any given byte in a report
actually *means* — is protocol reverse-engineering, and belongs in
`docs/protocols/<vendor>-<model>/`, not here.

## Layers of information — summary

| Layer | Gives you | Requires | Reliability |
|---|---|---|---|
| `hidapi` enumerate | VID/PID, strings, serial, usage page/usage, interface #, path | Nothing (no open, no permissions — reads sysfs) | High for VID/PID; low for strings/serial |
| OS (udev/sysfs on Linux) | Topology (grouping interfaces into one physical device), permission state | Nothing to read (`std::fs` symlink walk); udev rule to *open* later | High for topology; permission varies by system config |
| HID report descriptor (report-ID level only) | Count and size of report IDs per interface | Reads the same sysfs `report_descriptor` file; parsed via the `hidreport` crate | High (declared by the device itself) |
| HID report descriptor (field level) | Byte-level meaning of each report | Protocol reverse-engineering | Out of scope for discovery |

## Classification heuristic

Given one grouped candidate device (one or more interfaces, each
carrying one or more usage pairs, sharing an `Identity` per the grouping
algorithm above), classify it by walking every usage pair it declares —
regardless of whether those pairs came from separate interfaces or from
multiple top-level collections on the same one:

1. **Standard keyboard signal:** a usage pair `usage_page == 0x01`
   (Generic Desktop), `usage == 0x06` (Keyboard) — the usage a
   boot-protocol-capable keyboard interface declares.
2. **Consumer-control signal (common, not decisive alone):** a usage
   pair `usage_page == 0x0C` (Consumer) — media keys. Present on almost
   any modern keyboard, standard or not; doesn't imply configurability.
3. **Vendor-channel signal:** a usage pair with `usage_page` in the
   vendor-defined range (`0xFF00`–`0xFFFF` per the USB HID Usage Tables
   spec, which reserves that whole block for vendor use). This is the
   strong signal: a device with **both** signal 1 and signal 3 is a
   keyboard with a proprietary configuration channel — i.e. exactly the
   category `pmctl discover` most wants to flag, because it's a candidate
   for a new driver. Note this vendor usage pair may live on its own
   dedicated interface, or share an interface with signal 1 — both are
   real patterns in commodity keyboard controllers; the heuristic doesn't
   care which.
4. **Multiple report IDs on the vendor usage pair's interface** (from the
   report descriptor, via `hidreport` — not just `hidapi`) — a weak
   secondary signal that the vendor channel multiplexes several commands,
   worth noting for whoever starts reverse-engineering it, but not part
   of the classification decision itself.

Resulting categories `discover` assigns: `Keyboard` (signal 1 only),
`Configurable Keyboard` (signals 1 + 3), `Unknown HID` (no interface
matches a page OPM has an opinion about — mice, headsets, and anything
else fall here until this heuristic grows dedicated rules for them), and
`Vendor-only` (signal 3 with no signal 1 — e.g. a bare configuration
dongle with no keyboard interface at all).

This is a heuristic, not a proof — it says "worth investigating", not
"definitely has RGB/macros". It is expected to need adjustment the moment
it's run against a real device; that adjustment is exactly what the next
phase (running this against the AK820) is for.

## The `pmctl discover` command

### Behavior

1. Enumerate all HID interfaces currently visible to the OS.
2. Group interfaces into candidate devices (see grouping discussion
   above).
3. Classify each candidate device (see heuristic above).
4. Cross-check each candidate against `opm-core`'s `DriverRegistry`:
   does any linked-in driver claim this VID/PID? (Zero drivers exist
   today, so this always reports "unsupported" for now — the command
   must still be fully useful with an empty registry.)
5. Print a result. Never sends a report or reads/writes device state,
   never blocks waiting for hardware to appear. The one narrow exception
   to "never opens a device" is the accessibility check (see below),
   which performs a zero-I/O open-then-immediately-close.

### Exit codes

| Code | Meaning |
|---|---|
| `0` | Ran successfully. Includes the case where devices were found but some are inaccessible — that's reported per-device in the output, not a process failure. |
| `1` | The enumeration backend itself failed to initialize (e.g. `HidApi::new()` returned `Err` — no HID subsystem available at all). |
| `2` | Invalid invocation (bad flags/arguments) — the standard `clap` usage-error convention, not custom-handled. |

### Accessibility check

Per-device `ok` / `permission denied` (used in both the default output
and `--export`) is determined by attempting to open the device's path
read-only and immediately closing it, without issuing any read or write
— not by inspecting file permission bits directly. A `stat`/`access(2)`-
only check would misreport devices made accessible via a udev-installed
ACL (`setfacl`), which is common and not visible in the mode bits alone;
an open-then-close is the only check that matches what actually
happens when a driver later tries to use the device. This is the one
narrow, explicitly-documented exception to "discovery never opens a
device" — it transfers zero bytes and holds the handle for no longer
than the check itself.

### Output — default (human-readable)

One row per candidate device: manufacturer (best-effort name),
product (best-effort name), VID:PID, interface count, classification,
driver status (`supported` / `detected, unsupported`), accessibility
(`ok` / `permission denied` — see permissions above). Devices that
`opm-core` already has a working driver for are shown too, not hidden —
`discover` is a superset of what `list` shows (see "Relationship to
other commands" below).

### `--verbose` / `-v`

Expands each device into its constituent interfaces: interface number,
usage page/usage (with the well-known name where OPM recognizes it, e.g.
"Generic Desktop / Keyboard", raw hex otherwise), report ID count, OS
path, raw manufacturer/product strings before any cleanup.

### `--export <format>` — the discovery report

`pmctl discover --export json` is the command's most consequential
feature, and the reason this design treats `discover` as more than a
`lsusb`-alike. It writes a self-contained **report** — not just a device
dump — meant to leave the machine it was captured on: pasted into a
GitHub issue, attached to a PR against `docs/inventory/`, or sent to a
maintainer who doesn't own the hardware in question. That portability
requirement shapes what the report contains:

- **Host context** — OS name/version, kernel version, CPU architecture,
  OPM version, and which `hidapi` backend was used (hidraw vs. libusb;
  see above — this materially affects whether `usage_page`/`usage` are
  even populated, so a report without it is hard to interpret).
- **Per-device data** — everything `-v` shows: identity fields,
  interface list, usage page/usage (decoded name where known), report ID
  counts, `hidraw` path, accessibility. Plus a report-local identifier
  (e.g. an index) so devices and their interfaces can be cross-referenced
  within the same file without relying on the OS path, which isn't
  portable across machines.

By default, `--export` writes to an auto-named file
(`opm-discover-report-<YYYY-MM-DD>.json` in the working directory);
`--output -` writes to stdout instead, for piping.

**Privacy default: `serial_number` is redacted unless `--include-serial`
is passed.** A report is meant to be pasted somewhere public. A serial
number is rarely needed to identify a *model* (VID/PID/usage already do
that) and is the one field in the whole report that could identify one
specific physical unit rather than "a AK820" in general — there's no
reason to default to including it in something designed to leave the
owner's machine.

**Where this is headed, not a Phase 1 deliverable:** the report schema
is deliberately kept complete enough (see `--verbose`'s field list above)
that it could, later, seed a driver crate's scaffolding — a
`Driver::probe()` matcher written from a report someone else submitted,
for hardware no maintainer owns. That's explicitly out of scope for this
phase (see `roadmap.md`'s "Explicitly not planned right now") — it's
recorded here only so the report's shape isn't accidentally too
lossy to support it once it's actually attempted.

#### Schema (draft)

Every report carries a `schema_version` from day one — a git-tracked,
long-lived, externally-consumed format with no version field has no way
to evolve without silently breaking old captures. Policy: additive
changes bump the minor number; consumers (including a future driver
scaffolder) must ignore fields they don't recognize rather than fail on
them.

```jsonc
{
  "schema_version": "1.0",
  "opm_version": "0.1.0",
  "host": {
    "os": "linux",
    "os_version": "...",       // e.g. distro name/version, best-effort
    "kernel_version": "...",   // uname -r equivalent
    "arch": "x86_64",
    "hidapi_backend": "linux-static-hidraw"
  },
  "devices": [
    {
      "local_id": 0,             // report-local, stable only within this file
      "vendor_id": "0x0c45",
      "product_id": "0x8508",
      "manufacturer_string": "...",
      "product_string": "...",
      "serial_number": null,    // redacted by default, see above
      "classification": "configurable_keyboard",
      "classified_by": "0.1.0", // heuristic/OPM version that produced this
      "driver_status": "unsupported",
      "interfaces": [
        {
          "interface_number": 2,
          "path": "/dev/hidraw3",
          "accessible": true,
          "usage_pairs": [{ "usage_page": "0xff00", "usage": "0x01" }],
          "report_ids": [1, 2]
        }
      ]
    }
  ]
}
```

Every `devices.md`/`captures/` entry inherits `classified_by` from its
capture, so a later change to the classification heuristic doesn't
silently make old rows look authoritative for a verdict a newer OPM
would no longer reach — see `docs/inventory/README.md`.

### What it explicitly does not do

- Does not persist anything to disk **unless `--export` is passed** —
  the default (no flags, `-v`) is pure stdout, like `lsusb`. A machine
  running bare `discover` twice in a row should never behave differently
  the second time because of the first run; `--export` is the one
  explicit, opt-in exception, and it writes a new file rather than
  mutating any existing state.
- Does not dump raw report descriptor bytes by default (only report-ID
  *counts*, under `-v`) — full descriptor bytes are a `docs/protocols/`
  concern, captured with `usbmon`/Wireshark per that directory's existing
  workflow, not something `discover` reinvents.
- Does not watch for hotplug events or loop; one process, one snapshot,
  exits.
- Does not require root or any udev rule to run meaningfully — it must
  degrade gracefully (report "permission denied" per-device) rather than
  fail outright on a machine with no rules installed, since that's the
  expected out-of-the-box state for a new contributor.

### Relationship to `list` and `info`

The existing stub for `pmctl list` (`crates/opm-cli/src/commands/list.rs`)
describes it as showing peripherals "recognized by any registered
`opm-core` driver" — i.e. the supported subset only, for end users who
just want to know "can OPM control any of my hardware right now."
`discover` is the broader, developer/contributor-facing tool: every HID
device, supported or not, with enough raw detail to start a new driver.
Concretely: `list` is `discover` filtered to `driver status == supported`
and rendered without the raw usage-page/interface detail end users don't
need. This should be revisited once `list` is actually implemented, to
decide whether `list` calls into the same underlying enumeration or
whether `discover` is later reimplemented in terms of `list`'s data —
but the two must not duplicate separate enumeration logic.

## Where this lives in the architecture

`opm-core`'s real constraint (`overview.md`) is narrower than "zero
dependencies forever": it must never depend on a transport library
(`hidapi`, `libusb`, `udev`, ...) or know about any specific vendor — but
ordinary general-purpose crates are fine. Discovery fundamentally needs
a transport library to enumerate anything, which forces the open
question `overview.md` already flagged ("whether HID access is its own
crate... is an open question... to answer once the first driver is being
written") to be answered now, since discovery needs it before any driver
exists.

Decision, recorded as [ADR
0002](decisions/0002-discovery-lives-outside-opm-core.md): the plain-data
type describing a detected-but-unopened HID interface — this is the
`Identity` facet, see below — is defined **in `opm-core`** itself,
dependency-free (aside from general-purpose crates like `serde`, needed
for `--export`). `opm-discovery` (`crates/opm-discovery`) — enumeration,
grouping, classification logic — lives outside `opm-core` and **depends
on it** to produce `Identity` values, using `hidapi` internally to do so
— not the other way around. `opm-cli` depends on `opm-discovery`
directly for `pmctl discover`, without going through `opm-core`'s
`DriverRegistry`. `opm-core` itself still never depends on `hidapi`.

`Identity` is **not** the `Device` from `driver-model.md`. An `Identity`
is inert data about an unopened interface; a `Device` is a live handle to
something a driver has already opened and taken responsibility for.
Discovery produces `Identity` values and hands them to drivers (via
`DriverRegistry`); a driver that recognizes one hands back a `Device`.

In [`domain-model.md`](domain-model.md)'s vocabulary, `Identity` is
deliberately the only facet a `Driver` needs in order to decide "mine"
(via `Driver::probe()`), before it has opened any `Transport` or knows
anything about the device's `Protocol`.

## Testing strategy

None of grouping, classification, or `--export` serialization should
need physical hardware to test. That only holds if the discovery crate
is structured with a hard seam: a thin adapter that calls `hidapi` (plus
sysfs/`hidreport` reads) to produce raw `Identity`/interface data, and
everything else — deduping by path, topology-based grouping, the
classification heuristic, report assembly — written as pure functions
over that already-serializable data. With that split:

- Every file under `docs/inventory/captures/` doubles as a regression
  fixture: deserialize it, run it through grouping/classification, assert
  the result matches the row recorded in `devices.md`. This is the same
  data the community-contribution workflow already produces — no
  separate test-fixture format to maintain.
- Deliberately adversarial fixtures are cheap to hand-write once the
  first real capture exists: two devices with identical hardcoded
  serials (tests point 3 of the grouping algorithm), a descriptor with a
  vendor usage pair sharing an interface with the keyboard usage pair
  (tests the classification heuristic's collection-level matching), a
  malformed/truncated report descriptor (tests that `hidreport` parse
  failures are handled, not just the happy path).
- The one thing that *can't* be unit-tested this way is the adapter
  itself (does `hidapi::HidApi::new()` actually behave as expected on a
  real Linux machine) — that stays a manual, real-hardware check, same
  as the rest of this document's **open — needs real hardware** items.

## Experimentation methodology

Once implementation starts (next phase, not this one), the process for
learning anything new via discovery is:

1. Run `pmctl discover --export json` against the device in hand, save
   the report under `docs/inventory/captures/<vendor>-<model>-<date>.json`.
2. Compare the result against this document's classification prediction.
   Where it matches: good, move on. Where it doesn't: that's a finding.
3. Write the finding up in `docs/inventory/devices.md` (the durable,
   cross-device summary — one row) and, if it's substantial enough to
   change a heuristic or assumption in this document, as a dated entry
   appended to a "Findings" section added to this file at that point
   (mirroring `docs/protocols/<device>/findings.md`'s format, but for
   discovery-level findings rather than protocol-level ones).
4. If a finding invalidates something this document currently states as
   settled (e.g. "the AK820's serial number is hardcoded/identical across
   units, so grouping must rely on topology, not serial, exactly as
   designed" — confirming, not contradicting, the current design — or
   something that genuinely does contradict it), update this document in
   the same change — this file is a living design doc, not a snapshot.

### Report template for a discovery finding

```
## YYYY-MM-DD — <device> — <short title>

Command run: `pmctl discover --export json` (or manual hidapi probing,
if `discover` isn't implemented yet)
Capture: docs/inventory/captures/<file>.json

What was expected (per this document's heuristic):
...

What was observed:
...

Conclusion / what changes as a result:
...
```

## Findings

### 2026-07-09 — Ajazz AK820 (Pro) — first real capture

Command run: throwaway `hidapi` + `hidreport` probe scripts (`opm-discovery`
doesn't exist yet).
Capture: [`docs/inventory/captures/ajazz-ak820-2026-07-09.json`](../inventory/captures/ajazz-ak820-2026-07-09.json)

**What was expected:** a keyboard interface (usage 0x01/0x06) plus at
least one vendor-defined interface (0xFF00–0xFFFF), classifying as
`Configurable Keyboard`; grouping to rely on topology since serial was
expected to be unreliable; `hidreport` to parse the descriptor cleanly.

**What was observed:** confirmed, and richer than predicted.

- 4 hidraw interfaces (0-3), all sharing one sysfs parent USB device
  (`.../usb3/3-1`) — topology-based grouping worked exactly as designed.
- Interface 0: single collection, usage `0x01/0x06` (keyboard), no
  report ID.
- **Interface 1 alone declares five top-level usage pairs** sharing one
  `hidapi` `path`/`interface_number` — richer than the "an interface with
  two collections" case this document anticipated: `0x0c/0x01`
  (consumer), `0x01/0x80` (system control), `0x01/0x06` (a *second*
  keyboard usage, distinct from interface 0's), `0x01/0x02` (mouse!), and
  `0xffff/0x01` (vendor). It multiplexes 5 distinct report IDs
  (`1,2,3,5,6`) across those.
- Interfaces 2 and 3 are each a single, dedicated vendor-defined usage
  pair (`0xff68/0x61` and `0xff13/0x01` respectively) — i.e. **three**
  vendor usage pages across this device (one shared with interface 1,
  two standalone), not the single vendor channel the design's examples
  implied. Likely separate command channels (macros/RGB/profiles?) —
  irrelevant to discovery, a Phase 6 question.
- Classification: `Configurable Keyboard` (signals 1 + 3 both present).
  **Matches prediction.**
- `serial_number` was `Some("")` — present but empty, a third case
  beyond the plain "absent vs. present" this document discussed.
  Functionally equivalent to absent for grouping purposes: it does
  **not** contradict the decision to group by topology rather than
  serial, if anything it reinforces it (an empty string is an even more
  obvious collision risk across units than a hardcoded non-empty one).
- `manufacturer_string` was `"SONiX"`, not `"Ajazz"` — confirms the
  manufacturer-identification finding exactly as reasoned: VID `0x0c45`
  and the device's own string both resolve to the OEM controller vendor
  (Sonix Technology), not the brand printed on the keyboard. VID/PID
  alone will not be a safe "is this an Ajazz product" signal if Sonix
  ships the same controller under other brands.
- `/dev/hidraw0`-`3` were all `root:root`, mode `0600` — opening any of
  them as the logged-in user failed with permission denied, no udev rule
  installed on this machine. Confirms the accessibility-check design
  (report "detected, not accessible" rather than fail) reflects a real,
  common, out-of-the-box condition, not a hypothetical.
- `hidreport` parsed all four descriptors without error on the first
  try. One parsing-script caveat, not a finding about the hardware: a
  naive walk of *every* collection attached to a field (rather than only
  top-level `Application` collections) also surfaces nested `Logical`
  collections (e.g. a `Pointer` usage nested inside interface 1's
  `Mouse` collection) — `opm-discovery`'s real implementation must
  filter to top-level collections specifically to match what `hidapi`
  itself reports, or grouping/classification will see phantom extra
  usage pairs.

**Conclusion:** every heuristic and algorithm decision in this document
held up against real hardware unchanged. No revision needed to the
grouping strategy, the classification signals, or the dependency
choices (`hidapi` 2.x/`linux-static-hidraw`, `hidreport`/`hut`). The one
implementation note worth carrying forward: filter to top-level
collections only when walking `hidreport`'s output.

## Folder structure introduced by this phase

```
docs/
├── architecture/
│   ├── discovery.md                 # this document
│   └── decisions/
│       └── 0002-discovery-lives-outside-opm-core.md
└── inventory/                       # new
    ├── README.md                    # what this folder is, how to contribute to it
    ├── devices.md                   # one row per physical device ever discovered
    └── captures/                    # raw `pmctl discover --export json` reports
```

## Risks and open questions

- **Grouping's topology-first algorithm is confirmed against the AK820
  (see Findings) but only with one physical unit connected.** The one
  scenario it was specifically designed for and still hasn't been
  observed — two identical units, sharing an identical empty/hardcoded
  serial, plugged in at once, correctly grouped as two devices via
  topology rather than merged — needs a second unit to actually test.
- **Gray-market VID/PID reuse — confirmed as a real, not hypothetical,
  concern.** `manufacturer_string` reports `"SONiX"` (the OEM controller
  vendor), not `"Ajazz"`, and VID `0x0c45` is registered to Sonix
  Technology, not Ajazz. If some other rebrand of the same Sonix
  controller shares this exact VID:PID, VID/PID alone can never be a
  safe matching key for `Driver::probe()` — `release_number`, the exact
  set of four interfaces and their usage pairs, or product-string
  matching would have to do more work than usual. Still unknown; would
  need a second Sonix-based keyboard to actually check.
- **hidapi backend and dependency pins need to survive upgrades.** The
  design now names concrete choices (`hidapi` 2.x /
  `linux-static-hidraw`, `hidreport`/`hut` for descriptor parsing) rather
  than leaving them open — the residual risk is purely process: an
  unreviewed `cargo update` silently moving off `linux-static-hidraw` (or
  a hidapi major version changing per-collection enumeration behavior
  again, as it did going into 0.13) would degrade classification with no
  compile-time signal. Worth a comment at the dependency declaration
  site, once code exists, not just a line in this document.
- **Permission variance across distros.** Some distros ship udev rules
  granting `plugdev` access to HID devices by default, others don't.
  `discover`'s output for the same physical hardware may differ between
  machines purely due to this, independent of the hardware itself —
  `discover`'s output must make the permission state explicit per-device
  so this doesn't get mistaken for a hardware or classification
  difference.
- **Wireless/BLE variants.** A device that also has a Bluetooth mode may
  enumerate completely differently (different `bus_type`, possibly
  different PID) than its USB-wired mode. Not a concern for the AK820
  (wired), but worth a one-line flag for whoever tackles the first
  wireless device.
- **False positives from unrelated built-in hardware.** A laptop's
  built-in keyboard is itself a HID device satisfying signal 1
  ("Standard keyboard signal") and will show up in `discover` output
  next to the external device actually being investigated. Not a bug —
  correct behavior — but worth calling out so it isn't mistaken for one
  the first time someone runs the command on a laptop.
- **macOS/Windows discovery is explicitly deferred.** Everything OS-layer
  above is Linux-specific (sysfs/udev/evdev). `overview.md` already lists
  cross-platform transport as a "Later" roadmap item; this document
  doesn't attempt to design for those platforms and shouldn't be read as
  having done so.
- **A shared, public report format is also a shared, public attack
  surface for bad data.** Once `docs/inventory/` accepts community-
  submitted `--export` reports, nothing stops a submitted report from
  being wrong (typos, a misidentified device, a doctored file) or, later,
  from being fed automatically into tooling (e.g. a hypothetical driver
  scaffolder). Any future automation that *consumes* submitted reports
  must treat them as untrusted input, not as ground truth — a concern to
  revisit if and when that automation is actually built, not solved here.

## Artifacts produced by this phase

- This document, plus [`domain-model.md`](domain-model.md) (the
  vocabulary this document's `Identity` concept feeds into).
- [ADR 0002](decisions/0002-discovery-lives-outside-opm-core.md).
- `docs/inventory/` (README, `devices.md`, empty `captures/`).
- Updated `docs/roadmap.md` and `docs/README.md` reflecting the above.
- The first real capture,
  [`docs/inventory/captures/ajazz-ak820-2026-07-09.json`](../inventory/captures/ajazz-ak820-2026-07-09.json),
  and its row in `docs/inventory/devices.md` — the empirical evidence
  this document didn't have when first written (see Findings).

## Conclusions

- Discovery and protocol reverse-engineering are genuinely different
  activities with a clean boundary: discovery stops at "which interfaces
  exist and what do they declare about themselves", protocol work starts
  at "what does a specific byte in a specific report mean." Keeping that
  boundary explicit keeps `docs/protocols/` and this document from
  fighting over the same ground.
- The most consequential design decision wasn't classification (the
  heuristic here is fairly mechanical) — it was **how to group
  interfaces into a physical device**, since everything downstream
  (classification, driver matching, `pmctl discover`'s output shape)
  assumes that grouping already happened correctly. It's settled now
  (topology-first, serial as metadata only, dedupe by path for
  multi-collection interfaces) — but "settled on paper" and "confirmed
  against real hardware" are different things, and only the second one
  actually closes this out.
- `opm-core`'s real constraint — never a transport library, not "zero
  dependencies forever" — is narrower than the project's own docs
  originally implied, and getting that distinction right is what let
  `Identity` live in `opm-core` (ADR 0002) without contradiction. Where a
  general-purpose crate (`serde`) or a purpose-built parser (`hidreport`)
  clearly does the job better than a hand-rolled version, this design
  now takes that dependency rather than reinventing it — the zero-deps
  framing was never meant to be an argument for doing everything from
  scratch.

## Next steps (feed into `docs/roadmap.md`)

- [x] Get the AK820 enumerating via `hidapi` (a throwaway script is
      enough — doesn't need to be `pmctl discover` yet) and capture the
      raw output. This is the first real test of every heuristic above,
      including the newly-concrete grouping algorithm. See Findings.
- [x] Confirm the `hidapi` 2.x / `linux-static-hidraw` pin behaves as
      documented on the maintainer's actual Linux distro/kernel, and that
      `hidreport` parses the AK820's real report descriptor cleanly. Both
      confirmed; note the top-level-collections-only filtering caveat in
      Findings.
- [x] Fill in `docs/inventory/devices.md`'s first row (the AK820) from
      that capture, stamped with the `classified_by` version.
- [x] Implement the `opm-discovery` crate and wire up `pmctl discover`'s
      default output, with grouping/classification as pure functions per
      "Testing strategy" above — run against the real AK820, matching
      the throwaway-script findings exactly (see devlog).
- [x] `--export`, `--verbose`, and the exit-code table this document
      designed — implemented and run against the real AK820 (see
      devlog). Phase 1 is complete as designed.

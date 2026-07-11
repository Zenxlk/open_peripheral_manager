# RFC: HID transport abstraction (Phase 2)

Status: Accepted, implemented, validated against real hardware (see
Findings below).

Date: 2026-07-10

## Summary

`opm-discovery` (Phase 1) enumerates HID devices and tells you which
interface a candidate device's vendor channel lives on — but it only
ever reads (report descriptors, sysfs) or does a zero-I/O open-then-close
check. Nothing in the codebase can yet open a device and actually
exchange bytes with it. This document designs that layer: a
vendor-agnostic `Transport` trait that opens one HID interface and moves
Input/Output/Feature reports across it, on top of which Phase 3's
`Driver`/`Device`/`Capability` machinery and Phase 6's real AK820
protocol will eventually be built.

This is deliberately not about the AK820's protocol. Nothing here knows
what any byte *means* — see `domain-model.md`'s `Transport`/`Protocol`
split. A `Transport` moves opaque report buffers; a driver's internal
`Protocol` module (Phase 6) is what knows report `0x05`'s third byte is
an RGB channel.

## Motivation

`domain-model.md` names `Transport` as the second facet, public in
`opm-core` as "a trait/abstraction — but its shape is undesigned." Phase
3 (`driver-model.md`) can't finish `Driver`/`Device` without knowing what
a `Device` actually holds onto to talk to its hardware, and Phase 4 (the
first real driver, even with stub capabilities) can't open the AK820 at
all without this. The roadmap deliberately held this open until Phase 1
was validated against real hardware, so this design is informed by an
`Identity` (interfaces, paths, usage pairs) that's now known to actually
match reality, not guessed at.

## Goals

- Define a `Transport` trait: open one HID interface (by the `path` an
  `Identity`'s `Interface` already carries) and read/write reports over
  it, without any driver crate calling `hidapi` directly.
- Decide where the trait lives vs. where its `hidapi`-backed
  implementation lives, following the same constraint ADR 0002 already
  established (`opm-core` never depends on a transport library).
- Make protocol code (Phase 6) and capability code (Phase 3) testable
  without physical hardware, by making `Transport` a trait a test can
  fake, not a concrete `hidapi` type baked into `Driver`/`Device`.
- Decide, concretely, which of HID's report kinds (Input / Output /
  Feature) the trait exposes and how it handles the mandatory
  Report-ID-as-first-byte convention `hidapi` itself uses.

## Non-goals

- Anything protocol-specific (what a report's bytes mean). That's Phase
  6, `docs/protocols/<vendor>-<model>/`.
- The `Driver`/`Device`/`Capability` traits themselves — this document
  feeds `driver-model.md` (Phase 3), it doesn't replace it. `Device`
  holding one or more `Transport`s is assumed here but not designed here.
- Hotplug / reconnect handling. A `Transport` that loses its device mid-
  session surfaces an error on the next call; automatic reconnection (if
  ever wanted) is a `Driver`-level concern, not this layer's.
- Non-HID transports (raw USB bulk/interrupt with no HID interface at
  all). Same non-goal `discovery.md` already carries — revisit only if a
  real device forces the question.
- Cross-platform support. Like `discovery.md`, this is validated against
  Linux/`hidapi` only; macOS/Windows are structurally similar (`hidapi`
  abstracts the OS layer for actual I/O, unlike enumeration's sysfs
  dependency) but unverified.

## Research questions

### Blocking or async?

Blocking. `pmctl` (and, later, driver `Protocol` code) issues short,
synchronous command/response exchanges — send a feature report, read the
reply, done — never a long-lived stream that would benefit from an async
runtime. `hidapi` itself is a blocking C library underneath; wrapping it
in `async fn` would only add a runtime dependency (`tokio`/`async-std`)
to `opm-core` for no real concurrency gain. If OPM ever needs a `pmctl
watch`-style daemon (explicitly out of scope, see `discovery.md`'s
non-goals) that decision gets revisited then, against a real need, not
speculatively now.

### Which report kinds does the trait expose?

HID defines three kinds of report a host can exchange with a device,
and `hidapi` 2.6.6 exposes each with two related methods:

| Kind | Direction | `hidapi` methods | Transport used |
|---|---|---|---|
| Output | host → device | `write()` (interrupt OUT if the device has one, else falls back to the control endpoint), `send_output_report()` (always control endpoint, `Set_Report`) | varies by device |
| Input | device → host | `read()` / `read_timeout()` (interrupt IN) | interrupt |
| Feature | either direction | `send_feature_report()`, `get_feature_report()` | control endpoint |

`get_input_report()` (a fourth method, reading an Input report on demand
via the control endpoint rather than waiting on the interrupt endpoint)
is gated `#[cfg(any(hidapi, target_os = "linux"))]` in the `hidapi` crate
itself — not universally available across its own supported platforms.
Left out of this trait for now; revisit if a real device's protocol
needs it (Phase 6 would surface that need concretely).

**Decision:** expose all three kinds — `write_output`, `read_input`,
`get_feature`/`set_feature` — since which one(s) the AK820's vendor
channel actually uses is unknown until Phase 6, and adding a report kind
later is a breaking trait change every driver crate would have to absorb.
`write_output` maps to `hidapi::write()` (interrupt-preferred), not
`send_output_report()` (control-only) — the common case for a composite
device's dedicated interrupt endpoint; if a real device turns out to need
the control-only variant specifically, that's a concrete, evidence-backed
reason to add it later, not a guess to design in now.

### The Report-ID-as-first-byte convention

Every `hidapi` I/O method that touches Output or Feature reports requires
the caller to prepend the Report ID as the buffer's first byte (`0x00`
for devices with only a single, unnumbered report) — a raw-C-API detail,
not a Rust-idiomatic one. `opm-discovery`'s `Identity.interfaces[].report_ids`
already gives driver code the *set* of valid IDs for an interface; making
every call site manually splice a byte into a buffer (and remember that
`get_feature_report`'s returned length includes that byte, but the
*data* starts at index 1) is exactly the kind of low-level bookkeeping
this abstraction exists to remove.

**Decision:** `Transport`'s methods take `report_id: u8` as an explicit
parameter, separate from the payload slice. The implementation splices
it onto (or strips it from) the buffer internally, matching how
`opm-core::identity` already models report IDs as data, not as a magic
buffer offset. Devices using a single unnumbered report simply pass `0`.

### Timeout policy

`hidapi::HidDevice::read_timeout` takes `-1` (block forever), `0`
(non-blocking, return immediately), or a millisecond count — but
`hidapi` also has a separate, stateful `set_blocking_mode()` that governs
plain `read()`. Carrying both a mutable mode *and* a per-call timeout
argument invites the two disagreeing with each other, and is a bigger API
than needed. **Decision:** `Transport::read_input` takes a `ReadTimeout`
enum argument on every call (see trait below) instead of a stateful mode
switch — no hidden mutable state, no `set_blocking_mode()` call site to
forget. `hidapi`'s "block forever" option is intentionally *not*
reproduced 1:1: a device that stops responding mid-command (unplugged,
firmware wedged) should surface an error, not hang `pmctl` forever, so
`Blocking` in this trait's `ReadTimeout` is a documented, generous, but
finite default (see the trait doc comment below), and callers wanting a
literal indefinite wait must say so explicitly.

### Where does the trait live vs. its implementation?

Same shape as ADR 0002: `opm-core` must never depend on `hidapi`. The
`Transport` trait, its `Error` type, and `ReadTimeout` are plain
signatures with no I/O behavior of their own — they belong in `opm-core`,
so `Device`/`Driver` (Phase 3) can reference `Box<dyn Transport>` without
`opm-core` ever depending on a transport library. The `hidapi`-backed
implementation is a new crate, `opm-transport`, mirroring `opm-discovery`:
depends on `opm-core` (for the trait it implements) and `hidapi`
internally, not the other way around. Recorded as
[ADR 0003](decisions/0003-transport-trait-in-core-impl-in-opm-transport.md).

`opm-discovery` does **not** depend on `opm-transport`, and vice versa —
they solve different problems (zero-I/O enumeration vs. sustained I/O)
against the same underlying `hidapi` library, and discovery's own
non-goals ("never opens a device for sustained I/O") mean it has no
actual need for `Transport`. Some duplication (both wrap
`hidapi::HidApi`/`open_path`) is expected and acceptable, not a smell to
eliminate by merging the crates — `accessible.rs`'s open-then-immediately-
close check has nothing in common with a `Transport`'s read/write
lifecycle.

### Why a trait at all, and not just a `struct HidTransport`?

Two independent reasons converge on the same answer:

1. **`opm-core` can't name a concrete `hidapi`-backed type** without
   depending on `hidapi` (the exact thing ADR 0002 already ruled out for
   the analogous `Identity` question).
2. **Testability.** Phase 3's `Capability` implementations and Phase 6's
   `Protocol` code (e.g. "does sending report `0x05` with byte 2 = `0xFF`
   set the keyboard to red?") need to be testable without a physical
   keyboard plugged into CI. A trait lets driver crates write a
   `FakeTransport` — an in-memory recorder that returns canned bytes —
   and test protocol logic against it. This is `discovery.md`'s "Testing
   strategy" principle (pure logic over a fake/serializable seam) applied
   one layer deeper, at the point where real I/O would otherwise force
   every test to need hardware.

### Per-interface or per-device?

**Decision:** a `Transport` wraps exactly one HID interface (one
`hidapi` `path`, matching one `Identity::Interface`), not a whole
physical device. The AK820 alone has three distinct vendor usage pages
spread across three different interfaces (see `discovery.md`'s
Findings) — a driver that needs all three open at once holds three
`Transport`s, one per path. Bundling multiple interfaces behind a single
`Transport` would hide that multiplicity instead of letting `Device`
(Phase 3) decide how many it needs and why.

### Concurrent / multiple opens of the same path

Linux's `hidraw` doesn't enforce single-writer exclusivity at the kernel
level — nothing stops two processes (or two `Transport`s in the same
process) from opening the same path simultaneously, unlike a serial
port's typical `O_EXCL` convention. This document doesn't add exclusivity
enforcement: it's a real gap (two `Transport`s on the same path could
race), but no evidence yet that OPM itself would ever open the same path
twice within one process — `Driver::open` (Phase 3) is expected to open
each interface exactly once and hand the resulting `Device` to the
caller. Flagged here so it isn't forgotten if that assumption turns out
wrong.

## The `Transport` trait

Lives in `opm-core::transport` (mirrors `opm-core::identity`'s module
shape — plain signatures, no `hidapi` import anywhere in this file):

```rust
/// How long [`Transport::read_input`] waits for an Input report before
/// giving up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadTimeout {
    /// Return immediately with `Error::WouldBlock` if nothing is queued.
    NonBlocking,
    /// Wait up to this many milliseconds.
    Millis(u32),
    /// Wait a long, but finite, default (see the implementation crate for
    /// the exact value) — deliberately not a literal indefinite wait, so
    /// a stalled device can't hang the caller forever. Prefer `Millis`
    /// with a value informed by the device's actual protocol once known.
    Blocking,
}

/// An open channel to exactly one HID interface, able to exchange
/// Input/Output/Feature reports with it. Says nothing about what any
/// report's bytes *mean* — see `docs/architecture/domain-model.md`'s
/// `Transport`/`Protocol` split.
///
/// Implementations open one `Identity::Interface::path`; a physical
/// device with multiple relevant interfaces (common — see
/// `docs/architecture/discovery.md`'s Findings) needs one `Transport`
/// per interface.
pub trait Transport: Send {
    /// Sends an Output report. `report_id` is `0` for interfaces that
    /// don't use numbered reports (see `Identity::Interface::report_ids`).
    /// `data` is the report payload *without* the Report ID byte —
    /// implementations handle that framing internally.
    fn write_output(&self, report_id: u8, data: &[u8]) -> Result<usize, Error>;

    /// Reads the next queued Input report into `buf`, waiting per
    /// `timeout`. Unlike Output/Feature reports, an Input report is
    /// pushed by the device whenever it has one, not requested by ID —
    /// there is no way to ask for a *specific* report ID, only to read
    /// whatever comes next. Returns `(report_id, payload_len)`: which
    /// report ID the device tagged this one with (`0` for unnumbered
    /// reports) and how many payload bytes were written to `buf` (the
    /// Report ID byte itself is not included, mirroring `write_output`).
    fn read_input(&self, buf: &mut [u8], timeout: ReadTimeout) -> Result<(u8, usize), Error>;

    /// Requests a Feature report by `report_id` over the control endpoint.
    fn get_feature(&self, report_id: u8, buf: &mut [u8]) -> Result<usize, Error>;

    /// Sends a Feature report over the control endpoint.
    fn set_feature(&self, report_id: u8, data: &[u8]) -> Result<(), Error>;
}
```

**Implementation note (found while implementing, not while designing):**
the first draft of this trait gave `read_input` a `report_id: u8`
parameter symmetric with the other three methods. That doesn't hold up
against `hidapi`'s actual `read()`/`read_timeout()` semantics — Output
and Feature reports are host-*requested* by ID (`Set_Report`/`Get_Report`
over the control endpoint), but an Input report is *pushed* by the device
over the interrupt endpoint whenever it has one; there is no HID
mechanism to ask for "the next Input report with ID 3" specifically. The
signature above returns the ID the device tagged the report with instead
of accepting one as input — corrected before any code shipped, but
recorded here as exactly the kind of thing `discovery.md`'s own
Findings-section discipline exists for: real implementation catching a
design error the RFC pass didn't.

`open()` is deliberately **not** a trait method — a trait object
(`Box<dyn Transport>`, needed so `Device` can hold one without a generic
parameter) can't have a constructor that returns `Self`. Each
implementation exposes its own inherent constructor instead (e.g.
`opm_transport::HidTransport::open(path: &str) -> Result<HidTransport, Error>`
in the implementation crate), matching how `hidapi::HidApi::open_path`
itself already works.

### Error type

```rust
/// Everything that can go wrong opening or using a `Transport`.
/// Deliberately doesn't wrap `hidapi::HidError` (or any transport
/// library's error type) directly — see ADR 0002/0003 for why `opm-core`
/// can't name that type at all.
#[derive(Debug)]
pub enum Error {
    /// Opening the interface itself failed (missing, permission denied,
    /// already gone). `reason` is the backend's own message, preserved
    /// for display but not pattern-matched on.
    Open { path: String, reason: String },
    /// A read/write/feature-report call failed after a successful open
    /// (device unplugged mid-session, a genuine I/O error).
    Io { reason: String },
    /// `ReadTimeout::NonBlocking` (or a `Millis` timeout) elapsed with no
    /// data available. Not necessarily an error condition for the
    /// caller — polling code is expected to match on this specifically.
    WouldBlock,
}
```

## Crate layout — `opm-transport`

```
crates/
├── opm-core/         # + transport.rs: Transport trait, Error, ReadTimeout
├── opm-discovery/     # unchanged — does not depend on opm-transport
├── opm-transport/     # new: HidTransport, hidapi-backed, depends on opm-core + hidapi
└── opm-cli/
```

Not implemented as part of this document — this is the design, Phase 2's
actual implementation (and running it against the real AK820, opening an
interface and exchanging a first report) is the next step, same
discovery→validation split Phase 1 followed.

## Testing strategy

- `opm-core::transport` (the trait/error/enum definitions) has nothing to
  unit test beyond compilation — it's signatures, not logic.
- `opm-transport`'s `HidTransport` (the real `hidapi`-backed
  implementation) can't be meaningfully unit-tested without hardware,
  same conclusion `discovery.md` reached for its `raw.rs` adapter — stays
  a manual, real-hardware check once implemented.
- The payoff is one layer up: once `opm-transport` exists, a
  `FakeTransport` (implementing the same trait purely in memory — no
  crate for it yet, expected to live wherever the first driver crate
  needs it, likely a `dev-dependencies`-only test helper) lets Phase 3's
  `Capability` implementations and Phase 6's `Protocol` parsing be unit
  tested without a physical device — the actual reason this document
  insists on a trait rather than a concrete type.

## Findings

### 2026-07-10 — Ajazz AK820 (Pro) — first real open + Feature exchange

Command run: a throwaway probe (`opm-discovery` + `opm-transport`,
outside the repo, same convention as `discovery.md`'s throwaway scripts)
opening every interface `discover` reports and calling `get_feature` on
every declared report ID (`0` where none are declared).

**Blocked, then unblocked, by permissions exactly as predicted.** The
first run reproduced `discovery.md`'s documented finding exactly:
`HidTransport::open` returned `Error::Open` / "Permission denied" on
every interface — no udev rule installed, `/dev/hidraw*` still
`root:root` mode `0600`. Installing a udev rule
(`SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0c45", ATTRS{idProduct}=="800a", TAG+="uaccess"`
— `uaccess`, systemd-logind's ACL tag, rather than a `plugdev` group
this system doesn't have) and replugging the device fixed it: all four
`/dev/hidraw1`-`4` nodes gained a `+` (ACL) and group-read/write.
Confirms `discovery.md`'s own callout that a `stat`/mode-bits-only
accessibility check would have missed this — the nodes' owning group
never changed, only their ACL did.

**What was observed, per interface, calling `get_feature`:**

- Interface 0 (`/dev/hidraw1`, standard keyboard, `0x01/0x06`):
  `get_feature(0)` succeeded, 64 bytes, all zero — plausibly the
  standard boot-keyboard LED-state Feature report (all indicators off),
  though this document draws no protocol conclusion from that, only
  notes the exchange itself worked.
- Interface 1 (`/dev/hidraw2`, the five-usage-pair interface):
  `get_feature` failed for every declared report ID (`1, 2, 3, 5, 6`)
  with `ioctl (GFEATURE): Broken pipe` — the kernel's hidraw driver
  reporting "no Feature-type report exists with this ID," not a
  `Transport`-layer bug. Consistent with those IDs being Input-type
  reports (keyboard/consumer/mouse data), not Feature reports; discovery
  currently has no way to tell Input/Output/Feature report *kinds*
  apart (`Identity::Interface::report_ids` is just "IDs that exist
  somewhere in the descriptor") — a real Phase 1 modeling gap this
  exercise surfaced, not previously visible without attempting real I/O.
- Interface 2 (`/dev/hidraw3`, vendor `0xff68/0x61`): `get_feature(0)`
  also failed with the same "Broken pipe" — not every vendor-usage
  interface has a Feature report.
- **Interface 3 (`/dev/hidraw4`, vendor `0xff13/0x01`): `get_feature(0)`
  succeeded — 64 bytes, all zero.** The first confirmed real
  request/response exchange with the AK820's proprietary configuration
  channel. No protocol meaning is claimed for the content (an
  all-zero idle/default state is the most likely reading, not verified)
  — this is a `Transport`-layer milestone, not a Phase 6 one.

One methodology caveat, not a hardware finding: every probe used an
arbitrary 64-byte buffer, not the interface's actual declared report
length (which `opm-discovery` doesn't currently extract — only the *set*
of report IDs, not their sizes). Revisit once Phase 6 needs exact
lengths; `hidreport`, already a dependency, can supply this.

**Conclusion:** every design decision in this document survived contact
with real hardware. `read_input`'s corrected signature (see
"Implementation note" above) was itself only found *because* this
validation was attempted, not predicted in advance — the same value
`discovery.md`'s real-hardware pass already demonstrated. No further
revision needed to the trait's shape.

## Risks and open questions

- **The report-kind decision (Output via `write()`, not
  `send_output_report()`) is a guess, not evidence.** Nothing has probed
  which endpoint the AK820's vendor channel actually expects yet — that's
  Phase 6. If it turns out to need the control-only variant, `write_output`
  gains a documented behavior change or a sibling method; either is a
  small addition, not a redesign, given the trait already isolates this
  choice to one method.
- **`Blocking`'s "long, but finite, default" is unspecified here on
  purpose.** Picking an actual millisecond value without a real device's
  observed response latency to inform it would be a number pulled out of
  the air — left to the implementation crate, revisited once real timing
  data exists.
- **No exclusivity enforcement across multiple `Transport`s on the same
  path** (see "Concurrent / multiple opens" above) — a known gap, not
  believed to matter yet given `Driver::open`'s expected one-open-per-
  interface usage, but unverified against Phase 3's actual design.
- **Cross-platform behavior is unverified**, same caveat as
  `discovery.md`: `hidapi` abstracts the actual I/O calls across
  platforms (unlike enumeration's Linux-specific sysfs dependency), which
  is a reason for optimism that this trait needs no OS-specific redesign
  later — but nothing here has been checked against macOS/Windows.
- **`FakeTransport` doesn't exist yet and isn't designed here** — noted
  as the payoff this design is building toward (see "Testing strategy"),
  but where exactly it lives (a `dev-dependencies` helper in the first
  driver crate? a small shared test-utility crate?) is a Phase 3/4
  question, deliberately deferred.
- **`Identity` doesn't distinguish Input/Output/Feature report kinds —
  confirmed to matter, not just theoretical.** Found during this
  document's own Findings: `opm-discovery`'s `report_ids` is only "the
  set of IDs declared somewhere in the descriptor," so driver code has
  no way to know in advance which of the AK820's IDs `get_feature` will
  even accept without trying each one (as this session's probe did). A
  Phase 1/`opm-discovery` enhancement (report kind per ID, and report
  length while at it — see the Findings' methodology caveat), not a
  `Transport` problem — `Transport` correctly surfaced the mismatch as
  an `Error::Io`, it just can't resolve it. Worth a follow-up, not
  blocking Phase 3.

## Next steps (feed into `docs/roadmap.md`)

- [x] Implement `opm_core::transport` (trait, `Error`, `ReadTimeout`).
- [x] Implement `opm-transport`'s `HidTransport`, backed by `hidapi`.
- [x] Open a real interface on the AK820 Pro and exchange at least one
      report — a `get_feature` round trip against interface 3
      (`/dev/hidraw4`) succeeded; see Findings.
- [ ] Consider extending `opm-discovery`'s `Identity` with report kind
      (and length) per ID, per the Findings' surfaced gap — a Phase 1
      follow-up, not required before Phase 3.
- [x] Revisit this document's "Risks" once that first real exchange
      happened — see the new item above about report kinds; the rest of
      "Risks" stands unchanged.

# 2026-07-10 — Transport phase design, docs/changelog catch-up

Started by auditing the previous session's output before moving on:
`CHANGELOG.md` still only reflected the initial scaffold commit, with
nothing about `opm-discovery`, `pmctl discover`, or `docs/inventory/` —
updated it to cover Phase 1's actual `[Unreleased]` additions. Confirmed
Phase 1 is complete as designed (10/10 tests pass, `cargo fmt`/`clippy`
clean, ran against real hardware per the devlog), with its documented
limitations carried forward rather than papered over: Linux-only by
design, the classification heuristic validated against exactly one
physical unit, no udev rule shipped (out of scope for discovery), and
gray-market VID/PID reuse confirmed real (`manufacturer_string` reports
`"SONiX"`, the OEM chip vendor, not `"Ajazz"`).

Moved on to Phase 2 (`Transport`), following the same RFC-before-code
discipline `discovery.md` established. Asked, and got answered, two
scoping questions before writing anything: (1) proceed as an RFC
document rather than jumping to code — confirmed; (2) whether to leave
the `Transport` trait's exact signature as an open sketch for the user to
design himself (as `driver-model.md` deliberately does for
`Driver`/`Device`/`Capability`) or propose a concrete trait now, as this
session did for `Identity` — chose the latter.

Wrote [`docs/architecture/transport.md`](../architecture/transport.md)
against `hidapi` 2.6.6's actual source (read from the local Cargo
registry cache, not from memory) rather than guessing at its API:
`write()` vs. `send_output_report()`, `read()`/`read_timeout()`,
`get_feature_report()`/`send_feature_report()`, and the
`#[cfg(...)]`-gated `get_input_report()`. Key decisions, each with
reasoning recorded in the document itself:

- Blocking, not async — `pmctl`/driver `Protocol` code only ever does
  short synchronous exchanges; no real concurrency need to justify a
  runtime dependency.
- Expose all three report kinds (Output/Input/Feature) since which one
  the AK820's real vendor channel uses is unknown until Phase 6, and
  adding a report kind later would be a breaking change every driver
  crate absorbs.
- Abstract away `hidapi`'s "Report ID as buffer's first byte" C-API
  convention — `Transport`'s methods take `report_id: u8` as an explicit
  parameter instead, matching how `Identity` already models report IDs
  as data rather than a buffer offset.
- A `ReadTimeout` enum argument per call, not `hidapi`'s separate
  stateful `set_blocking_mode()` — no hidden mode to disagree with a
  per-call timeout. `Blocking` is deliberately a finite default, not a
  literal indefinite wait, so a stalled device can't hang `pmctl`
  forever.
- One `Transport` per HID interface (one `path`), not one per physical
  device — the AK820 alone has three interfaces carrying distinct vendor
  usage pages (see `discovery.md`'s Findings); a driver needing all three
  holds three `Transport`s.
- The trait lives in `opm-core` (transport-library-free), its
  `hidapi`-backed implementation in a new `opm-transport` crate — the
  exact dependency-direction pattern ADR 0002 already established for
  `Identity`/`opm-discovery`, now recorded as
  [ADR 0003](../architecture/decisions/0003-transport-trait-in-core-impl-in-opm-transport.md).
  `opm-transport` and `opm-discovery` deliberately don't depend on each
  other, despite both wrapping `hidapi` — different problems (sustained
  I/O vs. zero-I/O enumeration), accepted duplication.
- The concrete payoff of a trait over a bare `hidapi` struct: Phase 3
  `Capability` code and Phase 6 `Protocol` code can be unit-tested
  against a fake, in-memory `Transport` with no physical device attached
  — the real reason for the indirection, not dependency-direction purity
  for its own sake.

Updated `domain-model.md` (the `Transport` row, no longer "undesigned"),
`overview.md` (added `opm-transport` to the crate tree and its own
section, removed the now-answered "should there be a shared transport
crate" open question), and `roadmap.md` (Phase 2: designed, not yet
implemented, with its own next-step checklist) to stay in sync, same
discipline the previous session applied to Phase 1's documents.

Nothing in this session touched real hardware or wrote any Rust — like
`discovery.md`'s first design pass, this is reasoning from `hidapi`'s
actual source plus the USB HID report-type semantics it wraps. The
open risks section of `transport.md` is explicit about what's still a
guess (which report kind the AK820 actually needs, the `Blocking`
timeout's concrete value) versus what's a settled architectural choice.

## Addendum, same day: implemented, hit the predicted permission wall

Implemented the design for real: `opm_core::transport` (`Transport`,
`Error`, `ReadTimeout`, no `hidapi` import anywhere in the file) and a
new `opm-transport` crate (`HidTransport`, depends on `opm-core` +
`hidapi`). `cargo build`/`clippy -D warnings`/`fmt --check` all clean
across the whole workspace, matching every other crate's gates.

Implementing `read_input` surfaced a real design bug the RFC pass
missed: the first draft gave it a `report_id` *input* parameter,
symmetric with `write_output`/`get_feature`/`set_feature`. That doesn't
hold up against `hidapi`'s actual semantics — Output and Feature reports
are host-requested by ID over the control endpoint, but an Input report
is *pushed* by the device over the interrupt endpoint whenever it has
one; there's no way to ask for "the next Input report with ID 3"
specifically. Fixed before any of it shipped: `read_input` now returns
`(report_id, payload_len)` instead of accepting an ID as input.
`transport.md` records this as an "Implementation note" rather than
silently editing the RFC — the same finding-discipline `discovery.md`
established.

Wrote a throwaway probe (`probe_transport`, outside the repo, same
convention as Phase 1's throwaway `hidapi`/`hidreport` scripts) that
runs real `opm-discovery` against the machine, finds the AK820 by
VID:PID, and tries `HidTransport::open` on every interface that declares
a report ID. Result: `opm-discovery` still finds the AK820 (4 interfaces,
same shape as the 2026-07-09 capture), but every `open()` call returns
`Error::Open` with `"Permission denied"` — this machine still has no
udev rule installed, `/dev/hidraw*` is still `root:root` mode `0600`,
exactly the condition `discovery.md`'s Findings already documented.

**Conclusion:** the software side of Phase 2 is done — trait, impl,
error path all validated, including validating the error path against
real hardware (a real "permission denied" from a real device, not a
mocked one). The read/write happy path — actually opening the AK820 and
exchanging a report — is blocked on the same permission gap Phase 1
flagged as a known, expected, out-of-the-box condition, not a new
problem. Unblocking it needs either a udev rule (a persistent, system-
wide change outside this repo, not made unilaterally) or a manual `sudo`
run — left for the maintainer to do, then re-run `probe_transport`'s
equivalent to actually close this out.

## Addendum 2, same day: udev rule, real Feature-report exchange, Phase 2 closed out

Fixed the permission gap with a udev rule
(`SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0c45", ATTRS{idProduct}=="800a", TAG+="uaccess"`,
in `/etc/udev/rules.d/70-opm-ajazz-ak820.rules`) — `uaccess` (systemd-
logind's ACL tag) rather than a `plugdev` group, since this machine
(Arch) doesn't have one. The maintainer installed it himself (`sudo tee`
+ `udevadm control --reload-rules` + `udevadm trigger`); all four AK820
`/dev/hidraw*` nodes picked up a `+` (ACL) and group read/write.

Re-ran the probe. `HidTransport::open` now succeeds on every interface.
Walked every interface's declared report IDs (`0` where none are
declared) calling `get_feature` — a real, if exploratory, first use of
the `Transport` trait against physical hardware:

- Interface 0 (standard keyboard) and **interface 3 (`/dev/hidraw4`, a
  dedicated vendor channel, usage `0xff13/0x01`) both returned a real
  64-byte Feature report** (all zero — plausibly idle state, not
  interpreted further; that's Phase 6's job).
- Interfaces 1 and 2 (report IDs `1,2,3,5,6` and `0` respectively) all
  failed with `ioctl (GFEATURE): Broken pipe` — the kernel correctly
  reporting "no Feature-type report exists with this ID," not a
  `Transport` bug. Surfaced a real Phase 1 gap: `opm-discovery`'s
  `Identity::Interface::report_ids` records which IDs exist, not which
  *kind* (Input/Output/Feature) each one is — recorded in
  `transport.md`'s Findings/Risks as a Phase 1 follow-up, not a Phase 2
  blocker.

`transport.md`'s status line moved to "Accepted, implemented, validated
against real hardware," matching `discovery.md`'s convention exactly.
`roadmap.md`'s Phase 2 checklist is now fully checked. Deliberately
stopped at reads (`get_feature`) — did not attempt `write_output` or
`set_feature` against the AK820's still-unknown vendor protocol; sending
unvalidated writes to a proprietary channel is Phase 6's reverse-
engineering work, not something to do incidentally while validating the
transport layer.

Next: Phase 3 (`Driver`/`Device`/`Capability` traits, `driver-model.md`)
can now be designed against a `Transport` that's proven to actually open
and exchange data with the real device it'll eventually be built for.

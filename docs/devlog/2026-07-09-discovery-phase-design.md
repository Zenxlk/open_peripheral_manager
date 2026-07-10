# 2026-07-09 — Discovery phase design

Designed the hardware discovery phase before writing any HID/protocol
code, as an explicit research step: what can be learned about a connected
HID device (identity, interface topology, declared usage) without
knowing its protocol, how to classify a plain keyboard vs one with a
vendor-specific configuration channel, and how a future `pmctl discover`
should behave. Written up in `docs/architecture/discovery.md`.

This forced an architecture question `docs/architecture/overview.md` had
deliberately left open — where HID/transport access lives, given
`opm-core` may never depend on a transport library — since discovery
can't exist without a transport library. Resolved via
[ADR 0002](../architecture/decisions/0002-discovery-lives-outside-opm-core.md):
discovery gets its own crate, outside `opm-core`, that `opm-cli` and
future driver crates both depend on.

Also added `docs/inventory/` — a shallow, cross-device catalog (one row
per device ever discovered, supported or not), separate from
`docs/protocols/`'s deep per-device reverse-engineering notes, to give
gray-market vendor/product identification (unreliable in the public
USB-ID database for hardware like this) somewhere durable to accumulate.

Nothing in this session touched real hardware or wrote any Rust — it's
reasoning from `hidapi`/HID/USB documentation only. `docs/architecture/discovery.md`
itself flags where that reasoning is a guess versus established fact.

Next: get the AK820 enumerating via `hidapi` and check the design against
reality (see `docs/roadmap.md`).

## Addendum, same day: roadmap reorder + domain model + `--export`

Reordered `docs/roadmap.md` into eight explicit phases (0-7). The
significant change: protocol reverse-engineering moves from right after
discovery to **phase 6**, second-to-last, after the driver system, a
first driver (with stub capabilities), and the CLI. The reasoning: build
every architecture-shaping piece — transport, `Driver`/`Capabilities` —
against the general problem first, and let the AK820's actual byte-level
protocol (the most vendor-specific, least reusable part) come last
rather than shape the traits from day one.

Added `docs/architecture/domain-model.md`, naming five facets every
driver crate will need regardless of vendor: `Identity`, `Transport`,
`Capabilities`, `Driver`, `Protocol`. This doesn't change ADR 0001
(still one crate per driver) — it's about what's consistently organized
*inside* each one, so the second driver doesn't force renaming things
the AK820's driver happened to call them.

Expanded `discovery.md`'s `--json` flag into `--export <format>`: a
portable, shareable report (host OS/kernel context + full device data)
meant to leave the machine it was captured on — pasted into an issue, or
submitted to `docs/inventory/` by someone whose hardware a maintainer
doesn't own. Added a privacy default (`serial_number` redacted unless
`--include-serial`) since the report is designed to be shared publicly.
Noted, but explicitly deferred, the idea of eventually bootstrapping a
new driver crate's scaffolding from a submitted report.

## Addendum 2, same day: independent review + corrections

Had a fresh Fable 5 agent review the discovery-phase documents
critically before implementation starts. It verified several claims
against `hidapi`'s actual source rather than trusting the prose, and
found a real architecture bug: ADR 0002, as first written, had
`opm-core`'s `Driver` trait "name" the discovery crate's descriptor type
without depending on it — impossible in Rust. Fixed by first correcting
`overview.md`'s framing (`opm-core`'s actual rule was always "never a
transport library", not "zero dependencies forever" — that ambiguity is
what caused the contradiction), then flipping ADR 0002's dependency
direction: `Identity` is now defined in `opm-core` as plain, transport-
free data, and the newly-named `opm-discovery` crate depends on
`opm-core` (using `hidapi` internally) rather than the reverse.

Applied the review's other findings, choosing existing crates over
hand-rolled code wherever one already does the job well (`hidreport`/
`hut` for report-descriptor parsing, `serde` for `--export`) rather than
minimizing dependencies for its own sake:

- Corrected two factual errors: the sysfs `report_descriptor` file is
  what `hidapi` actually reads on Linux (not an ioctl, and it needs no
  permissions), and `enumerate()` returns one entry per top-level
  collection since libhidapi 0.13, not one per interface — the
  classification heuristic and grouping algorithm were reworded around
  usage pairs accordingly.
- Inverted the interface-grouping strategy: topology (sysfs parent USB
  device) is now the primary grouping key on Linux, serial number is
  recorded as metadata only — budget controllers are known to hardcode
  identical serials across units, which would have silently merged
  distinct physical devices under the old serial-first policy.
- Added the pieces `discovery.md` had promised but not written: an exit-
  code table, a concrete (open-then-close, zero-I/O) accessibility check
  definition, a draft JSON schema for `--export` with a `schema_version`
  field, and a "Testing strategy" section (pure grouping/classification
  functions over serializable data, so every community-submitted report
  doubles as a test fixture).

`docs/architecture/domain-model.md` and `overview.md` updated to match
throughout.

## Addendum 3, same day: first real hardware, AK820 Pro in hand

The maintainer's AK820 Pro was connected, so ran the design's own
prescribed "next step" before touching `opm-discovery`'s real
implementation: two throwaway scripts (`hidapi::device_list()`, then
`hidreport::ReportDescriptor::try_from()` against the raw sysfs
`report_descriptor` bytes), kept outside the repo, in the scratch
directory — not committed, per the design's "throwaway script is
enough" framing.

Result: every design decision held up unchanged against real hardware.
4 hidraw interfaces sharing one sysfs-topology parent (confirming
topology-first grouping); interface 1 alone declaring five top-level
usage pairs sharing one `hidapi` path (confirming, and exceeding, the
predicted multi-collection-per-interface case); three distinct
vendor-defined usage pages total; classification landed on
`Configurable Keyboard` exactly as predicted; `serial_number` was
`Some("")` rather than `None` — an unanticipated third case, but one
that only reinforces the decision to not group by serial;
`manufacturer_string` reported `"SONiX"` (the OEM chip vendor) rather
than `"Ajazz"`, confirming the gray-market-VID concern was real, not
hypothetical; and all four `/dev/hidraw*` nodes were root-only with no
udev rule installed on this machine, confirming the accessibility-check
design reflects a real out-of-the-box condition.

Wrote this up as `discovery.md`'s first "Findings" entry, saved the
capture to `docs/inventory/captures/ajazz-ak820-2026-07-09.json` (hand-
built against the draft `--export` schema, since the real command
doesn't exist yet), filled in `docs/inventory/devices.md`'s first row,
and filled in `docs/protocols/ajazz-ak820/README.md`'s previously-empty
"Hardware identity" section. `docs/roadmap.md` and `discovery.md`'s own
checklists updated to reflect Phase 1's validation steps as done — only
implementing the `opm-discovery` crate itself remains.

## Addendum 4, same day: `opm-discovery` implemented, `pmctl discover` works

Wrote the crate for real. `Identity`/`Interface`/`UsagePair` (plain,
`serde`-derived data, no transport dependency) landed in `opm-core` as
`identity.rs`, per ADR 0002's corrected dependency direction. The new
`crates/opm-discovery` holds: a thin `hidapi` adapter (`raw.rs`,
decoupled into a `RawEntry` type so nothing downstream needs `hidapi`
itself); the sysfs topology resolver (`topology.rs`, the one impure I/O
function the grouping algorithm needs); pure `dedupe_by_path` /
`group_by_topology` / `build_identity` functions (`group.rs`) with unit
tests built from the real AK820 capture's raw shape; the classification
heuristic (`classify.rs`) with tests covering each signal combination,
including the AK820's actual case of the vendor signal living on a
separate interface from the keyboard signal; report-ID extraction via
`hidreport` (`descriptor.rs`); and the accessibility check (`accessible.rs`,
open-then-close, no reads/writes). 10 unit tests, all passing, zero
`clippy -D warnings` findings, `cargo fmt --check` clean — matching this
project's CI gates from commit one.

Wired a first `pmctl discover` (default output only — `--export`,
`--verbose`, and the exit-code table are follow-up work) and ran it
against the same AK820 Pro. Output matched the earlier throwaway-script
findings exactly, this time through the real topology-resolution code
path rather than a hand-fed test fixture: 4 interfaces grouped into one
device via actual sysfs traversal, classified `Configurable Keyboard`,
report IDs `[1, 2, 3, 5, 6]` on interface 1. Also correctly enumerated
and classified two other HID devices already attached to the machine
(a USB mouse, an I2C touchpad) as `Unknown HID` — the classification
heuristic's mouse-usage and touchpad case has no dedicated category yet,
exactly as designed (this document doesn't claim to classify everything,
only to flag what's plausibly a configurable keyboard).

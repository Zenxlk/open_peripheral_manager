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

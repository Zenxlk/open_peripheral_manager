# Device inventory

A cross-device catalog of every physical peripheral OPM has ever been
pointed at with `pmctl discover` (or, until that command exists, whatever
throwaway enumeration script stands in for it) — regardless of whether a
driver exists for it yet.

This is different from [`docs/protocols/`](../protocols/): that directory
holds the deep, per-device reverse-engineering notes for devices actively
being supported. This directory is shallow and broad — one row per device
ever seen, supported or not — and exists so identity information (vendor
name, VID/PID, interface layout) doesn't have to be rediscovered by hand
every time, and so USB-ID-database gaps for small/gray-market vendors
(see [`docs/architecture/discovery.md`](../architecture/discovery.md))
get filled in from real hardware instead of guessed at.

## Layout

- [`devices.md`](devices.md) — one row per physical device: vendor,
  product, VID:PID, interface summary, classification (per
  `discovery.md`'s heuristic), the OPM/heuristic version that produced
  that classification, and driver status. The durable, at-a-glance
  summary.
- `captures/` — raw `pmctl discover --export json` reports per device,
  named `<vendor>-<model>-<YYYY-MM-DD>.json`. The evidence a `devices.md`
  row is derived from. Small (a few KB of text), so — unlike
  `docs/protocols/*/captures/`, which holds large binary USB traces —
  these are tracked in git, not ignored.

## Contributing a new device

1. Run `pmctl discover --export json` (once it exists) against your
   hardware.
2. Save the report as `captures/<vendor>-<model>-<date>.json`.
3. Add one row to `devices.md` summarizing it, including the report's
   `classified_by` value (see `discovery.md`'s export schema) — a later
   change to the classification heuristic shouldn't make an old row look
   more authoritative than it is.
4. If anything about the result contradicts a heuristic or assumption in
   `docs/architecture/discovery.md`, note it there too — this catalog
   records *what was seen*; `discovery.md` records *what OPM currently
   believes*, and the two should never silently drift apart.

# Roadmap

No dates — this is a spare-time, learning-driven project. Rough order of
intent, revisited as reality dictates.

## Now: foundations

- [x] Cargo workspace, quality tooling, CI, docs structure.
- [ ] Design `opm-core`'s `Device`/`Driver`/capability traits (see
      `docs/architecture/driver-model.md`).

## Next: first device, read-only

- [ ] Get the Ajazz AK820 enumerating over HID on Linux; log raw reports.
- [ ] Reverse-engineer enough of the protocol to read basic device info
      (see `docs/protocols/ajazz-ak820/`).
- [ ] First real driver crate: `drivers/opm-driver-ajazz-ak820`,
      implementing `opm-core`'s traits against real findings instead of
      guesses.
- [ ] `pmctl list` / `pmctl info` actually work end to end.

## Later: first real feature

- [ ] Reverse-engineer and implement one write capability — likely RGB,
      since it's the most observable. Feeds back into finalizing the
      capability-detection pattern in `opm-core` against a second,
      concrete need (compare with `pmctl profile`).
- [ ] `pmctl rgb` / `pmctl profile` work for the AK820.

## Eventually

- [ ] Second device/vendor, to pressure-test that the driver
      abstraction actually generalizes and isn't secretly AK820-shaped.
- [ ] Windows/macOS transport support.
- [ ] GUI crate reusing `opm-core`, once there's a stable-enough trait
      surface to build one against.
- [ ] Publish `opm-core` (and stable drivers) to crates.io.

## Explicitly not planned right now

- Broad "support every keyboard" ambitions before the architecture has
  been proven against at least two real, different devices.
- A plugin/dynamic-loading system for drivers — static linking via the
  workspace is simpler and sufficient while the driver count is small.

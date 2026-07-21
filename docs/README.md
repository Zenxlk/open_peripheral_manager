# Documentation map

- [`architecture/`](architecture/) — how the project is put together and
  why. Start with [`architecture/overview.md`](architecture/overview.md).
  [`architecture/domain-model.md`](architecture/domain-model.md) names
  the shared vocabulary (`Identity`/`Transport`/`Capabilities`/`Driver`/
  `Protocol`) every later phase builds on.
  [`architecture/discovery.md`](architecture/discovery.md) designs how
  hardware is found and classified, before any protocol work starts.
  [`architecture/transport.md`](architecture/transport.md) designs how
  bytes actually move once a device is opened.
  [`architecture/driver-model.md`](architecture/driver-model.md) designs
  `Driver`/`Device`/`Capability` — how a driver crate declares "I can
  handle this" and exposes optional features. Design decisions with
  lasting consequences are recorded as ADRs in
  [`architecture/decisions/`](architecture/decisions/).
- [`inventory/`](inventory/) — a shallow, cross-device catalog: every
  physical device ever run through discovery, supported or not. See
  [`inventory/README.md`](inventory/README.md).
- [`protocols/`](protocols/) — one directory per device, documenting the
  reverse-engineering process: captures, findings, open questions. Starts
  with [`protocols/ajazz-ak820/`](protocols/ajazz-ak820/).
- [`devlog/`](devlog/) — a dated development diary. One file per session
  or milestone, newest last.
- [`roadmap.md`](roadmap.md) — where the project is headed.

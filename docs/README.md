# Documentation map

- [`architecture/`](architecture/) — how the project is put together and
  why. Start with [`architecture/overview.md`](architecture/overview.md).
  [`architecture/domain-model.md`](architecture/domain-model.md) names
  the shared vocabulary (`Identity`/`Transport`/`Capabilities`/`Driver`/
  `Protocol`) every later phase builds on.
  [`architecture/discovery.md`](architecture/discovery.md) designs how
  hardware is found and classified, before any protocol work starts.
  Design decisions with lasting consequences are recorded as ADRs in
  [`architecture/decisions/`](architecture/decisions/).
- [`inventory/`](inventory/) — a shallow, cross-device catalog: every
  physical device ever run through discovery, supported or not. See
  [`inventory/README.md`](inventory/README.md).
- [`protocols/`](protocols/) — one directory per device, documenting the
  reverse-engineering process: captures, findings, open questions. Starts
  with [`protocols/ajazz-ak820/`](protocols/ajazz-ak820/).
- [`learning/`](learning/) — the maintainer's running notes on Rust
  concepts learned while building this project. Not a tutorial for
  others; kept for personal reference and to show the project's history.
- [`devlog/`](devlog/) — a dated development diary. One file per session
  or milestone, newest last.
- [`roadmap.md`](roadmap.md) — where the project is headed.

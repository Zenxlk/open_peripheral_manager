# Architecture overview

Open Peripheral Manager (OPM) is a Rust **library-first** project: the
reusable logic lives in a library crate, and every user-facing surface
(CLI today, GUI eventually) is a thin client of that library. Nothing
device- or vendor-specific lives outside a driver crate.

```
open-peripheral-manager/
├── crates/
│   ├── opm-core/     # library: traits + registry every front-end depends on
│   └── opm-cli/      # binary `pmctl`: thin CLI front-end
├── drivers/
│   └── opm-driver-<vendor>-<model>/   # one crate per device (none yet)
└── docs/
```

## Why a Cargo workspace

A single repo, multiple crates, one `Cargo.lock`. This lets `opm-core`
compile independently of any driver or front-end, keeps compile times
down as drivers are added (only touched crates rebuild), and lets each
crate have exactly the dependencies it needs — a driver crate can depend
on a HID library without `opm-cli` ever seeing it in its dependency tree.

## Why each crate exists today

### `opm-core`

The vendor-agnostic contract layer: what a "device" and a "driver" are,
in the abstract. Both `opm-cli` and the future GUI depend on this crate
and, ideally, nothing else, to talk to a device. It has **zero**
dependencies right now and must never depend on a transport (HID/USB)
library or know about any specific vendor. See
[`driver-model.md`](driver-model.md) for the (still unfinished) design of
its traits.

### `opm-cli`

The `pmctl` binary. Exists as a separate crate — not folded into
`opm-core` — so that:

- `opm-core` stays a pure library, embeddable in a future GUI without
  pulling in `clap` or anything terminal-specific.
- CLI concerns (argument parsing, output formatting, exit codes) don't
  leak into the domain model.

It currently only parses arguments for the commands described in the
README (`list`, `info`, `rgb`, `profile`); each one prints a
"not implemented" message. No real logic yet.

### `drivers/` (empty for now)

Reserved location for one crate per manufacturer/device, e.g.
`drivers/opm-driver-ajazz-ak820`. Already wired into the workspace
(`drivers/*` in the root `Cargo.toml`) so the first driver crate needs no
further workspace configuration. See [`drivers/README.md`](../../drivers/README.md)
for the rationale and naming convention.

## Deliberately not created yet

- **A GUI crate.** The plan is for it to depend on `opm-core` exactly
  like `opm-cli` does, likely living at `crates/opm-gui` once a toolkit
  (egui? Tauri? iced?) is chosen — a decision to make when there's
  actually a `Device` trait to build a UI around, not before.
- **A transport/HID crate.** Whether HID access is its own crate
  (`opm-hid`) shared by drivers, or lives inside each driver crate, is an
  open question to answer once the first driver is being written and the
  real needs are known. Splitting it out prematurely risks guessing the
  wrong abstraction.
- **`opm-driver-api`.** If, once a second real driver exists, `opm-core`
  feels too heavy a dependency just to get at the `Driver`/`Device`
  traits, consider extracting those traits into a smaller crate that
  driver crates depend on instead of all of `opm-core`. Not worth doing
  for a single driver.

## Naming conventions

- **Crates:** `opm-<role>`, kebab-case. Driver crates:
  `opm-driver-<vendor>-<model>` (lowercase, hyphenated).
- **Binary:** the CLI binary is named `pmctl`, distinct from its crate
  name `opm-cli`, because that's the name users type.
- **Modules:** one concept per file, snake_case, no `mod.rs` — a module
  `foo` with submodules is `foo.rs` + `foo/bar.rs`, not `foo/mod.rs`
  (the modern, non-2015 style).
- **Traits:** capability traits are nouns describing the feature, not
  prefixed with `Has`/`Can` (e.g. `Rgb`, not `HasRgb`) — to be settled
  for real in [`driver-model.md`](driver-model.md).

## Quality gates

Configured from commit one, before any domain code exists:

- `rustfmt.toml` — formatting, enforced in CI (`cargo fmt --check`).
- `[workspace.lints]` in the root `Cargo.toml` — `clippy::all` and
  `missing_docs` as warnings, enforced in CI via
  `cargo clippy -- -D warnings`. Deliberately not `clippy::pedantic`
  yet; see the lints table itself for how to tighten this later.
- GitHub Actions (`.github/workflows/ci.yml`) — fmt, clippy, build, test,
  run on every push and PR.

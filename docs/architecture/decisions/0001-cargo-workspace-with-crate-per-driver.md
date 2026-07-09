# 0001. Cargo workspace with one crate per driver

Date: 2026-07-08

Status: Accepted

## Context

The project's stated goal is to support many peripherals from many
manufacturers over time, starting with the Ajazz AK820 keyboard, while
staying reusable as a library by both a CLI and a future GUI. A single
crate containing everything would couple unrelated vendors' code together
and force every consumer (including a future GUI) to compile every
driver's dependencies.

## Decision

Use a Cargo workspace. Keep vendor-agnostic contracts in a library crate
(`opm-core`) with no transport or protocol code. Give each front-end its
own crate (`opm-cli` today). Give each supported device its own crate
under `drivers/`, named `opm-driver-<vendor>-<model>`.

## Consequences

- Adding a new device means adding a new crate under `drivers/`, not
  editing existing ones — low risk of one vendor's changes breaking
  another's.
- `opm-core` can be published/versioned independently of any driver.
- Slightly more ceremony than a single crate (more `Cargo.toml` files,
  workspace-level dependency version management) — acceptable for a
  project meant to grow over years.
- The exact `Driver`/`Device` trait design is *not* fixed by this
  decision and is deliberately left open; see
  `docs/architecture/driver-model.md`.

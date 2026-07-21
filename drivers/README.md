# Drivers

This directory holds one Cargo crate per supported manufacturer/device
combination. It is wired into the workspace (`drivers/*` in the root
`Cargo.toml`), so a new driver crate is picked up automatically, no root
`Cargo.toml` edit required.

`opm-driver-ajazz-ak820` is the first one (see `docs/roadmap.md`):
matches the real AK820, opens its vendor `Transport`, exposes
`Rgb`/`Profiles`. Solid-color RGB (`Rgb::set_color`) is real and
validated against physical hardware; `get_color`/`Profiles` are still
stubs pending more of Phase 6's protocol reverse-engineering. Linked
into `pmctl` (`opm-cli/src/commands/registry.rs`). See
`docs/protocols/ajazz-ak820/README.md` for anything specific to
running this particular driver against real hardware (permissions,
troubleshooting) — this file stays about the `drivers/` convention in
general, not any one device.

## Why a crate per device (and not one big `drivers` crate)

- **Independent evolution.** Reverse-engineering the Ajazz AK820 protocol
  shouldn't risk breaking, or require touching, a hypothetical future
  Logitech driver.
- **Optional compilation.** Front-ends (and users building from source)
  should eventually be able to pick which drivers to compile in, instead
  of always pulling in every vendor's dependencies.
- **Clear ownership.** A crate boundary is a natural place to hang a
  `CODEOWNERS` entry, a changelog, and vendor-specific documentation.

## Naming convention

`opm-driver-<vendor>-<model>`, lowercase, hyphen-separated, e.g.:

```
drivers/opm-driver-ajazz-ak820/
```

## Shape of a driver crate

- Depends on `opm-core` (for the `Driver`/`Device`/`Capability` traits)
  and `opm-transport` (to actually open a device) — see
  `opm-driver-ajazz-ak820/Cargo.toml`.
- Contains all HID/USB transport-usage and protocol-specific logic for
  that one device — none of that belongs in `opm-core`.
- Ships its own protocol notes under
  `docs/protocols/<vendor>-<model>/`.

See `docs/architecture/driver-model.md` for the design of the traits a
driver crate implements.

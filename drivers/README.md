# Drivers

This directory holds one Cargo crate per supported manufacturer/device
combination. It is currently empty — no driver has been written yet — but
it is already wired into the workspace (`drivers/*` in the root
`Cargo.toml`) so the first driver crate is picked up automatically the
moment it's added, no root `Cargo.toml` edit required.

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

## Expected shape of a driver crate (for later, not implemented yet)

- Depends on `opm-core` and implements its `Driver`/`Device` traits.
- Contains all HID/USB transport and protocol-specific logic for that one
  device — none of that belongs in `opm-core`.
- Ships its own protocol notes under
  `docs/protocols/<vendor>-<model>/`.

See `docs/architecture/driver-model.md` for the (not yet finalized) design
of the traits a driver crate will need to implement.

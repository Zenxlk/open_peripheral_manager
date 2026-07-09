# Contributing

Thanks for considering contributing. The project is in early scaffolding
— see [`docs/roadmap.md`](docs/roadmap.md) for what's actually in scope
right now before proposing large changes.

## Ground rules

- By contributing, you agree your contributions are licensed under this
  project's dual MIT/Apache-2.0 license (see [`README.md`](README.md#license)).
- Please read the [Code of Conduct](CODE_OF_CONDUCT.md).

## Before you start

For anything beyond a small fix, please open an issue first to discuss
the approach — especially for anything touching `opm-core`'s traits,
which every driver and front-end depends on.

## Adding support for a new device

This is the contribution this project is most designed for. Rough shape
(also see [`drivers/README.md`](drivers/README.md) and
[`docs/architecture/driver-model.md`](docs/architecture/driver-model.md)):

1. Document the reverse-engineering process under
   `docs/protocols/<vendor>-<model>/` as you go — captures, findings,
   open questions. Future maintainers (including future you) will thank
   you.
2. Create `drivers/opm-driver-<vendor>-<model>`, implementing `opm-core`'s
   `Driver`/`Device` traits.
3. Wire it into `opm-cli` and open a PR.

## Development workflow

```
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

All four run in CI on every pull request; please run them locally first.

## Commit / PR style

- Keep PRs focused; unrelated cleanup belongs in its own PR.
- Use the PR template's testing checklist.
- Prefer clear, well-documented code over clever code — this is
  explicitly a project people are using to learn Rust, contributors
  included.

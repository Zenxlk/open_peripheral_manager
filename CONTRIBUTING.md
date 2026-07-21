# Contributing

Thanks for considering contributing. See [`docs/roadmap.md`](docs/roadmap.md)
for what's actually in scope right now before proposing large changes.

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

## Branching

`master` only ever moves via a merged pull request — including for the
maintainer's own changes, even working solo. Create a branch off
`master` for anything you're changing (`git checkout -b <short-name>`),
push it, and open a PR rather than committing to `master` directly.
This is what keeps `CHANGELOG.md`/`docs/devlog/` and the commit history
meaningful as the project grows past a single contributor.

See [`RELEASING.md`](RELEASING.md) for cutting a tagged release once
something's ready.

## Commit / PR style

- Keep PRs focused; unrelated cleanup belongs in its own PR.
- Use the PR template's testing checklist.
- Prefer clear, well-documented code over clever code — this is
  explicitly a project people are using to learn Rust, contributors
  included.

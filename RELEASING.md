# Releasing

This project isn't published to crates.io yet (see `docs/roadmap.md`'s
"Later, cross-cutting") — a release today means a tagged `pmctl` binary
attached to a GitHub Release, nothing more.

1. On `master`, with a clean working tree:
   - Bump `version` in the root `Cargo.toml`'s `[workspace.package]` —
     every crate in the workspace shares it (`version.workspace = true`
     in each crate's own `Cargo.toml`).
   - Run `cargo build --workspace` once so `Cargo.lock` picks up the
     new version.
   - Move `CHANGELOG.md`'s `[Unreleased]` section to a new `[X.Y.Z] -
     YYYY-MM-DD` heading (keep an empty `[Unreleased]` above it for
     what comes next).
2. Commit: `Release vX.Y.Z` (via a branch + PR like any other change —
   see `CONTRIBUTING.md`).
3. After that PR merges, on `master`: `git tag vX.Y.Z && git push origin
   vX.Y.Z`.
4. Pushing the tag triggers `.github/workflows/release.yml`: re-runs
   tests and clippy (a tag push doesn't go through branch protection,
   so this doesn't trust that CI already passed), builds
   `pmctl --release`, and publishes it as a GitHub Release with an
   auto-generated changelog from commit history.

## Versioning

Semantic versioning once there's a `0.y.z` → `1.0.0` boundary worth
defining; until then, `0.y.z` bumps are at the maintainer's judgment —
this is still a solo, pre-1.0, actively-changing project (see
`README.md`'s status).

## Arch packaging

[`packaging/arch/PKGBUILD`](packaging/arch/PKGBUILD) builds `pmctl`
from a tagged release's source tarball — not maintained in lockstep
automatically, since it pins an exact `pkgver`/`sha256sums` pair. After
cutting a new tag (above):

1. Update `PKGBUILD`'s `pkgver` to match, and its `sha256sums` to the
   new tarball's real hash:
   ```
   curl -sL -o /tmp/pmctl-X.Y.Z.tar.gz \
     https://github.com/Zenxlk/open_peripheral_manager/archive/refs/tags/vX.Y.Z.tar.gz
   sha256sum /tmp/pmctl-X.Y.Z.tar.gz
   ```
2. Regenerate `.SRCINFO` (AUR requires it committed alongside
   `PKGBUILD`): `cd packaging/arch && makepkg --printsrcinfo > .SRCINFO`.
3. Test locally before pushing: `cd packaging/arch && makepkg -si`
   (needs `base-devel` installed). `options=('!lto')` is required —
   Arch's default LTO build flags break `hidapi`'s vendored C shim
   (found and fixed while building this file the first time; see the
   comment above it in `PKGBUILD`).
4. Same branch + PR flow as any other change.

Not yet submitted to the AUR itself — this only builds/installs
locally (`makepkg -si`) for now.

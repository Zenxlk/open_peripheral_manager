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

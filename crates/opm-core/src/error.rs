//! Will hold the top-level `Error` type shared across `opm-core`'s public
//! API. Individual driver crates will likely define their own, more
//! specific error types and convert into this one at the `Driver`/`Device`
//! trait boundary.
//!
//! `thiserror` is the natural fit here once this has real variants; it is
//! intentionally not yet a dependency of this crate.
//!
//! Not yet designed — start here when you're ready to design error
//! handling.

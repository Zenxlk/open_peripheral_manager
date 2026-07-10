# Rust learning notes

Personal, running notes on Rust concepts encountered while building this
project — not a tutorial, just a record for future-me (and anyone curious
about the project's history). One entry per concept, added as it comes
up; no need to be exhaustive or polished.

Suggested format:

```
## Concept

What it is, in my own words. Where in this codebase it showed up (or
will show up). Anything that was confusing and how it clicked.
```

## Decoupling from a dependency's types at the boundary

When wrapping `hidapi`, I didn't pass `hidapi::DeviceInfo` around the
rest of the crate — I mapped it into my own `RawEntry` struct right at
the edge (`opm-discovery/src/raw.rs::enumerate()`), and everything past
that point (`group.rs`, `classify.rs`) only ever sees `RawEntry`. The
payoff showed up immediately: the unit tests in `group.rs` and
`classify.rs` construct `RawEntry`/`Identity` values by hand and never
touch `hidapi` at all, so they run instantly with no hardware and no
mocking framework. The rule of thumb: one thin, ugly adapter module at
the edge that talks to the outside world, and pure functions everywhere
else that only know about your own types.

## `HashMap::entry()` — insert-or-update without double lookups

`group.rs`'s `dedupe_by_path` needed "if this key exists, push into its
list; otherwise create a new one-item list." The naive way is
`if map.contains_key(&k) { ... } else { ... }` — two lookups, and easy
to get subtly wrong. `.entry(key).and_modify(|v| ...).or_insert_with(|| ...)`
does it in one lookup: `and_modify` runs only if the key was already
there, `or_insert_with` only if it wasn't, and the closure passed to
`or_insert_with` is lazy (only built when actually needed) which is why
it takes a closure and not a plain value.

## Preserving insertion order out of a `HashMap`

`HashMap` iteration order is unspecified, but I wanted `pmctl discover`'s
output to list devices in a stable, first-seen order (also matters for
the unit tests, which assert on specific rows). Fix used in both
`dedupe_by_path` and `group_by_topology`: keep a plain `Vec<Key>`
alongside the `HashMap`, pushed to only the first time a key is seen,
then iterate the `Vec` at the end and `.remove()` each entry out of the
map. Simple, no extra dependency — the `indexmap` crate exists
specifically to avoid writing this by hand, worth reaching for if this
pattern shows up a third time.

## Match ergonomics on `&Option<T>`

`hidreport`'s `Report::report_id()` returns `&Option<ReportId>`, not
`Option<ReportId>`. I expected to need `(*report.report_id()).map(...)`
or similar, but `if let Some(id) = report.report_id() { ... }` just
works, binding `id: &ReportId` — Rust's "match ergonomics" let a
`Some(x)` pattern match through a reference automatically, adjusting the
binding to a reference instead of forcing a manual deref everywhere.
Then, since `ReportId` is `Copy`, `(*id).into()` copies it out to
convert to `u8`. Tripped me up for a second in `descriptor.rs` before I
looked up why it compiled at all.

## `impl Trait` in a return type is a distinct, unnameable type per call site

`hidreport::ReportDescriptor::{input_reports, output_reports,
feature_reports}` all return `&[impl Report]` — and I tried to
`.iter().chain()` two of them together assuming they were "the same
kind of thing." Compile error: each `impl Report` is its own opaque
type, even though they satisfy the same trait, so `Chain` couldn't unify
them. Fix in `descriptor.rs`: a small generic helper,
`fn collect<R: Report>(report: &R, ...)`, called once per report slice
instead of trying to iterate them all through one chained iterator.
Lesson: `impl Trait` buys you "some type that implements this trait",
not "the same type every time" — reach for a generic function over the
trait, not a `chain()`, when you need to treat several `impl Trait`
values uniformly.

## `thiserror` + `?` + `#[from]`

`opm-discovery::error::Error` uses `#[derive(thiserror::Error)]` with
one variant per failure source (`hidapi::HidError`, `std::io::Error`,
`hidreport::ParserError`), each with an `#[error("...")]` message
template and a `#[from]` on the wrapped value. The `#[from]` is what
makes `hidapi::HidApi::new()?` inside a function returning
`Result<_, Error>` just work — it generates the `From<hidapi::HidError>
for Error` impl that `?` uses to convert the error automatically.
Without `#[from]` I'd have needed `.map_err(Error::Backend)` at every
call site.

## Cloning an iterator to consume it twice

`classify.rs::classify()` needs to check two independent conditions
(is there a keyboard usage pair? is there a vendor one?) over the same
`flat_map` iterator. `.any()` consumes the iterator, so checking a
second condition on the same iterator variable doesn't compile — cloning
the iterator first (`usage_pairs.clone().any(...)`) solves it cheaply
here since the iterator only holds references, not owned data, so
cloning it is just copying a couple of pointers/indices, not the
underlying data.

## Letting `clap` validate instead of hand-rolling it

`pmctl discover --export <FORMAT>` only supports `json` for now. First
instinct was `Option<String>` plus a manual `if format != "json" { ... }`
check. Instead, `ExportFormat` is a small enum deriving
`clap::ValueEnum` (`commands/discover.rs`), and the field is
`Option<ExportFormat>`. `clap` then rejects `--export xml` itself, with
its own usage-error message and exit code (2) — matching
`discovery.md`'s exit-code table for free, and one less thing to test by
hand. General lesson: if a CLI flag has a fixed, small set of valid
values, model it as a type, don't validate a string.

## A hand-rolled date algorithm instead of a date crate

`--export`'s default output filename needs today's date
(`opm-discover-report-<YYYY-MM-DD>.json`). Pulling in `chrono` or `time`
for "what's today's UTC date" felt like a lot of dependency for one
`format!`. Used Howard Hinnant's `civil_from_days` algorithm instead
(`commands/discover/report.rs`) — a ~15-line, well-known, pure function
that converts days-since-1970-01-01 into a (year, month, day) triple,
with the days count coming from
`SystemTime::now().duration_since(UNIX_EPOCH)`. Worth remembering as a
pattern: a genuinely small, well-known algorithm can be cheaper than a
dependency, but it's exactly the kind of code that deserves a unit test
against known dates (see the `civil_from_days` test) since it's easy to
get an off-by-one wrong and never notice.

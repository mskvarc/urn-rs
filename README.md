# urn-rs

[![Crate informations](https://img.shields.io/crates/v/urn-rs.svg?style=flat-square)](https://crates.io/crates/urn-rs)
[![Crates.io MSRV](https://img.shields.io/crates/msrv/urn-rs?style=flat-square)](https://crates.io/crates/urn-rs)
[![License](https://img.shields.io/crates/l/urn-rs.svg?style=flat-square)](https://github.com/mskvarc/urn-rs#license)
[![Documentation](https://docs.rs/urn-rs/badge.svg)](https://docs.rs/urn-rs)

Rust crate for parsing, building, comparing, and percent-encoding [RFC 8141](https://datatracker.ietf.org/doc/html/rfc8141) / [RFC 2141](https://datatracker.ietf.org/doc/html/rfc2141) URNs.

Fork of [`urn`](https://crates.io/crates/urn) by [chayleaf](https://github.com/chayleaf/urn). Nearly all of the design and implementation is his work. This fork adds performance work, `Ord` impls, benches, and Rust 2024 edition uplift. See [Attribution](#attribution) below.

## Highlights vs upstream

- **Performance**: SWAR fast path for plain pchar runs, hex lookup tables for percent decode/encode, reduced allocations in builder / accessors / setters / serde paths. Criterion bench suite under `benches/` (`parse`, `percent`, `builder_accessors`, `setters`, `serde`).
- **`Ord` / `PartialOrd`** on `Urn` and `UrnSlice` (lexicographic on the canonical form).
- **Rust 2024 edition**, MSRV **1.85**.
- Crate renamed to `urn-rs` (library imported as `use urn_rs::…`).

Everything else, RFC parsing semantics, equivalence rules, percent-encoding behavior, `no_std`/`alloc` story, Serde support, matches upstream.

## Parsing & equivalence

Parsing and equality follow the spec: only the significant portion of the URN is compared (NID is ASCII-case-insensitive; NSS percent-encoding is normalized for comparison; r-/q-/f-components do not affect equality). Per-namespace lexical equivalence rules defined by individual RFCs are **not** applied.

## Features

| Feature | Default | Effect                                               |
| ------- | ------- | ---------------------------------------------------- |
| `std`   | yes     | enables `alloc`, adds `std::error::Error` impls      |
| `alloc` |         | owned `Urn`, builder, `String`-returning APIs        |
| `serde` |         | `Serialize` / `Deserialize` for `Urn` and `UrnSlice` |

`no_std` build: disable default features. With neither `std` nor `alloc` you get `UrnSlice<'a>` (borrowed, zero-alloc). Add `alloc` back for owned `Urn` and the builder.

## Types

- `UrnSlice<'a>` — borrowed URN, available without `alloc`.
- `Urn` — owned URN (requires `alloc`).
- `UrnBuilder` — validating builder (requires `alloc`).

## Examples

Parse and inspect:

```rust
use urn_rs::Urn;

let u: Urn = "urn:example:foo?=bar#frag".parse()?;
assert_eq!(u.nid(), "example");
assert_eq!(u.nss(), "foo");
```

Build:

```rust
use urn_rs::UrnBuilder;

let u = UrnBuilder::new("example", "weather/zurich").build()?;
assert_eq!(u.as_str(), "urn:example:weather/zurich");
```

## Benches

```sh
cargo bench
cargo bench --bench parse
cargo bench --features serde --bench serde
```

Criterion output lands in `target/criterion/`.

## MSRV

Rust **1.85** (edition 2024). Bumping MSRV is a minor-version bump.

## Attribution

Original crate: [`urn`](https://crates.io/crates/urn) by [chayleaf](https://github.com/chayleaf/urn). All upstream commits are preserved in this repo's history under their original authorship.

## License

Triple-licensed BSD0 / MIT / Apache-2.0, same as upstream. At your choice:

- [BSD Zero Clause License](LICENSE-0BSD.md) (<https://opensource.org/licenses/0BSD>)
- [Apache 2.0](LICENSE-APACHE.md) (<http://www.apache.org/licenses/LICENSE-2.0>)
- [MIT](LICENSE-MIT.md) (<http://opensource.org/licenses/MIT>)

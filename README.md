# urn-rs

[![Crate](https://img.shields.io/crates/v/urn-rs.svg?style=flat-square)](https://crates.io/crates/urn-rs)
[![Docs](https://img.shields.io/docsrs/urn-rs?style=flat-square)](https://docs.rs/urn-rs)
[![MSRV](https://img.shields.io/crates/msrv/urn-rs?style=flat-square)](https://crates.io/crates/urn-rs)
[![License](https://img.shields.io/crates/l/urn-rs.svg?style=flat-square)](#license)

A small, allocation-conscious Rust crate for [RFC 8141](https://datatracker.ietf.org/doc/html/rfc8141) / [RFC 2141](https://datatracker.ietf.org/doc/html/rfc2141) URNs — parse, build, compare, percent-encode.

```rust
use urn_rs::{Urn, UrnBuilder};

let u: Urn = "urn:example:weather/zurich?+ttl=60#frag".parse()?;
assert_eq!(u.nid(), "example");
assert_eq!(u.nss(), "weather/zurich");

let built = UrnBuilder::new("isbn", "0451450523").build()?;
assert_eq!(built.as_str(), "urn:isbn:0451450523");
```

---

## Why this fork

Fork of [`urn`](https://crates.io/crates/urn) by [chayleaf](https://github.com/chayleaf/urn). RFC behavior unchanged; this fork adds:

- SWAR fast path and hex lookup tables for percent decode/encode.
- Fewer allocations in builder, accessors, setters, serde.
- `Ord` / `PartialOrd` on `Urn` and `UrnSlice`.
- Criterion bench suite under `benches/`.
- Rust 2024 edition, MSRV 1.85.
- Renamed to `urn-rs`.

Credit and history preserved — see [Attribution](#attribution).

## Install

```sh
cargo add urn-rs
```

## Feature flags

| Flag       | Default | Enables                                                                    |
| ---------- | :-----: | -------------------------------------------------------------------------- |
| `std`      |   yes   | `alloc` + `std::error::Error` impls                                        |
| `alloc`    |         | Owned `Urn`, `UrnBuilder`, `String`-returning APIs                         |
| `serde`    |         | `Serialize` / `Deserialize` for `Urn` and `UrnSlice`                       |
| `exact-eq` |         | Include r-/q-/f-components in `PartialEq` / `Ord` / `Hash` (normalized)    |

For `no_std`, disable default features. You get `UrnSlice<'a>` (borrowed, zero-alloc). Add `alloc` back for `Urn` and the builder.

## Equivalence

By default, equality compares only the significant portion of the URN:

- NID is ASCII-case-insensitive.
- NSS percent-encoding is normalized before comparison.
- r-/q-/f-components are ignored.
- Per-namespace lexical-equivalence rules from individual RFCs are **not** applied.

Enable `exact-eq` when distinct r/q/f components must produce distinct map keys — equality then covers the whole normalized URN.

Because default equality can collapse distinct `as_str()` outputs to the same value, `Urn` and `UrnSlice` do **not** implement `Borrow<str>` — doing so would violate the `Borrow` / `Eq` / `Hash` contract. For `HashMap` lookup keyed by the raw string, key on `String` directly, or enable `exact-eq` to make the hash contract match `as_str()`.

## Benchmarks

```sh
cargo bench
cargo bench --bench parse
cargo bench --features serde --bench serde
```

Criterion output: `target/criterion/`.

## MSRV

Rust 1.85 (edition 2024).

## Attribution

Original crate: [`urn`](https://crates.io/crates/urn) by [chayleaf](https://github.com/chayleaf/urn). Upstream commits are preserved in this repo's history under their original authorship. This fork is a thin layer of performance and ergonomics work on top of their design.

## License

Triple-licensed, same as upstream. Pick whichever fits:

- [0BSD](https://github.com/mskvarc/urn-rs/blob/master/LICENSE-0BSD.md)
- [Apache-2.0](https://github.com/mskvarc/urn-rs/blob/master/LICENSE-APACHE.md)
- [MIT](https://github.com/mskvarc/urn-rs/blob/master/LICENSE-MIT.md)

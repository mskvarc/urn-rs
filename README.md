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

Fork of [`urn`](https://crates.io/crates/urn) by [chayleaf](https://github.com/chayleaf/urn). Compared with upstream, this fork:

- Follows RFC 8141 more closely. It rejects a NID that ends in `-` and the reserved NID `urn`, the encoders leave `~` and `&` as written, and the decoders reject an empty NSS, r- or q-component.
- Adds the `urn!` macro for compile-time checked literals (`static` feature).
- Adds `q_pairs`, `percent::decode_q_param` and `percent::encode_q_param` for `key=value` q-components.
- Uses a SWAR fast path and hex lookup tables for percent decode/encode.
- Allocates less in the builder, accessors, setters and serde. Deserializing a `UrnSlice` borrows from the input.
- Implements `Ord` / `PartialOrd` on `Urn` and `UrnSlice`.
- Is tested against an RFC 8141 conformance corpus, with proptest properties, cargo-fuzz targets and cargo-mutants.
- Has a Criterion bench suite under `crates/urn-rs-core/benches/`.
- Uses the Rust 2024 edition, MSRV 1.88.
- Is renamed to `urn-rs`.

Credit and history preserved — see [Attribution](#attribution).

## Install

```sh
cargo add urn-rs
```

## Feature flags

| Flag       | Default | Enables                                                                    |
| ---------- | :-----: | -------------------------------------------------------------------------- |
| `std`      |   yes   | `alloc` + links the standard library                                       |
| `alloc`    |         | Owned `Urn`, `UrnBuilder`, `String`-returning APIs                         |
| `serde`    |         | `Serialize` / `Deserialize` for `Urn` and `UrnSlice`                       |
| `static`   |         | `urn!` macro for compile-time checked URN literals                         |

For `no_std`, disable default features. You get `UrnSlice<'a>` (borrowed, zero-alloc). Add `alloc` back for `Urn` and the builder.

## Compile-time literals

The `static` feature adds `urn!`, which checks a URN literal at compile time:

```sh
cargo add urn-rs --features static
```

```rust
use urn_rs::{UrnSlice, urn};

const ISBN: UrnSlice<'static> = urn!("urn:isbn:0451450523");
```

An invalid literal is a compile error, and nothing is parsed at runtime. `UrnSlice::from_static` does the same check at runtime.

## Crates

| Crate                           | Contents                                          |
| ------------------------------- | ------------------------------------------------- |
| [`urn-rs`][crate-urn-rs]        | The crate to depend on. Re-exports the other two. |
| [`urn-rs-core`][crate-core]     | Types, parser, percent-encoding                   |
| [`urn-rs-static`][crate-static] | The `urn!` proc macro                             |

[crate-urn-rs]: https://github.com/mskvarc/urn-rs/tree/master/crates/urn-rs
[crate-core]: https://github.com/mskvarc/urn-rs/tree/master/crates/urn-rs-core
[crate-static]: https://github.com/mskvarc/urn-rs/tree/master/crates/urn-rs-static

## Equivalence

Equality compares only the significant portion of the URN (RFC 8141 §3):

- The `urn:` scheme and the NID are ASCII-case-insensitive.
- The NSS is compared byte for byte, except that the hex digits of `%XX` triplets are case-insensitive (`%2c` equals `%2C`). Triplets aren't decoded, so `a~b` and `a%7Eb` are different URNs.
- r-/q-/f-components are ignored.
- Per-namespace lexical-equivalence rules from individual RFCs are **not** applied.

Because equality can collapse distinct `as_str()` outputs to the same value, `Urn` and `UrnSlice` do **not** implement `Borrow<str>` — doing so would violate the `Borrow` / `Eq` / `Hash` contract. For `HashMap` lookup keyed by the raw string, or when distinct r/q/f components must produce distinct map keys, key on `String` directly.

## Benchmarks

```sh
cargo bench
cargo bench --bench parse
cargo bench --features serde --bench serde
```

Criterion output: `target/criterion/`.

## MSRV

Rust 1.88 (edition 2024).

## Attribution

Original crate: [`urn`](https://crates.io/crates/urn) by [chayleaf](https://github.com/chayleaf/urn). Upstream commits are preserved in this repo's history under their original authorship. This fork is a thin layer of performance and ergonomics work on top of their design.

## License

Triple-licensed, same as upstream. Pick whichever fits:

- [0BSD](https://github.com/mskvarc/urn-rs/blob/master/LICENSE-0BSD.md)
- [Apache-2.0](https://github.com/mskvarc/urn-rs/blob/master/LICENSE-APACHE.md)
- [MIT](https://github.com/mskvarc/urn-rs/blob/master/LICENSE-MIT.md)

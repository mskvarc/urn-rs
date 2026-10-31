# urn-rs-static

[![Crate](https://img.shields.io/crates/v/urn-rs-static.svg?style=flat-square)](https://crates.io/crates/urn-rs-static)
[![Docs](https://img.shields.io/docsrs/urn-rs-static?style=flat-square)](https://docs.rs/urn-rs-static)
[![MSRV](https://img.shields.io/crates/msrv/urn-rs-static?style=flat-square)](https://crates.io/crates/urn-rs-static)
[![License](https://img.shields.io/crates/l/urn-rs-static.svg?style=flat-square)](#license)

Compile-time checked URN literals for [`urn-rs`](../../README.md).

```rust
use urn_rs::UrnSlice;
use urn_rs_static::urn;

const ISBN: UrnSlice<'static> = urn!("urn:isbn:0451450523");
static WEATHER: UrnSlice<'static> = urn!("URN:Example:weather?=op=map");

assert_eq!(WEATHER.as_str(), "urn:example:weather?=op=map");
```

`urn!` validates the literal with the `urn-rs` parser during macro expansion. It normalizes the literal the same way the runtime parser does: the scheme and NID are lowercased, and percent-encoded hex digits are uppercased. It expands to a `const` block that builds the `UrnSlice<'static>` from precomputed component lengths, so there's no parsing or allocation at runtime.

An invalid literal fails to compile, and the error points at the literal:

```rust,compile_fail
let _ = urn_rs_static::urn!("urn:example:"); // error: invalid URN: invalid urn nss (namespace-specific string)
```

## Install

```sh
cargo add urn-rs urn-rs-static
```

The calling crate must depend on `urn-rs` directly, because the expansion refers to it. Renaming it in `Cargo.toml` is supported. Use the same release of both crates: the macro calls a hidden `urn-rs` constructor that isn't covered by semver.

## MSRV

Rust 1.85 (edition 2024).

## License

Triple-licensed, at your option:

- [0BSD](LICENSE-0BSD.md)
- [MIT](LICENSE-MIT.md)
- [Apache-2.0](LICENSE-APACHE.md)

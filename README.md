# urn-rs

[![Crate informations](https://img.shields.io/crates/v/urn-rs.svg?style=flat-square)](https://crates.io/crates/urn-rs)
[![Crates.io MSRV](https://img.shields.io/crates/msrv/urn-rs?style=flat-square)](https://crates.io/crates/urn-rs)
[![License](https://img.shields.io/crates/l/urn-rs.svg?style=flat-square)](https://github.com/mskvarc/urn-rs#license)
[![Documentation](https://docs.rs/urn-rs/badge.svg)](https://docs.rs/urn-rs)

Rust crate for parsing, building, comparing, and percent-encoding [RFC 8141](https://datatracker.ietf.org/doc/html/rfc8141) / [RFC 2141](https://datatracker.ietf.org/doc/html/rfc2141) URNs.

Fork of [`urn`](https://crates.io/crates/urn) by [chayleaf](https://github.com/chayleaf/urn). Nearly all of the design and implementation is his work. This fork adds pluggable namespaces, performance work, `Ord` impls, benches, and Rust 2024 edition uplift. See [Attribution](#attribution) below.

## Highlights vs upstream

- **Pluggable namespaces** (`src/namespace.rs`): `UrnNamespace` trait for typed NSS parsing/building. Built-in impls behind features:
  - `ngsi-ld` — `urn:ngsi-ld:<Type>:<Id>`, adds `as_ngsi_ld`, `ngsi_ld_type`, `ngsi_ld_id`, `Urn::try_from_ngsi_ld`.
  - `uuid` — canonical `urn:uuid:<8-4-4-4-12>` validation, borrowed string form (`as_uuid_str`, `try_from_uuid_str`).
  - `uuid-typed` — same + typed `::uuid::Uuid` round-trip (`as_uuid`, `try_from_uuid`). Pulls in the `uuid` crate.
- **Performance**: SWAR fast path for plain pchar runs, hex lookup tables for percent decode/encode, reduced allocations in builder / accessors / setters / serde paths. Criterion bench suite under `benches/` (`parse`, `percent`, `builder_accessors`, `setters`, `serde`).
- **`Ord` / `PartialOrd`** on `Urn` and `UrnSlice` (lexicographic on the canonical form).
- **Rust 2024 edition**, MSRV **1.85**.
- Crate renamed to `urn-rs` (library name `urn` preserved for `use` sites, see `Cargo.toml`).

Everything else, RFC parsing semantics, equivalence rules, percent-encoding behavior, `no_std`/`alloc` story, Serde support, matches upstream.

## Parsing & equivalence

Parsing and equality follow the spec: only the significant portion of the URN is compared (NID is ASCII-case-insensitive; NSS percent-encoding is normalized for comparison; r-/q-/f-components do not affect equality). Per-namespace lexical equivalence rules defined by individual RFCs are **not** applied.

## Features

| Feature      | Default | Effect                                               |
| ------------ | ------- | ---------------------------------------------------- |
| `std`        | yes     | enables `alloc`, adds `std::error::Error` impls      |
| `alloc`      |         | owned `Urn`, builder, `String`-returning APIs        |
| `serde`      |         | `Serialize` / `Deserialize` for `Urn` and `UrnSlice` |
| `ngsi-ld`    |         | NGSI-LD namespace helpers                            |
| `uuid`       |         | UUID NSS validation (string form)                    |
| `uuid-typed` |         | `uuid` + typed `::uuid::Uuid` round-trip             |

`no_std` build: disable default features. With neither `std` nor `alloc` you get `UrnSlice<'a>` (borrowed, zero-alloc). Add `alloc` back for owned `Urn` and the builder.

## Types

- `UrnSlice<'a>` — borrowed URN, available without `alloc`.
- `Urn` — owned URN (requires `alloc`).
- `UrnBuilder` — validating builder (requires `alloc`).
- `UrnNamespace` — trait for structured NSS types.

## Examples

Parse and inspect:

```rust
use urn::Urn;

let u: Urn = "urn:example:foo?=bar#frag".parse()?;
assert_eq!(u.nid(), "example");
assert_eq!(u.nss(), "foo");
```

Build:

```rust
use urn::UrnBuilder;

let u = UrnBuilder::new("example", "weather/zurich").build()?;
assert_eq!(u.as_str(), "urn:example:weather/zurich");
```

NGSI-LD (feature `ngsi-ld`):

```rust
use urn::Urn;

let u = Urn::try_from_ngsi_ld("Vehicle", "car1")?;
assert_eq!(u.as_str(), "urn:ngsi-ld:Vehicle:car1");
let p = u.as_ngsi_ld().unwrap();
assert_eq!((p.r#type, p.id), ("Vehicle", "car1"));
```

UUID typed (feature `uuid-typed`):

```rust
use urn::Urn;

let raw: uuid::Uuid = "f47ac10b-58cc-4372-a567-0e02b2c3d479".parse()?;
let u = Urn::try_from_uuid(raw)?;
assert_eq!(u.as_uuid(), Some(raw));
```

Custom namespace:

```rust
use urn::{Urn, UrnBuilder, namespace::UrnNamespace};

struct Isbn;
impl UrnNamespace for Isbn {
    const NID: &'static str = "isbn";
    type Parts<'a> = &'a str;
    fn parse_nss(nss: &str) -> Option<&str> {
        (!nss.is_empty()).then_some(nss)
    }
    fn write_nss(p: &&str, out: &mut String) { out.push_str(p); }
}

let u = Urn::try_from("urn:isbn:0451450523")?;
assert_eq!(u.parts::<Isbn>(), Some("0451450523"));
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

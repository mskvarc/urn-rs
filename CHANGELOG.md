# Changelog

## [1.0.0](https://github.com/mskvarc/urn-rs/compare/v0.9.2...v1.0.0) - 2026-11-12

### Highlights

The first stable release. urn-rs now follows [RFC 8141](https://datatracker.ietf.org/doc/html/rfc8141) more closely, and it is split into three crates: depend on `urn-rs`, which re-exports `urn-rs-core` and, with the `static` feature, the macro from `urn-rs-static`.

- `urn!("urn:isbn:0451450523")` checks a URN literal at compile time and builds a `UrnSlice<'static>`, with nothing parsed at runtime. It needs the `static` feature.
- `UrnSlice::q_pairs` iterates over the `key=value` pairs of a q-component. `percent::decode_q_param` and `percent::encode_q_param` decode and encode a single key or value.
- Deserializing a `UrnSlice` borrows from the input when the deserializer lends the string, as `serde_json::from_str` does. In a derived struct, mark the field `#[serde(borrow)]`.
- `UrnSlice::into_owned` turns a slice into a `Urn`, and `Urn::into_string` returns the `String`.

### Upgrade notes

- The minimum supported Rust version is 1.88.
- The encoders leave `~` and `&` as written instead of percent-encoding them, as RFC 3986 and RFC 8141 ask. Equivalence never decodes `%XX` triplets, so a URN encoded from the same input can differ from one 0.9 produced: `a~b` and `a%7Eb` are different URNs. A namespace registered under RFC 2141 may still expect `%7E` and `%26`.
- A NID that ends in `-` is rejected, and so is the reserved NID `urn`.
- `UrnBuilder` lowercases the NID, as parsing already did.
- `decode_nss`, `decode_r_component` and `decode_q_component`, and their `_iter` forms, reject empty input.
- `encode_nss_iter`, `encode_r_component_iter`, `encode_q_component_iter` and `encode_f_component_iter` return `Result<EncodeIter, Error>`. Like the allocating encoders, the NSS, r- and q-component ones reject empty input.
- `UrnSlice` implements `Clone`, so `slice.to_owned()` now returns a `UrnSlice` instead of a `Urn`. Use `slice.into_owned()` or `Urn::from(&slice)` for an owned `Urn`.
- The `exact-eq` feature is removed. Equality, ordering and hashing follow RFC 8141 §3 only. To key a map on the exact string, r-, q- and f-components included, key it on `as_str()` or a `String`.
- `urn-rs`, `urn-rs-core` and `urn-rs-static` require each other at the exact same version. Depend on `urn-rs` alone; if you also depend on one of the others, keep it at the same version.

### Changes

#### ⚠️ Breaking changes

- bump msrv to 1.88
- reject empty input in encode iterators
- drop exact-eq feature
- stop percent-encoding ~ and & in encoders
- implement clone for urnslice. UrnSlice no longer has a custom ToOwned impl. to_owned()
  now returns UrnSlice<'a> instead of Urn. Use Urn::from(&slice) or .into()
  to get an owned Urn.

#### 🚀 Features

- add q-component pair encoder
- add urn! macro behind a static feature
- add urn-rs-static crate with compile-time checked urn! macro
- add q-component pairs iterator and pair decoder
- expose into_owned and add into_string for urn

#### 🐛 Bug Fixes

- borrow from the input when deserializing urnslice
- reject empty nss, r and q in percent decoders
- lowercase nid in urnbuilder
- reject reserved nid urn
- reject nid ending in hyphen

#### 🔧 Refactor

- split parser internals out of the percent module

#### 📚 Documentation

- link discussions from the issue forms
- add contributing guide, security policy, code of conduct and issue forms
- fix crates.io links and refresh readme
- fix intra-doc links in builds without alloc

#### 🏗️ Build

- pin the toolchain
- commit the lock files
- pin urn-rs crates to the exact same release

#### 👷 CI

- open release notes with the tag message
- add workflows, cargo-deny and dependabot updates

#### 🧪 Tests

- kill surviving mutants and document equivalent ones
- add cargo-fuzz targets for parsing, percent-encoding and setters
- add proptest properties against an rfc 8141 reference parser
- add rfc 8141 conformance corpus

#### 🎨 Style

- format code in doc comments
- format toml files with taplo
- format the fuzz crate

#### 🧹 Chores

- keep stray commits out of the changelog
- ignore .ds_store
- update contact info
- update dependencies
- share manifest fields across the workspace
- tighten clippy lints

## [0.9.2](https://github.com/mskvarc/urn-rs/compare/v0.9.1...v0.9.2) - 2026-04-22

### Changes

#### 🚀 Features

- encode iterators
- add slice handling
- add tryfrom implementations for byte slices

#### 🧪 Tests

- add missing tests

#### 🎨 Style

- reformat code

#### 🧹 Chores

- bump version

## [0.9.1](https://github.com/mskvarc/urn-rs/compare/v0.9.0...v0.9.1) - 2026-04-22

### Changes

#### 🐛 Bug Fixes

- clippy warnings
- misc clippy warnings

#### 🔧 Refactor

- rework error handling

#### 📚 Documentation

- expand doc comments
- eq clarification
- update changelog

#### 🧪 Tests

- make all tests pass again

#### 🎨 Style

- format code
- reformat code

#### 🧹 Chores

- bump version
- de-duplica nid checks

## [0.9.0](https://github.com/mskvarc/urn-rs/compare/v0.8.0...v0.9.0) - 2026-04-22

### Changes

#### 🚀 Features

- batch scan in percent encoding
- add exact equivalence features

#### 🐛 Bug Fixes

- clippy warnings

#### ⚡ Performance

- more swar tweaks
- add inline attribute to accessors

#### 🔧 Refactor

- remove double validation in builder
- kill double validation in setters
- drop urn namespace functionality

#### 📚 Documentation

- rewrite readme
- update changelog

#### 🧹 Chores

- bump version

## [0.8.0](https://github.com/mskvarc/urn-rs/compare/v0.7.0...v0.8.0) - 2026-04-21

### Changes

#### 🚀 Features

- bump rust edition
- add explicit namespaces
- implement ord and partialord

#### 🐛 Bug Fixes

- make doctests pass
- clippy warnings
- error in core is in stable

#### ⚡ Performance

- swar for plain-pchar runs
- use hex lookup tables
- more performance improvements
- add performance fixes

#### 🔧 Refactor

- markdown licenses
- cargo manifest
- drop nightly feature
- crate rename
- dependencies and versions

#### 📚 Documentation

- update lib name in readme
- add changelog
- add badges to readme
- rewrite readme

#### 🏗️ Build

- add clif configuration
- add rustfmt configuration

#### 👷 CI

- add dependabot

#### 🎨 Style

- reformat code
- license formatting

#### 🧹 Chores

- update description
- more improvements
- add bench suite

## [0.7.0](https://github.com/mskvarc/urn-rs/releases/tag/v0.7.0) - 2023-04-08

The last release of the upstream [`urn`](https://crates.io/crates/urn) crate by chayleaf, where this fork starts.

<!-- generated by git-cliff -->

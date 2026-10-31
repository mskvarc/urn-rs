# urn-rs-core

Core types and parser behind [`urn-rs`](../../README.md).

Depend on `urn-rs` instead: it re-exports everything in this crate, and its `static` feature adds the compile-time checked `urn!` macro. This crate is separate only so the macro crate can use the parser without a dependency cycle.

## License

Triple-licensed, at your option:

- [0BSD](LICENSE-0BSD.md)
- [MIT](LICENSE-MIT.md)
- [Apache-2.0](LICENSE-APACHE.md)

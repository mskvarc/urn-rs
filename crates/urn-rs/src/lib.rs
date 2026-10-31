//! A crate for handling [URNs](https://datatracker.ietf.org/doc/html/rfc8141).
//!
//! Features
//! - `serde` - [Serde](https://serde.rs) support
//! - `std` (enabled by default) - links the standard library; implies `alloc`. [`Error`]
//!   implements [`core::error::Error`] in every build, `no_std` included.
//! - `alloc` (enabled by default) - [alloc](https://doc.rust-lang.org/alloc/index.html) support
//!   (you probably want to keep this enabled)
//! - `exact-eq` - compare, hash and order the whole URN, r-, q- and f-components included,
//!   instead of only the RFC 8141 equivalence part (scheme, NID and NSS)
//! - `static` - the `urn!` macro, which checks a URN literal at compile time
//!
//! # Example
//! ```
//! # #[cfg(not(feature = "std"))]
//! # fn main() { }
//! # #[cfg(feature = "std")]
//! # use urn_rs::{Urn, UrnSlice, UrnBuilder};
//! # #[cfg(feature = "std")]
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let urn = UrnBuilder::new("example", "1234:5678").build()?;
//! assert_eq!(urn.as_str(), "urn:example:1234:5678");
//! assert_eq!(urn, "urn:example:1234:5678".parse::<Urn>()?); // Using std::str::parse
//! assert_eq!(urn.nss(), "1234:5678");
//! # Ok(())
//! # }
//! ```

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use urn_rs_core::*;

#[doc(hidden)]
pub use urn_rs_core as __private;

/// Validate a URN literal at compile time and expand to a `UrnSlice<'static>`.
///
/// See [`urn_rs_static::urn`] for details.
///
/// ```
/// use urn_rs::{UrnSlice, urn};
///
/// const ISBN: UrnSlice<'static> = urn!("urn:isbn:0451450523");
/// assert_eq!(ISBN.nid(), "isbn");
/// ```
#[cfg(feature = "static")]
#[cfg_attr(docsrs, doc(cfg(feature = "static")))]
pub use urn_rs_static::urn;

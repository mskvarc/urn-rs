//! Compile-time checked URN literals for [`urn-rs`](https://docs.rs/urn-rs).
//!
//! [`urn!`] validates a string literal with the `urn-rs` parser during macro expansion and
//! expands to a `const` expression of type `UrnSlice<'static>`. An invalid literal is a compile
//! error, and nothing is parsed or allocated at runtime.
//!
//! ```
//! use urn_rs::UrnSlice;
//! use urn_rs_static::urn;
//!
//! const ISBN: UrnSlice<'static> = urn!("urn:isbn:0451450523");
//! assert_eq!(ISBN.nid(), "isbn");
//! assert_eq!(ISBN.nss(), "0451450523");
//! ```
//!
//! ```compile_fail
//! let _ = urn_rs_static::urn!("urn:x:nss"); // NID too short
//! ```
//!
//! The calling crate must depend on `urn-rs` directly, since the expansion refers to it.
//! Renaming it in `Cargo.toml` is supported.

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{Error as SynError, LitStr, parse_macro_input};
use urn_rs::UrnSlice;

/// Path to `urn-rs` as the calling crate names it.
fn urn_rs_path() -> TokenStream2 {
    match crate_name("urn-rs") {
        Ok(FoundCrate::Name(name)) => {
            let ident = format_ident!("{name}");
            quote!(::#ident)
        }
        // `Itself` means urn-rs's own tests or examples, which reach the library as `urn_rs`.
        Ok(FoundCrate::Itself) | Err(_) => quote!(::urn_rs),
    }
}

/// Validate a URN literal at compile time and expand to a `UrnSlice<'static>`.
///
/// The URN is normalized during expansion, just like at runtime: the `urn:` scheme and NID are
/// lowercased, and the hex digits of every `%XX` triplet are uppercased. The expansion is a
/// `const` block, so it can initialize a `const` or a `static`.
///
/// ```
/// use urn_rs::UrnSlice;
/// use urn_rs_static::urn;
///
/// static WEATHER: UrnSlice<'static> = urn!("URN:Example:weather%2f?+r?=op=map#f");
/// assert_eq!(WEATHER.as_str(), "urn:example:weather%2F?+r?=op=map#f");
/// assert_eq!(WEATHER.r_component(), Some("r"));
/// assert_eq!(WEATHER.q_component(), Some("op=map"));
/// assert_eq!(WEATHER.f_component(), Some("f"));
/// ```
///
/// An invalid literal is reported at the literal, with the parser's error:
///
/// ```compile_fail
/// let _ = urn_rs_static::urn!("urn:example:"); // empty NSS
/// ```
#[proc_macro]
pub fn urn(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let value = lit.value();
    let urn = match UrnSlice::try_from(value.as_str()) {
        Ok(urn) => urn,
        Err(err) => return SynError::new(lit.span(), format!("invalid URN: {err}")).to_compile_error().into(),
    };
    let krate = urn_rs_path();
    let s = urn.as_str();
    let nid_len = Literal::usize_unsuffixed(urn.nid().len());
    let nss_len = Literal::usize_unsuffixed(urn.nss().len());
    let r_component_len = Literal::usize_unsuffixed(urn.r_component().map_or(0, str::len));
    let q_component_len = Literal::usize_unsuffixed(urn.q_component().map_or(0, str::len));
    quote! {
        const { #krate::UrnSlice::__from_static_parts(#s, #nid_len, #nss_len, #r_component_len, #q_component_len) }
    }
    .into()
}

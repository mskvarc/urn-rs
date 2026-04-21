//! Pluggable URN namespace support.
//!
//! Define [`UrnNamespace`] for a type to teach this crate how to parse and construct URNs whose
//! NSS has internal structure (e.g. `urn:ngsi-ld:<Type>:<Id>`, `urn:uuid:<UUID>`).
//!
//! Built-in namespaces are gated behind cargo features:
//! - `ngsi-ld` — [`NgsiLd`]
//! - `uuid` — [`Uuid`] (borrowed canonical string form)
//! - `uuid-typed` — additionally depend on the `uuid` crate and expose owned [`::uuid::Uuid`] parts

#[cfg(feature = "alloc")]
use crate::UrnBuilder;
use crate::UrnSlice;
#[cfg(feature = "alloc")]
use crate::{Result, Urn};
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::string::String;

/// A URN namespace with structured NSS.
///
/// Implement this for a marker type to enable [`UrnSlice::parts`] and
/// [`UrnBuilder::from_parts`] for URNs in that namespace.
pub trait UrnNamespace {
    /// The namespace identifier (NID). Must be ASCII lowercase.
    const NID: &'static str;
    /// The structured representation of the NSS. `'a` is the lifetime of the borrowed URN.
    type Parts<'a>;
    /// Parse an already-validated NSS. Return `None` if the NSS does not match this
    /// namespace's shape.
    fn parse_nss(nss: &str) -> Option<Self::Parts<'_>>;
    /// Compose NSS into `out`. `out` is empty on entry. The result must round-trip through
    /// [`UrnNamespace::parse_nss`] and pass this crate's generic NSS validator.
    #[cfg(feature = "alloc")]
    fn write_nss(parts: &Self::Parts<'_>, out: &mut String);
}

impl UrnSlice<'_> {
    /// Decode the NSS according to `N`. Returns `None` if the NID does not match `N::NID`
    /// or the NSS does not match `N`'s expected shape.
    #[must_use]
    pub fn parts<N: UrnNamespace>(&self) -> Option<N::Parts<'_>> {
        if !self.nid().eq_ignore_ascii_case(N::NID) {
            return None;
        }
        N::parse_nss(self.nss())
    }
}

#[cfg(feature = "alloc")]
impl UrnBuilder<'_> {
    /// Build a [`Urn`] from the structured parts of namespace `N`.
    ///
    /// The produced NSS is run through this crate's generic validator, so malformed output
    /// from a bad impl is caught here.
    ///
    /// # Errors
    /// Returns an error if the composed NSS fails validation.
    pub fn from_parts<N: UrnNamespace>(parts: N::Parts<'_>) -> Result<Urn> {
        let mut nss = String::new();
        N::write_nss(&parts, &mut nss);
        UrnBuilder::new(N::NID, &nss).build()
    }
}

// --- ngsi-ld ---------------------------------------------------------------

/// The ETSI NGSI-LD namespace: `urn:ngsi-ld:<Type>:<Id>`.
#[cfg(feature = "ngsi-ld")]
#[cfg_attr(docsrs, doc(cfg(feature = "ngsi-ld")))]
pub struct NgsiLd;

/// Structured parts of an NGSI-LD URN.
#[cfg(feature = "ngsi-ld")]
#[cfg_attr(docsrs, doc(cfg(feature = "ngsi-ld")))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NgsiLdParts<'a> {
    /// Entity type, e.g. `Vehicle`.
    pub r#type: &'a str,
    /// Entity id, e.g. `car1`. May itself contain `:` — everything after the first `:` is id.
    pub id: &'a str,
}

#[cfg(feature = "ngsi-ld")]
impl UrnNamespace for NgsiLd {
    const NID: &'static str = "ngsi-ld";
    type Parts<'a> = NgsiLdParts<'a>;
    fn parse_nss(nss: &str) -> Option<NgsiLdParts<'_>> {
        let (t, id) = nss.split_once(':')?;
        if t.is_empty() || id.is_empty() {
            return None;
        }
        Some(NgsiLdParts { r#type: t, id })
    }
    #[cfg(feature = "alloc")]
    fn write_nss(p: &NgsiLdParts<'_>, out: &mut String) {
        out.push_str(p.r#type);
        out.push(':');
        out.push_str(p.id);
    }
}

#[cfg(feature = "ngsi-ld")]
impl UrnSlice<'_> {
    /// Parse this URN as an NGSI-LD URN. Returns `None` if NID isn't `ngsi-ld` or NSS has no `:`.
    #[must_use]
    #[cfg_attr(docsrs, doc(cfg(feature = "ngsi-ld")))]
    pub fn as_ngsi_ld(&self) -> Option<NgsiLdParts<'_>> {
        self.parts::<NgsiLd>()
    }
    /// NGSI-LD entity type, or `None`.
    #[must_use]
    #[cfg_attr(docsrs, doc(cfg(feature = "ngsi-ld")))]
    pub fn ngsi_ld_type(&self) -> Option<&str> {
        self.as_ngsi_ld().map(|p| p.r#type)
    }
    /// NGSI-LD entity id, or `None`.
    #[must_use]
    #[cfg_attr(docsrs, doc(cfg(feature = "ngsi-ld")))]
    pub fn ngsi_ld_id(&self) -> Option<&str> {
        self.as_ngsi_ld().map(|p| p.id)
    }
}

#[cfg(all(feature = "ngsi-ld", feature = "alloc"))]
impl Urn {
    /// Build `urn:ngsi-ld:<type_>:<id>`. Both parts must already be percent-encoded per NSS rules.
    ///
    /// # Errors
    /// Returns an error if `type_` or `id` contains bytes that aren't valid in an NSS.
    #[cfg_attr(docsrs, doc(cfg(all(feature = "ngsi-ld", feature = "alloc"))))]
    pub fn try_from_ngsi_ld(type_: &str, id: &str) -> Result<Urn> {
        UrnBuilder::from_parts::<NgsiLd>(NgsiLdParts { r#type: type_, id })
    }
}

// --- uuid (string form) ----------------------------------------------------

/// The RFC 4122 UUID namespace: `urn:uuid:<uuid>`. Canonical 8-4-4-4-12 hex form only.
#[cfg(feature = "uuid")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid")))]
pub struct Uuid;

#[cfg(feature = "uuid")]
fn is_canonical_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 36 {
        return false;
    }
    for (i, &c) in b.iter().enumerate() {
        match i {
            8 | 13 | 18 | 23 => {
                if c != b'-' {
                    return false;
                }
            }
            _ => {
                if !c.is_ascii_hexdigit() {
                    return false;
                }
            }
        }
    }
    true
}

#[cfg(all(feature = "uuid", not(feature = "uuid-typed")))]
impl UrnNamespace for Uuid {
    const NID: &'static str = "uuid";
    type Parts<'a> = &'a str;
    fn parse_nss(nss: &str) -> Option<&str> {
        if is_canonical_uuid(nss) { Some(nss) } else { None }
    }
    #[cfg(feature = "alloc")]
    fn write_nss(p: &&str, out: &mut String) {
        out.push_str(p);
    }
}

#[cfg(all(feature = "uuid", not(feature = "uuid-typed")))]
impl UrnSlice<'_> {
    /// UUID as a borrowed canonical string, or `None`.
    #[must_use]
    #[cfg_attr(docsrs, doc(cfg(feature = "uuid")))]
    pub fn as_uuid_str(&self) -> Option<&str> {
        self.parts::<Uuid>()
    }
}

#[cfg(all(feature = "uuid", not(feature = "uuid-typed"), feature = "alloc"))]
impl Urn {
    /// Build `urn:uuid:<s>`. `s` must be a canonical 8-4-4-4-12 hex UUID.
    ///
    /// # Errors
    /// Returns an error if `s` is not a canonical UUID.
    #[cfg_attr(docsrs, doc(cfg(all(feature = "uuid", feature = "alloc"))))]
    pub fn try_from_uuid_str(s: &str) -> Result<Urn> {
        UrnBuilder::from_parts::<Uuid>(s)
    }
}

// --- uuid-typed ------------------------------------------------------------

#[cfg(feature = "uuid-typed")]
impl UrnNamespace for Uuid {
    const NID: &'static str = "uuid";
    type Parts<'a> = ::uuid::Uuid;
    fn parse_nss(nss: &str) -> Option<::uuid::Uuid> {
        if !is_canonical_uuid(nss) {
            return None;
        }
        ::uuid::Uuid::parse_str(nss).ok()
    }
    #[cfg(feature = "alloc")]
    fn write_nss(p: &::uuid::Uuid, out: &mut String) {
        use core::fmt::Write;
        // hyphenated is the canonical 36-char form
        let _ = write!(out, "{}", p.as_hyphenated());
    }
}

#[cfg(feature = "uuid-typed")]
impl UrnSlice<'_> {
    /// UUID as an owned [`::uuid::Uuid`], or `None`.
    #[must_use]
    #[cfg_attr(docsrs, doc(cfg(feature = "uuid-typed")))]
    pub fn as_uuid(&self) -> Option<::uuid::Uuid> {
        self.parts::<Uuid>()
    }
    /// UUID as a borrowed canonical string, or `None`.
    #[must_use]
    #[cfg_attr(docsrs, doc(cfg(feature = "uuid-typed")))]
    pub fn as_uuid_str(&self) -> Option<&str> {
        if self.nid().eq_ignore_ascii_case(<Uuid as UrnNamespace>::NID) && is_canonical_uuid(self.nss()) {
            Some(self.nss())
        } else {
            None
        }
    }
}

#[cfg(all(feature = "uuid-typed", feature = "alloc"))]
impl Urn {
    /// Build `urn:uuid:<u>` from a typed UUID.
    ///
    /// # Errors
    /// Only fails under catastrophic validator regression — canonical-hyphenated output
    /// is always a valid NSS.
    #[cfg_attr(docsrs, doc(cfg(all(feature = "uuid-typed", feature = "alloc"))))]
    pub fn try_from_uuid(u: ::uuid::Uuid) -> Result<Urn> {
        UrnBuilder::from_parts::<Uuid>(u)
    }
    /// Build `urn:uuid:<s>`. `s` must be a canonical 8-4-4-4-12 hex UUID.
    ///
    /// # Errors
    /// Returns an error if `s` is not a canonical UUID.
    #[cfg_attr(docsrs, doc(cfg(all(feature = "uuid-typed", feature = "alloc"))))]
    pub fn try_from_uuid_str(s: &str) -> Result<Urn> {
        let u = ::uuid::Uuid::parse_str(s).map_err(|_| crate::Error::InvalidNss)?;
        Self::try_from_uuid(u)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "alloc", any(feature = "ngsi-ld", feature = "uuid", feature = "uuid-typed")))]
    use super::*;

    #[cfg(all(feature = "ngsi-ld", feature = "alloc"))]
    #[test]
    fn ngsi_ld_roundtrip() {
        let u = Urn::try_from("urn:ngsi-ld:Vehicle:car1").unwrap();
        let p = u.as_ngsi_ld().unwrap();
        assert_eq!(p.r#type, "Vehicle");
        assert_eq!(p.id, "car1");
        assert_eq!(u.ngsi_ld_type(), Some("Vehicle"));
        assert_eq!(u.ngsi_ld_id(), Some("car1"));

        let built = Urn::try_from_ngsi_ld("Vehicle", "car1").unwrap();
        assert_eq!(built.as_str(), "urn:ngsi-ld:Vehicle:car1");
    }

    #[cfg(all(feature = "ngsi-ld", feature = "alloc"))]
    #[test]
    fn ngsi_ld_wrong_nid() {
        let u = Urn::try_from("urn:example:foo:bar").unwrap();
        assert_eq!(u.as_ngsi_ld(), None);
    }

    #[cfg(all(feature = "ngsi-ld", feature = "alloc"))]
    #[test]
    fn ngsi_ld_no_colon() {
        let u = Urn::try_from("urn:ngsi-ld:lonely").unwrap();
        assert_eq!(u.as_ngsi_ld(), None);
    }

    #[cfg(all(feature = "uuid", feature = "alloc"))]
    #[test]
    fn uuid_roundtrip_str() {
        let s = "f47ac10b-58cc-4372-a567-0e02b2c3d479";
        let u = Urn::try_from_uuid_str(s).unwrap();
        assert_eq!(u.as_str(), "urn:uuid:f47ac10b-58cc-4372-a567-0e02b2c3d479");
        assert_eq!(u.as_uuid_str(), Some(s));
    }

    #[cfg(all(feature = "uuid", feature = "alloc"))]
    #[test]
    fn uuid_non_canonical() {
        let u = Urn::try_from("urn:uuid:notauuid").unwrap();
        assert_eq!(u.as_uuid_str(), None);
    }

    #[cfg(all(feature = "uuid-typed", feature = "alloc"))]
    #[test]
    fn uuid_typed_roundtrip() {
        let raw: ::uuid::Uuid = "f47ac10b-58cc-4372-a567-0e02b2c3d479".parse().unwrap();
        let u = Urn::try_from_uuid(raw).unwrap();
        assert_eq!(u.as_uuid(), Some(raw));
    }

    #[cfg(all(feature = "alloc", feature = "ngsi-ld"))]
    #[test]
    fn custom_namespace_extensibility() {
        struct Isbn;
        impl UrnNamespace for Isbn {
            const NID: &'static str = "isbn";
            type Parts<'a> = &'a str;
            fn parse_nss(nss: &str) -> Option<&str> {
                (!nss.is_empty()).then_some(nss)
            }
            fn write_nss(p: &&str, out: &mut String) {
                out.push_str(p);
            }
        }
        let u = Urn::try_from("urn:isbn:0451450523").unwrap();
        assert_eq!(u.parts::<Isbn>(), Some("0451450523"));
        let built = UrnBuilder::from_parts::<Isbn>("0451450523").unwrap();
        assert_eq!(built.as_str(), "urn:isbn:0451450523");
    }
}

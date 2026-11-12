#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{string::String, vec::Vec};
use core::fmt;

use serde::de::{Deserialize, Deserializer, Error as DeError, Visitor};

use crate::UrnSlice;

#[cfg(feature = "alloc")]
use crate::Urn;

/// Turns the string a deserializer hands over into a `UrnSlice`. Input borrowed for `'de` stays
/// borrowed; anything shorter-lived is copied, which needs `alloc`. Bytes are accepted too, as
/// long as they're UTF-8.
struct UrnSliceVisitor;

impl<'de> Visitor<'de> for UrnSliceVisitor {
    type Value = UrnSlice<'de>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a URN string")
    }

    fn visit_borrowed_str<E: DeError>(self, v: &'de str) -> Result<Self::Value, E> {
        UrnSlice::try_from(v).map_err(E::custom)
    }

    fn visit_borrowed_bytes<E: DeError>(self, v: &'de [u8]) -> Result<Self::Value, E> {
        UrnSlice::try_from(v).map_err(E::custom)
    }

    /// Validates before copying, so a rejected URN costs no allocation.
    #[cfg(feature = "alloc")]
    fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
        UrnSlice::try_from(v).map(|urn| urn.into_owned().0).map_err(E::custom)
    }

    #[cfg(feature = "alloc")]
    fn visit_string<E: DeError>(self, v: String) -> Result<Self::Value, E> {
        UrnSlice::try_from(v).map_err(E::custom)
    }

    #[cfg(feature = "alloc")]
    fn visit_bytes<E: DeError>(self, v: &[u8]) -> Result<Self::Value, E> {
        UrnSlice::try_from(v).map(|urn| urn.into_owned().0).map_err(E::custom)
    }

    #[cfg(feature = "alloc")]
    fn visit_byte_buf<E: DeError>(self, v: Vec<u8>) -> Result<Self::Value, E> {
        UrnSlice::try_from(v).map_err(E::custom)
    }
}

/// Borrows from the input when the deserializer lends the string, as `serde_json::from_str`
/// does for a string without escapes, and the URN is already normalized (see
/// [`UrnSlice::as_str`]). Otherwise the slice owns a copy, which needs the `alloc` feature.
///
/// In a derived struct, mark the field `#[serde(borrow)]`:
///
/// ```
/// # fn main() -> Result<(), serde_json::Error> {
/// use serde::Deserialize;
/// use urn_rs::UrnSlice;
///
/// #[derive(Deserialize)]
/// struct Record<'a> {
///     #[serde(borrow)]
///     id: UrnSlice<'a>,
/// }
///
/// let json = r#"{"id":"urn:example:foo"}"#;
/// let record: Record<'_> = serde_json::from_str(json)?;
/// assert_eq!(record.id.nss(), "foo");
/// # Ok(()) }
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<'de: 'a, 'a> Deserialize<'de> for UrnSlice<'a> {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        de.deserialize_str(UrnSliceVisitor)
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl serde::Serialize for UrnSlice<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<'de> Deserialize<'de> for Urn {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        UrnSlice::deserialize(de).map(UrnSlice::into_owned)
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl serde::Serialize for Urn {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    use alloc::{borrow::ToOwned, string::ToString, vec::Vec};

    use serde::{
        Deserialize,
        de::{Visitor, value::Error as ValueError},
    };

    use super::UrnSliceVisitor;
    #[cfg(feature = "alloc")]
    use crate::Urn;
    use crate::{UrnSlice, cow::TriCow};

    const NORMALIZED: &str = "urn:example:a%2Fb?+r?=q#f";
    const DENORMALIZED: &str = "URN:Example:a%2fb?+r?=q#f";
    /// `NORMALIZED` as a JSON string.
    const JSON: &str = r#""urn:example:a%2Fb?+r?=q#f""#;

    fn is_borrowed(urn: &UrnSlice<'_>) -> bool {
        matches!(urn.urn, TriCow::Borrowed(_))
    }

    #[derive(Deserialize)]
    struct Record<'a> {
        #[serde(borrow)]
        id: UrnSlice<'a>,
    }

    #[test]
    fn borrowed_input_stays_borrowed() {
        let urn = UrnSliceVisitor.visit_borrowed_str::<ValueError>(NORMALIZED).unwrap();
        assert!(is_borrowed(&urn));
        assert_eq!(urn.as_str(), NORMALIZED);

        let urn = UrnSliceVisitor.visit_borrowed_bytes::<ValueError>(NORMALIZED.as_bytes()).unwrap();
        assert!(is_borrowed(&urn));
        assert_eq!(urn.as_str(), NORMALIZED);

        let record: Record<'_> = serde_json::from_str(r#"{"id":"urn:example:a%2Fb?+r?=q#f"}"#).unwrap();
        assert!(is_borrowed(&record.id));
        assert_eq!(record.id.as_str(), NORMALIZED);
    }

    #[test]
    fn rejects_invalid_input() {
        assert!(UrnSliceVisitor.visit_borrowed_str::<ValueError>("urn:x:nss").is_err());
        assert!(UrnSliceVisitor.visit_borrowed_bytes::<ValueError>(b"urn:example:\xff").is_err());
        assert!(serde_json::from_str::<Record<'_>>(r#"{"id":"urn:example:"}"#).is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn copies_what_it_cannot_borrow() {
        // Borrowed but not normalized: normalizing needs a copy.
        let urn = UrnSliceVisitor.visit_borrowed_str::<ValueError>(DENORMALIZED).unwrap();
        assert!(!is_borrowed(&urn));
        assert_eq!(urn.as_str(), NORMALIZED);

        for urn in [
            UrnSliceVisitor.visit_str::<ValueError>(DENORMALIZED).unwrap(),
            UrnSliceVisitor.visit_str::<ValueError>(NORMALIZED).unwrap(),
            UrnSliceVisitor.visit_string::<ValueError>(DENORMALIZED.to_owned()).unwrap(),
            UrnSliceVisitor.visit_bytes::<ValueError>(DENORMALIZED.as_bytes()).unwrap(),
            UrnSliceVisitor.visit_bytes::<ValueError>(NORMALIZED.as_bytes()).unwrap(),
            UrnSliceVisitor.visit_byte_buf::<ValueError>(DENORMALIZED.as_bytes().to_vec()).unwrap(),
        ] {
            assert!(!is_borrowed(&urn));
            assert_eq!(urn.as_str(), NORMALIZED);
        }

        // serde_json reads a reader's strings into a scratch buffer, which can't be borrowed.
        let urn = UrnSlice::deserialize(&mut serde_json::Deserializer::from_reader(JSON.as_bytes())).unwrap();
        assert!(!is_borrowed(&urn));
        assert_eq!(urn.as_str(), NORMALIZED);

        assert_eq!(
            UrnSliceVisitor.visit_bytes::<ValueError>(b"urn:example:\xff").unwrap_err().to_string(),
            "urn contains invalid utf-8"
        );
        assert!(UrnSliceVisitor.visit_str::<ValueError>("urn:example:").is_err());
        assert!(UrnSliceVisitor.visit_string::<ValueError>("urn:example:".to_owned()).is_err());
        assert!(UrnSliceVisitor.visit_byte_buf::<ValueError>(b"urn:example:\xff".to_vec()).is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn reports_what_it_expects() {
        let err = serde_json::from_str::<Record<'_>>(r#"{"id":1}"#).err().unwrap();
        assert!(err.to_string().starts_with("invalid type: integer `1`, expected a URN string"), "{err}");
    }

    #[cfg(not(feature = "alloc"))]
    #[test]
    fn needs_alloc_for_what_it_cannot_borrow() {
        assert!(UrnSliceVisitor.visit_borrowed_str::<ValueError>(DENORMALIZED).is_err());
        assert!(UrnSliceVisitor.visit_str::<ValueError>(NORMALIZED).is_err());
        assert!(UrnSliceVisitor.visit_bytes::<ValueError>(NORMALIZED.as_bytes()).is_err());
        assert!(UrnSlice::deserialize(&mut serde_json::Deserializer::from_reader(JSON.as_bytes())).is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn urn_round_trips() {
        let urn = Urn::try_from(NORMALIZED).unwrap();
        let json = serde_json::to_string(&urn).unwrap();
        assert_eq!(json, r#""urn:example:a%2Fb?+r?=q#f""#);
        assert_eq!(serde_json::from_str::<Urn>(&json).unwrap().as_str(), NORMALIZED);
        assert_eq!(
            serde_json::from_str::<Vec<Urn>>(r#"["URN:Example:a%2fb?+r?=q#f"]"#).unwrap()[0].as_str(),
            NORMALIZED
        );
    }
}

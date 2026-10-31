//! Property tests. urn-rs is checked against a reference parser written straight from the RFC 8141
//! ABNF (`reference.rs`), and its parsing paths, comparisons, builder, setters and
//! percent-encoding are checked against each other.
//!
//! Test binaries always link `std`, even when the library is built without it, so the tests
//! themselves use `std`; only the library calls are limited by the feature set.
//!
//! Set `PROPTEST_CASES` to run more cases than the default 256 per property.

mod reference;
mod strategy;

use std::{collections::hash_map::RandomState, hash::BuildHasher, str};

use proptest::{collection::vec, prelude::*};
use urn_rs::{Error, UrnSlice};
#[cfg(feature = "alloc")]
use urn_rs::{Urn, UrnBuilder, percent};

#[cfg(feature = "alloc")]
use crate::strategy::{component, tricky};
use crate::{
    reference::{Kind, Parts},
    strategy::{Components, component_value, components, nid_value, optional_value, urn_like, urn_pair},
};

/// A parse result as seen from outside: the normalized string and components, or the error.
type Outcome = Result<Parts, Error>;

fn parts_of(urn: &UrnSlice<'_>) -> Parts {
    Parts {
        normalized: urn.as_str().to_owned(),
        nid: urn.nid().to_owned(),
        nss: urn.nss().to_owned(),
        r: urn.r_component().map(str::to_owned),
        q: urn.q_component().map(str::to_owned),
        f: urn.f_component().map(str::to_owned),
    }
}

fn outcome(result: Result<UrnSlice<'_>, Error>) -> Outcome {
    result.map(|urn| parts_of(&urn))
}

/// Parse through `&mut str`, which normalizes in place and so gives the exact answer in every
/// feature set.
fn parse_mut(s: &str) -> Outcome {
    let mut buf = s.to_owned();
    outcome(UrnSlice::try_from(buf.as_mut_str()))
}

/// Parse a URN the test expects to be valid.
fn parse_valid(buf: &mut String) -> Result<UrnSlice<'_>, TestCaseError> {
    let input = buf.clone();
    UrnSlice::try_from(buf.as_mut_str()).map_err(|e| TestCaseError::fail(format!("{input:?} should parse: {e:?}")))
}

/// Whether a borrowed parsing path (`&str`, `&[u8]`) gave the right answer. Without `alloc` those
/// paths can't copy, so they may fail with `AllocRequired` instead: for a valid URN only when it
/// needs normalizing, for an invalid one also before reaching the real error.
fn borrowed_ok(got: &Outcome, want: &Outcome, input: &str) -> bool {
    got == want || (cfg!(not(feature = "alloc")) && *got == Err(Error::AllocRequired) && !want.as_ref().is_ok_and(|p| p.normalized == input))
}

proptest! {
    /// The verdict, the component split and the normalized form all match the reference parser,
    /// and the normalized form is a fixed point.
    #[test]
    fn parse_agrees_with_reference(s in urn_like()) {
        let want = reference::parse(&s);
        prop_assert_eq!(&parse_mut(&s), &want, "{:?}", s);
        if let Ok(parts) = want {
            // Already normalized, so borrowing needs no copy and normalizing again changes nothing.
            prop_assert_eq!(outcome(UrnSlice::try_from(parts.normalized.as_str())), Ok(parts.clone()));
            prop_assert_eq!(parse_mut(&parts.normalized), Ok(parts));
        }
    }

    /// Every way to parse a string gives the same answer, whether the URN ends up borrowed,
    /// mutably borrowed or owned.
    #[test]
    fn entry_points_agree(s in urn_like()) {
        let want = parse_mut(&s);
        let mut bytes = s.clone().into_bytes();
        prop_assert_eq!(&outcome(UrnSlice::try_from(bytes.as_mut_slice())), &want, "&mut [u8] {:?}", s);
        for (path, got) in [("&str", outcome(UrnSlice::try_from(s.as_str()))), ("&[u8]", outcome(UrnSlice::try_from(s.as_bytes())))] {
            prop_assert!(borrowed_ok(&got, &want, &s), "{path} {s:?}: {got:?}, want {want:?}");
        }

        #[cfg(feature = "alloc")]
        {
            let owned = [
                ("FromStr UrnSlice", outcome(s.parse::<UrnSlice>())),
                ("String", outcome(UrnSlice::try_from(s.clone()))),
                ("Vec<u8>", outcome(UrnSlice::try_from(s.clone().into_bytes()))),
            ];
            for (path, got) in owned {
                prop_assert_eq!(&got, &want, "{} {:?}", path, s);
            }
            let mut buf = s.clone();
            let urns = [
                ("FromStr Urn", s.parse::<Urn>()),
                ("Urn &str", Urn::try_from(s.as_str())),
                ("Urn &mut str", Urn::try_from(buf.as_mut_str())),
                ("Urn String", Urn::try_from(s.clone())),
                ("Urn &[u8]", Urn::try_from(s.as_bytes())),
                ("Urn Vec<u8>", Urn::try_from(s.clone().into_bytes())),
            ];
            for (path, got) in urns {
                let normalized = want.as_ref().map(|p| p.normalized.clone()).map_err(|e| *e);
                prop_assert_eq!(&got.as_ref().map(ToString::to_string).map_err(|e| *e), &normalized, "{} Display {:?}", path, s);
                prop_assert_eq!(&got.clone().map(Urn::into_string), &normalized, "{} into_string {:?}", path, s);
                prop_assert_eq!(&got.map(|u| parts_of(&u)), &want, "{} {:?}", path, s);
            }
        }
    }

    /// The byte paths reject invalid UTF-8 and otherwise agree with the string paths.
    #[test]
    fn byte_paths_check_utf8(prefix in prop_oneof![Just(""), Just("urn:"), Just("urn:ab:")], tail in vec(any::<u8>(), 0..24)) {
        let input = [prefix.as_bytes(), &tail].concat();
        let want = str::from_utf8(&input).map_or(Err(Error::InvalidUtf8), parse_mut);
        let mut buf = input.clone();
        prop_assert_eq!(&outcome(UrnSlice::try_from(buf.as_mut_slice())), &want);
        let got = outcome(UrnSlice::try_from(input.as_slice()));
        prop_assert!(got == want || (cfg!(not(feature = "alloc")) && got == Err(Error::AllocRequired)), "&[u8]: {got:?}, want {want:?}");
        #[cfg(feature = "alloc")]
        {
            prop_assert_eq!(&outcome(UrnSlice::try_from(input.clone())), &want);
            prop_assert_eq!(&Urn::try_from(input.as_slice()).map(|u| parts_of(&u)), &want);
            prop_assert_eq!(Urn::try_from(input).map(|u| parts_of(&u)), want);
        }
    }

    /// `PartialEq`, `Ord`, `PartialOrd` and `Hash` agree with each other and with the reference
    /// comparison key, with or without `exact-eq`.
    #[test]
    fn comparisons_agree_with_reference((a, b) in urn_pair()) {
        let (Ok(pa), Ok(pb)) = (reference::parse(&a), reference::parse(&b)) else {
            return Err(TestCaseError::reject("not both valid"));
        };
        let (mut buf_a, mut buf_b) = (a, b);
        let (ua, ub) = (parse_valid(&mut buf_a)?, parse_valid(&mut buf_b)?);
        let (key_a, key_b) = (pa.eq_key(), pb.eq_key());
        let equal = key_a == key_b;
        let order = key_a.cmp(&key_b);

        prop_assert_eq!(ua == ub, equal);
        prop_assert_eq!(ub == ua, equal);
        prop_assert_eq!(ua.cmp(&ub), order);
        prop_assert_eq!(ub.cmp(&ua), order.reverse());
        prop_assert_eq!(ua.partial_cmp(&ub), Some(order));
        // Unequal URNs collide only by chance, which the random hasher keys make negligible.
        let state = RandomState::new();
        prop_assert_eq!(state.hash_one(&ua) == state.hash_one(&ub), equal);

        #[cfg(feature = "alloc")]
        {
            let (oa, ob) = (Urn::from(&ua), Urn::from(&ub));
            prop_assert_eq!(oa == ob, equal);
            prop_assert_eq!(oa == ub, equal);
            prop_assert_eq!(ua == ob, equal);
            prop_assert_eq!(oa.cmp(&ob), order);
            prop_assert_eq!(oa.partial_cmp(&ob), Some(order));
            // `Urn: Borrow<UrnSlice>`, so both must hash the same.
            prop_assert_eq!(state.hash_one(&oa), state.hash_one(&ua));
        }
    }

    /// Each setter either rejects the value and leaves the URN alone, or gives the URN that
    /// parsing the edited string gives.
    #[test]
    fn setters_agree_with_building_from_scratch(base in components(), set in setter()) {
        let input = base.render();
        let Ok(before) = reference::parse(&input) else {
            return Err(TestCaseError::reject("base not valid"));
        };
        // The generated pieces can split differently once joined (an r-component containing `?=`
        // parses as r and q), so edit the pieces the URN actually parsed to.
        let parsed = Components {
            scheme: "urn".to_owned(),
            nid: before.nid.clone(),
            nss: before.nss.clone(),
            r: before.r.clone(),
            q: before.q.clone(),
            f: before.f.clone(),
        };
        let want = set.expected(&parsed);

        #[cfg(feature = "alloc")]
        {
            let mut urn = UrnSlice::try_from(input.as_str()).map_err(|e| TestCaseError::fail(format!("{e:?}")))?;
            check_set(set.apply(&mut urn), &urn, &want, &before)?;
            let mut urn = UrnSlice::try_from(input.clone()).map_err(|e| TestCaseError::fail(format!("{e:?}")))?;
            check_set(set.apply(&mut urn), &urn, &want, &before)?;
            let mut urn = Urn::try_from(input.as_str()).map_err(|e| TestCaseError::fail(format!("{e:?}")))?;
            check_set(set.apply(&mut urn), &urn, &want, &before)?;
        }

        // `&mut str`: written in place when the length doesn't change.
        let mut buf = input;
        let mut urn = parse_valid(&mut buf)?;
        check_set(set.apply(&mut urn), &urn, &want, &before)?;
    }
}

/// One setter call.
#[derive(Clone, Debug)]
enum Set {
    Nid(String),
    Nss(String),
    R(Option<String>),
    Q(Option<String>),
    F(Option<String>),
}

fn setter() -> impl Strategy<Value = Set> {
    prop_oneof![
        nid_value().prop_map(Set::Nid),
        component_value(Kind::Nss).prop_map(Set::Nss),
        optional_value(Kind::R).prop_map(Set::R),
        optional_value(Kind::Q).prop_map(Set::Q),
        optional_value(Kind::F).prop_map(Set::F),
    ]
}

impl Set {
    fn apply(&self, urn: &mut UrnSlice<'_>) -> Result<(), Error> {
        match self {
            Self::Nid(v) => urn.set_nid(v),
            Self::Nss(v) => urn.set_nss(v),
            Self::R(v) => urn.set_r_component(v.as_deref()),
            Self::Q(v) => urn.set_q_component(v.as_deref()),
            Self::F(v) => urn.set_f_component(v.as_deref()),
        }
    }

    /// What `base` becomes after this call: rejected if the value isn't valid on its own,
    /// otherwise whatever the edited string parses to.
    fn expected(&self, base: &Components) -> Outcome {
        let valid = |value: &Option<String>, kind| value.as_deref().is_none_or(|v| reference::is_component(v, kind));
        let edited = match self {
            Self::Nid(v) if !reference::is_nid(v) => return Err(Error::InvalidNid),
            Self::Nss(v) if !reference::is_component(v, Kind::Nss) => return Err(Error::InvalidNss),
            Self::R(v) if !valid(v, Kind::R) => return Err(Error::InvalidRComponent),
            Self::Q(v) if !valid(v, Kind::Q) => return Err(Error::InvalidQComponent),
            Self::F(v) if !valid(v, Kind::F) => return Err(Error::InvalidFComponent),
            Self::Nid(v) => Components {
                nid: v.clone(),
                ..base.clone()
            },
            Self::Nss(v) => Components {
                nss: v.clone(),
                ..base.clone()
            },
            Self::R(v) => Components { r: v.clone(), ..base.clone() },
            Self::Q(v) => Components { q: v.clone(), ..base.clone() },
            Self::F(v) => Components { f: v.clone(), ..base.clone() },
        };
        reference::parse(&edited.render())
    }
}

/// Check one setter call. A rejected call must leave the URN as it was. Without `alloc`, a call
/// that changes the URN's length can't be done in place and fails with `AllocRequired`.
fn check_set(result: Result<(), Error>, urn: &UrnSlice<'_>, want: &Outcome, before: &Parts) -> Result<(), TestCaseError> {
    let after = parts_of(urn);
    match (result, want) {
        (Ok(()), Ok(want)) => prop_assert_eq!(&after, want),
        (Err(Error::AllocRequired), Ok(want)) if cfg!(not(feature = "alloc")) && want.normalized.len() != before.normalized.len() => {
            prop_assert_eq!(&after, before);
        }
        (Err(e), Err(want)) => {
            prop_assert_eq!(e, *want);
            prop_assert_eq!(&after, before);
        }
        (got, want) => prop_assert!(false, "got {got:?} leaving {after:?}, want {want:?}"),
    }
    Ok(())
}

/// The URN accessor for a component of `kind`.
#[cfg(feature = "alloc")]
fn component_of<'a>(urn: &'a UrnSlice<'_>, kind: Kind) -> Option<&'a str> {
    match kind {
        Kind::Nss => Some(urn.nss()),
        Kind::R => urn.r_component(),
        Kind::Q => urn.q_component(),
        Kind::F => urn.f_component(),
    }
}

/// The percent-encoding functions for one component kind.
#[cfg(feature = "alloc")]
struct Codec {
    kind: Kind,
    encode: fn(&str) -> Result<String, Error>,
    decode: fn(&str) -> Result<String, Error>,
    encode_iter: fn(&str) -> percent::EncodeIter<'_>,
    decode_iter: fn(&str) -> percent::DecodeIter<'_>,
}

#[cfg(feature = "alloc")]
const CODECS: [Codec; 4] = [
    Codec {
        kind: Kind::Nss,
        encode: percent::encode_nss,
        decode: percent::decode_nss,
        encode_iter: percent::encode_nss_iter,
        decode_iter: percent::decode_nss_iter,
    },
    Codec {
        kind: Kind::R,
        encode: percent::encode_r_component,
        decode: percent::decode_r_component,
        encode_iter: percent::encode_r_component_iter,
        decode_iter: percent::decode_r_component_iter,
    },
    Codec {
        kind: Kind::Q,
        encode: percent::encode_q_component,
        decode: percent::decode_q_component,
        encode_iter: percent::encode_q_component_iter,
        decode_iter: percent::decode_q_component_iter,
    },
    Codec {
        kind: Kind::F,
        encode: percent::encode_f_component,
        decode: percent::decode_f_component,
        encode_iter: percent::encode_f_component_iter,
        decode_iter: percent::decode_f_component_iter,
    },
];

/// A builder for `urn:ex:nss` with `value` in the `kind` slot.
#[cfg(feature = "alloc")]
const fn builder_with(kind: Kind, value: &str) -> UrnBuilder<'_> {
    let builder = UrnBuilder::new("ex", "nss");
    match kind {
        Kind::Nss => builder.nss(value),
        Kind::R => builder.r_component(Some(value)),
        Kind::Q => builder.q_component(Some(value)),
        Kind::F => builder.f_component(Some(value)),
    }
}

/// What the builder gives for these pieces: the first invalid piece's error, otherwise whatever
/// the assembled string parses to.
#[cfg(feature = "alloc")]
fn expected_build(nid: &str, nss: &str, r: Option<&str>, q: Option<&str>, f: Option<&str>) -> Outcome {
    if !reference::is_nid(nid) {
        return Err(Error::InvalidNid);
    }
    for (value, kind) in [(Some(nss), Kind::Nss), (r, Kind::R), (q, Kind::Q), (f, Kind::F)] {
        if value.is_some_and(|v| !reference::is_component(v, kind)) {
            return Err(kind.error());
        }
    }
    let owned = |v: Option<&str>| v.map(str::to_owned);
    let pieces = Components {
        scheme: "urn".to_owned(),
        nid: nid.to_owned(),
        nss: nss.to_owned(),
        r: owned(r),
        q: owned(q),
        f: owned(f),
    };
    reference::parse(&pieces.render())
}

#[cfg(feature = "alloc")]
proptest! {
    /// Encoding any string gives a valid component that decodes back to the string. The
    /// iterators agree with the allocating functions.
    #[test]
    fn encode_then_decode_round_trips(s in prop_oneof![any::<String>(), tricky(), component(Kind::R)]) {
        for codec in &CODECS {
            let encoded = (codec.encode)(&s);
            if s.is_empty() && codec.kind != Kind::F {
                prop_assert_eq!(encoded, Err(codec.kind.error()));
                continue;
            }
            let encoded = encoded.map_err(|e| TestCaseError::fail(format!("{:?} encode {s:?}: {e:?}", codec.kind)))?;
            prop_assert!(reference::is_component(&encoded, codec.kind), "{:?} encode {s:?} gave {encoded:?}", codec.kind);
            prop_assert_eq!((codec.decode)(&encoded), Ok(s.clone()));
            prop_assert_eq!(String::from_utf8((codec.encode_iter)(&s).collect()).ok(), Some(encoded.clone()));
            // Encoded hex digits are uppercase, so the builder keeps the component as it is.
            let built = builder_with(codec.kind, &encoded).build().map_err(|e| TestCaseError::fail(format!("{encoded:?}: {e:?}")))?;
            prop_assert_eq!(component_of(&built, codec.kind), Some(encoded.as_str()));
        }
    }

    /// Decoding accepts exactly the valid components and gives what the reference decoder gives.
    #[test]
    fn decode_agrees_with_reference(s in prop_oneof![component(Kind::Nss), component(Kind::R), component(Kind::F), tricky(), Just(String::new())]) {
        for codec in &CODECS {
            let want = reference::decode(&s, codec.kind).ok_or(codec.kind.error());
            prop_assert_eq!((codec.decode)(&s), want, "{:?} {:?}", codec.kind, s);
            let want_bytes = reference::decode_bytes(&s, codec.kind).ok_or(codec.kind.error());
            prop_assert_eq!((codec.decode_iter)(&s).collect::<Result<Vec<u8>, Error>>(), want_bytes, "{:?} iter {:?}", codec.kind, s);
        }
    }

    /// The builder accepts exactly the valid pieces and gives what parsing the assembled string
    /// gives.
    #[test]
    fn builder_agrees_with_parsing(
        nid in nid_value(),
        nss in component_value(Kind::Nss),
        r in optional_value(Kind::R),
        q in optional_value(Kind::Q),
        f in optional_value(Kind::F),
    ) {
        let want = expected_build(&nid, &nss, r.as_deref(), q.as_deref(), f.as_deref());
        let builder = || UrnBuilder::new(&nid, &nss).r_component(r.as_deref()).q_component(q.as_deref()).f_component(f.as_deref());
        prop_assert_eq!(&builder().build().map(|u| parts_of(&u)), &want);
        prop_assert_eq!(outcome(builder().build_slice()), want);
    }
}

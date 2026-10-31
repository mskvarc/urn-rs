//! Checks behind the fuzz targets. Each target hands its input to one function here, which panics
//! when urn-rs misbehaves. The oracle is the reference parser the property tests use
//! (`tests/props/reference.rs`), written straight from the RFC 8141 ABNF.
//!
//! The fuzz crate always links `std`; only the urn-rs calls are limited by the feature set, which
//! mirrors the urn-rs features (`--no-default-features` fuzzes the `no_std` build).

#[path = "../../crates/urn-rs/tests/props/reference.rs"]
mod reference;

use std::{cmp::Ordering, collections::hash_map::RandomState, hash::BuildHasher, str};

use arbitrary::Arbitrary;
use urn_rs::{Error, UrnSlice};
#[cfg(feature = "alloc")]
use urn_rs::{Urn, percent};

use crate::reference::{Kind, Parts};

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

/// Parse through `&mut [u8]`, which normalizes in place and so gives the exact answer in every
/// feature set.
fn parse_mut(data: &[u8]) -> Outcome {
    let mut buf = data.to_vec();
    outcome(UrnSlice::try_from(buf.as_mut_slice()))
}

/// Whether a borrowed parsing path (`&str`, `&[u8]`) gave the right answer. Without `alloc` those
/// paths can't copy, so they may fail with `AllocRequired` instead: for a valid URN only when it
/// needs normalizing, for an invalid one also before reaching the real error.
fn borrowed_ok(got: &Outcome, want: &Outcome, input: &[u8]) -> bool {
    got == want || (cfg!(not(feature = "alloc")) && *got == Err(Error::AllocRequired) && !want.as_ref().is_ok_and(|p| p.normalized.as_bytes() == input))
}

/// The URN string made of these pieces.
fn render(nid: &str, nss: &str, r: Option<&str>, q: Option<&str>, f: Option<&str>) -> String {
    let mut s = format!("urn:{nid}:{nss}");
    for (prefix, value) in [("?+", r), ("?=", q), ("#", f)] {
        if let Some(value) = value {
            s.push_str(prefix);
            s.push_str(value);
        }
    }
    s
}

/// Parsing never panics, every entry point gives the same answer, and a valid URN round-trips:
/// its normalized form parses back to the same URN without being changed again.
///
/// # Panics
/// When urn-rs misbehaves.
pub fn parse(data: &[u8]) {
    let want = parse_mut(data);
    let text = str::from_utf8(data);
    if text.is_err() {
        assert_eq!(want, Err(Error::InvalidUtf8), "{data:?}");
    }

    let got = outcome(UrnSlice::try_from(data));
    assert!(borrowed_ok(&got, &want, data), "&[u8] {data:?}: {got:?}, want {want:?}");
    if let Ok(s) = text {
        let mut buf = s.to_owned();
        assert_eq!(outcome(UrnSlice::try_from(buf.as_mut_str())), want, "&mut str {s:?}");
        let got = outcome(UrnSlice::try_from(s));
        assert!(borrowed_ok(&got, &want, data), "&str {s:?}: {got:?}, want {want:?}");
    }

    #[cfg(feature = "alloc")]
    {
        assert_eq!(outcome(UrnSlice::try_from(data.to_vec())), want, "Vec<u8> {data:?}");
        let normalized = want.as_ref().map(|p| p.normalized.clone()).map_err(|e| *e);
        assert_eq!(Urn::try_from(data).map(Urn::into_string), normalized, "Urn &[u8] {data:?}");
        if let Ok(s) = text {
            assert_eq!(outcome(UrnSlice::try_from(s.to_owned())), want, "String {s:?}");
            assert_eq!(outcome(s.parse::<UrnSlice>()), want, "FromStr UrnSlice {s:?}");
            assert_eq!(s.parse::<Urn>().map(|u| u.to_string()), normalized, "FromStr Urn {s:?}");
        }
    }

    let Ok(parts) = want else { return };
    assert_eq!(parts.normalized.len(), data.len(), "normalizing changed the length of {data:?}");
    let pieces = render(&parts.nid, &parts.nss, parts.r.as_deref(), parts.q.as_deref(), parts.f.as_deref());
    assert_eq!(parts.normalized, pieces, "accessors don't add up to as_str()");
    let urn = UrnSlice::try_from(parts.normalized.as_str()).unwrap_or_else(|e| panic!("{:?} doesn't parse back: {e:?}", parts.normalized));
    assert_eq!(parts_of(&urn), parts, "normalizing again changed {:?}", parts.normalized);
    assert_eq!(parse_mut(parts.normalized.as_bytes()), Ok(parts.clone()));
    assert_eq!(urn.to_string(), parts.normalized);

    let mut buf = data.to_vec();
    let Ok(in_place) = UrnSlice::try_from(buf.as_mut_slice()) else { return };
    assert_eq!(urn, in_place);
    assert_eq!(urn.cmp(&in_place), Ordering::Equal);
    let state = RandomState::new();
    assert_eq!(state.hash_one(&urn), state.hash_one(&in_place));
    #[cfg(feature = "alloc")]
    {
        let owned = in_place.clone().into_owned();
        assert_eq!(owned, urn);
        assert_eq!(state.hash_one(&owned), state.hash_one(&urn));
        assert_eq!(parts_of(&owned), parts);
    }
}

/// Parsing agrees with the reference parser on the verdict, the error, the component split and
/// the normalized form. The input is up to two URNs separated by the first `\n`; when both are
/// valid, equality, ordering and hashing agree with the reference comparison key.
///
/// # Panics
/// When urn-rs misbehaves.
pub fn differential(data: &[u8]) {
    let (a, b) = match data.iter().position(|&c| c == b'\n') {
        Some(i) => (&data[..i], Some(&data[i + 1..])),
        None => (data, None),
    };
    let check = |input: &[u8]| {
        let want = str::from_utf8(input).map_or(Err(Error::InvalidUtf8), reference::parse);
        assert_eq!(parse_mut(input), want, "{:?}", String::from_utf8_lossy(input));
        want
    };
    let pa = check(a);
    let Some(b) = b else { return };
    let (Ok(pa), Ok(pb)) = (pa, check(b)) else { return };

    let (mut buf_a, mut buf_b) = (a.to_vec(), b.to_vec());
    let (Ok(ua), Ok(ub)) = (UrnSlice::try_from(buf_a.as_mut_slice()), UrnSlice::try_from(buf_b.as_mut_slice())) else {
        unreachable!("checked against the reference parser above");
    };
    let (key_a, key_b) = (pa.eq_key(), pb.eq_key());
    let equal = key_a == key_b;
    let order = key_a.cmp(&key_b);
    let context = format!("{:?} vs {:?}", pa.normalized, pb.normalized);
    assert_eq!(ua == ub, equal, "{context}");
    assert_eq!(ub == ua, equal, "{context}");
    assert_eq!(ua.cmp(&ub), order, "{context}");
    assert_eq!(ub.cmp(&ua), order.reverse(), "{context}");
    assert_eq!(ua.partial_cmp(&ub), Some(order), "{context}");
    let state = RandomState::new();
    if equal {
        assert_eq!(state.hash_one(&ua), state.hash_one(&ub), "{context}");
    }

    #[cfg(feature = "alloc")]
    {
        let (oa, ob) = (Urn::from(&ua), Urn::from(&ub));
        assert_eq!(oa == ob, equal, "{context}");
        assert_eq!(oa == ub, equal, "{context}");
        assert_eq!(ua == ob, equal, "{context}");
        assert_eq!(oa.cmp(&ob), order, "{context}");
        assert_eq!(oa.partial_cmp(&ob), Some(order), "{context}");
        // `Urn: Borrow<UrnSlice>`, so both must hash the same.
        assert_eq!(state.hash_one(&oa), state.hash_one(&ua), "{context}");
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

/// For every component kind: decoding gives what the reference decoder gives, and encoding gives
/// a valid component that decodes back to the input.
///
/// # Panics
/// When urn-rs misbehaves.
#[cfg(feature = "alloc")]
pub fn percent(data: &[u8]) {
    let Ok(s) = str::from_utf8(data) else { return };
    for codec in &CODECS {
        let kind = codec.kind;
        assert_eq!((codec.decode)(s), reference::decode(s, kind).ok_or(kind.error()), "{kind:?} decode {s:?}");
        let want_bytes = reference::decode_bytes(s, kind).ok_or(kind.error());
        assert_eq!(
            (codec.decode_iter)(s).collect::<Result<Vec<u8>, Error>>(),
            want_bytes,
            "{kind:?} decode_iter {s:?}"
        );

        let encoded = (codec.encode)(s);
        if s.is_empty() && kind != Kind::F {
            assert_eq!(encoded, Err(kind.error()), "{kind:?} encode \"\"");
            continue;
        }
        let encoded = encoded.unwrap_or_else(|e| panic!("{kind:?} encode {s:?}: {e:?}"));
        assert!(reference::is_component(&encoded, kind), "{kind:?} encode {s:?} gave {encoded:?}");
        assert_eq!((codec.decode)(&encoded).as_deref(), Ok(s), "{kind:?} round trip {s:?}");
        assert_eq!((codec.encode_iter)(s).collect::<Vec<u8>>(), encoded.as_bytes(), "{kind:?} encode_iter {s:?}");
    }
}

/// One setter call.
#[derive(Arbitrary, Clone, Debug)]
pub enum Call {
    /// `set_nid`.
    Nid(String),
    /// `set_nss`.
    Nss(String),
    /// `set_r_component`.
    R(Option<String>),
    /// `set_q_component`.
    Q(Option<String>),
    /// `set_f_component`.
    F(Option<String>),
}

impl Call {
    fn apply(&self, urn: &mut UrnSlice<'_>) -> Result<(), Error> {
        match self {
            Self::Nid(v) => urn.set_nid(v),
            Self::Nss(v) => urn.set_nss(v),
            Self::R(v) => urn.set_r_component(v.as_deref()),
            Self::Q(v) => urn.set_q_component(v.as_deref()),
            Self::F(v) => urn.set_f_component(v.as_deref()),
        }
    }

    /// What `model` becomes after this call: rejected if the value isn't valid on its own,
    /// otherwise whatever the edited string parses to.
    fn expected(&self, model: &Parts) -> Outcome {
        let valid = |value: &Option<String>, kind| value.as_deref().is_none_or(|v| reference::is_component(v, kind));
        let (nid, nss, r, q, f) = (
            model.nid.as_str(),
            model.nss.as_str(),
            model.r.as_deref(),
            model.q.as_deref(),
            model.f.as_deref(),
        );
        let edited = match self {
            Self::Nid(v) if !reference::is_nid(v) => return Err(Error::InvalidNid),
            Self::Nss(v) if !reference::is_component(v, Kind::Nss) => return Err(Error::InvalidNss),
            Self::R(v) if !valid(v, Kind::R) => return Err(Error::InvalidRComponent),
            Self::Q(v) if !valid(v, Kind::Q) => return Err(Error::InvalidQComponent),
            Self::F(v) if !valid(v, Kind::F) => return Err(Error::InvalidFComponent),
            Self::Nid(v) => render(v, nss, r, q, f),
            Self::Nss(v) => render(nid, v, r, q, f),
            Self::R(v) => render(nid, nss, v.as_deref(), q, f),
            Self::Q(v) => render(nid, nss, r, v.as_deref(), f),
            Self::F(v) => render(nid, nss, r, q, v.as_deref()),
        };
        reference::parse(&edited)
    }
}

/// A URN to start from and the setter calls to make on it.
#[derive(Arbitrary, Debug)]
pub struct SetterInput {
    /// The starting URN. An invalid one is replaced with a fixed valid URN, so every input
    /// exercises the setters.
    pub base: String,
    /// The calls, made in order.
    pub calls: Vec<Call>,
}

/// Make `calls` on `urn` in order. Each call either rejects the value and leaves the URN alone,
/// or gives the URN that parsing the edited string gives. Without `alloc`, a call that changes
/// the URN's length can't be done in place and fails with `AllocRequired`.
fn run_calls(path: &str, urn: &mut UrnSlice<'_>, calls: &[Call], start: &Parts) {
    let mut model = start.clone();
    for call in calls {
        let want = call.expected(&model);
        let result = call.apply(urn);
        let after = parts_of(urn);
        match (result, &want) {
            (Ok(()), Ok(want)) => assert_eq!(&after, want, "{path} {call:?} on {:?}", model.normalized),
            (Err(Error::AllocRequired), Ok(want)) if cfg!(not(feature = "alloc")) && want.normalized.len() != model.normalized.len() => {
                assert_eq!(after, model, "{path} {call:?} failed but changed the URN");
            }
            (Err(e), Err(want)) => {
                assert_eq!(e, *want, "{path} {call:?} on {:?}", model.normalized);
                assert_eq!(after, model, "{path} {call:?} failed but changed the URN");
            }
            (got, want) => panic!("{path} {call:?} on {:?}: got {got:?} leaving {after:?}, want {want:?}", model.normalized),
        }
        model = after;
    }
}

/// Random sequences of setter calls, made on a borrowed, a mutably borrowed and an owned URN.
///
/// # Panics
/// When urn-rs misbehaves.
pub fn setters(input: &SetterInput) {
    let base = match reference::parse(&input.base) {
        Ok(_) => input.base.as_str(),
        Err(_) => "urn:example:nss",
    };
    let Ok(start) = reference::parse(base) else {
        unreachable!("the fallback base is valid");
    };

    let mut buf = base.to_owned();
    let mut urn = UrnSlice::try_from(buf.as_mut_str()).unwrap_or_else(|e| panic!("{base:?}: {e:?}"));
    run_calls("&mut str", &mut urn, &input.calls, &start);

    #[cfg(feature = "alloc")]
    {
        let mut urn = UrnSlice::try_from(base).unwrap_or_else(|e| panic!("{base:?}: {e:?}"));
        run_calls("&str", &mut urn, &input.calls, &start);
        let mut urn = UrnSlice::try_from(base.to_owned()).unwrap_or_else(|e| panic!("{base:?}: {e:?}"));
        run_calls("String", &mut urn, &input.calls, &start);
        let mut urn = Urn::try_from(base).unwrap_or_else(|e| panic!("{base:?}: {e:?}"));
        run_calls("Urn", &mut urn, &input.calls, &start);
    }
}

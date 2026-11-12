//! Conformance corpus for RFC 8141 URN syntax, normalization and equivalence.
//!
//! The cases in `valid.rs`, `invalid.rs` and `equivalence.rs` come from RFC 8141, RFC 2141 and
//! the test suites of other URN libraries. Each case cites its source. Where a library or RFC 2141
//! disagrees with RFC 8141, the case follows RFC 8141 and says so in its source note.
//!
//! Every case is checked through every public entry point available in the current feature set,
//! so the borrowed, mutably borrowed and owned parsing paths are all held to the same answers.
//!
//! Test binaries always link `std`, even when the library is built without it, so the harness
//! itself uses `std`; only the library calls are limited by the feature set.
#![expect(clippy::panic, clippy::expect_used, reason = "the harness reports a failing case by panicking")]

mod case;
mod equivalence;
mod invalid;
mod valid;

use urn_rs::{Error, UrnSlice};
#[cfg(feature = "alloc")]
use urn_rs::{Urn, UrnBuilder};

use std::{
    panic::{self, AssertUnwindSafe},
    str,
};

use crate::case::{Equiv, Invalid, Valid};

/// Run `check` on every case and fail once at the end, listing every case that failed.
fn run_all<T>(kind: &str, cases: &[T], check: impl Fn(&T)) {
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|case| panic::catch_unwind(AssertUnwindSafe(|| check(case))).err())
        .map(|payload| {
            payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_default()
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} {kind} cases failed:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

/// Largest input in the corpus, in bytes. Inputs are copied into stack buffers of this size so the
/// `&mut str` and `&mut [u8]` paths can be tested without `alloc`.
const MAX_INPUT: usize = 128;

/// Copy `input` into `buf` and return the mutable view of the copy.
fn mut_copy<'b>(buf: &'b mut [u8; MAX_INPUT], input: &str) -> &'b mut [u8] {
    assert!(input.len() <= MAX_INPUT, "corpus input longer than MAX_INPUT: {input:?}");
    let bytes = &mut buf[..input.len()];
    bytes.copy_from_slice(input.as_bytes());
    bytes
}

/// Copy `input` into `buf` and return the copy as `&mut str`.
fn mut_str<'b>(buf: &'b mut [u8; MAX_INPUT], input: &str) -> &'b mut str {
    str::from_utf8_mut(mut_copy(buf, input)).expect("corpus inputs are UTF-8")
}

fn assert_parts(case: &Valid, urn: &UrnSlice<'_>, path: &str) {
    let ctx = format!("{path}: {:?} ({})", case.input, case.source);
    assert_eq!(urn.as_str(), case.normalized, "normalized form, {ctx}");
    assert_eq!(urn.nid(), case.nid, "nid, {ctx}");
    assert_eq!(urn.nss(), case.nss, "nss, {ctx}");
    assert_eq!(urn.r_component(), case.r, "r-component, {ctx}");
    assert_eq!(urn.q_component(), case.q, "q-component, {ctx}");
    assert_eq!(urn.f_component(), case.f, "f-component, {ctx}");
}

#[test]
fn valid_cases_parse_to_expected_parts() {
    run_all("valid", valid::CASES, check_valid);
}

fn check_valid(case: &Valid) {
    // `&mut str` and `&mut [u8]` normalize in place and never need to allocate.
    let mut buf = [0u8; MAX_INPUT];
    let urn = UrnSlice::try_from(mut_copy(&mut buf, case.input)).unwrap_or_else(|e| panic!("&mut [u8]: {:?} ({}): {e:?}", case.input, case.source));
    assert_parts(case, &urn, "&mut [u8]");
    let urn = UrnSlice::try_from(mut_str(&mut buf, case.input)).unwrap_or_else(|e| panic!("&mut str: {:?} ({}): {e:?}", case.input, case.source));
    assert_parts(case, &urn, "&mut str");

    // `&str` and `&[u8]` borrow when the input is already normalized. Without `alloc` they
    // can't copy, so a non-normalized input is rejected with `AllocRequired`.
    let needs_copy = case.input != case.normalized;
    for (path, result) in [
        ("&str", UrnSlice::try_from(case.input)),
        ("&[u8]", UrnSlice::try_from(case.input.as_bytes())),
        ("from_static", UrnSlice::from_static(case.input)),
    ] {
        match result {
            Ok(urn) => assert_parts(case, &urn, path),
            Err(Error::AllocRequired) if needs_copy && cfg!(not(feature = "alloc")) => {}
            Err(e) => panic!("{path}: {:?} ({}): {e:?}", case.input, case.source),
        }
    }

    #[cfg(feature = "alloc")]
    check_valid_owned(case);
}

/// Owned parsing paths, the builder, `Display` and the re-parse round trip.
#[cfg(feature = "alloc")]
fn check_valid_owned(case: &Valid) {
    let ctx = format!("{:?} ({})", case.input, case.source);
    let owned = [
        ("FromStr UrnSlice", case.input.parse::<UrnSlice>()),
        ("String", UrnSlice::try_from(case.input.to_owned())),
        ("Vec<u8>", UrnSlice::try_from(case.input.as_bytes().to_vec())),
    ];
    for (path, result) in owned {
        let urn = result.unwrap_or_else(|e| panic!("{path}: {ctx}: {e:?}"));
        assert_parts(case, &urn, path);
    }

    let urns = [
        ("FromStr Urn", case.input.parse::<Urn>()),
        ("Urn &str", Urn::try_from(case.input)),
        ("Urn String", Urn::try_from(case.input.to_owned())),
        ("Urn Vec<u8>", Urn::try_from(case.input.as_bytes().to_vec())),
    ];
    for (path, result) in urns {
        let urn = result.unwrap_or_else(|e| panic!("{path}: {ctx}: {e:?}"));
        assert_parts(case, &urn, path);
        assert_eq!(urn.to_string(), case.normalized, "Display, {ctx}");
        assert_eq!(urn.clone().into_string(), case.normalized, "into_string, {ctx}");
    }

    // The normalized form is a fixed point: parsing it again changes nothing.
    let again = UrnSlice::try_from(case.normalized).unwrap_or_else(|e| panic!("re-parse of normalized form, {ctx}: {e:?}"));
    assert_parts(case, &again, "re-parse");

    // Building from the (already encoded) components gives the same URN.
    let built = UrnBuilder::new(case.nid, case.nss)
        .r_component(case.r)
        .q_component(case.q)
        .f_component(case.f)
        .build()
        .unwrap_or_else(|e| panic!("builder, {ctx}: {e:?}"));
    assert_parts(case, &built, "builder");
}

#[test]
fn invalid_cases_are_rejected_with_expected_error() {
    run_all("invalid", invalid::CASES, check_invalid);
}

fn check_invalid(case: &Invalid) {
    let ctx = format!("{:?} ({})", case.input, case.source);
    let expect = |path: &str, result: Result<UrnSlice<'_>, Error>| match result {
        Err(e) => assert_eq!(e, case.error, "{path}: {ctx}"),
        Ok(urn) => panic!("{path}: {ctx}: accepted as {:?}", urn.as_str()),
    };

    let mut buf = [0u8; MAX_INPUT];
    expect("&mut [u8]", UrnSlice::try_from(mut_copy(&mut buf, case.input)));
    expect("&mut str", UrnSlice::try_from(mut_str(&mut buf, case.input)));
    // Without `alloc`, the borrowed paths can stop early with `AllocRequired` when the prefix
    // before the error needs normalizing. Only the `&mut` paths are exact there.
    for (path, result) in [("&str", UrnSlice::try_from(case.input)), ("&[u8]", UrnSlice::try_from(case.input.as_bytes()))] {
        match result {
            Err(Error::AllocRequired) if cfg!(not(feature = "alloc")) => {}
            result => expect(path, result),
        }
    }

    #[cfg(feature = "alloc")]
    {
        expect("FromStr", case.input.parse::<UrnSlice>());
        expect("String", UrnSlice::try_from(case.input.to_owned()));
        expect("Vec<u8>", UrnSlice::try_from(case.input.as_bytes().to_vec()));
        match Urn::try_from(case.input) {
            Err(e) => assert_eq!(e, case.error, "Urn &str: {ctx}"),
            Ok(urn) => panic!("Urn &str: {ctx}: accepted as {:?}", urn.as_str()),
        }
    }
}

#[test]
fn equivalence_cases_compare_as_expected() {
    run_all("equivalence", equivalence::CASES, check_equiv);
}

fn check_equiv(case: &Equiv) {
    let ctx = format!("{:?} vs {:?} ({})", case.a, case.b, case.source);
    // `&mut str` parsing never allocates, so equivalence is checked in every feature set.
    let (mut buf_a, mut buf_b) = ([0u8; MAX_INPUT], [0u8; MAX_INPUT]);
    let a = UrnSlice::try_from(mut_str(&mut buf_a, case.a)).unwrap_or_else(|e| panic!("{ctx}: {:?}: {e:?}", case.a));
    let b = UrnSlice::try_from(mut_str(&mut buf_b, case.b)).unwrap_or_else(|e| panic!("{ctx}: {:?}: {e:?}", case.b));

    let expected = case.equal;
    assert_eq!(a == b, expected, "PartialEq, {ctx}");
    assert_eq!(b == a, expected, "PartialEq is symmetric, {ctx}");
    assert_eq!(a.cmp(&b).is_eq(), expected, "Ord agrees with Eq, {ctx}");
    assert_eq!(a.cmp(&b), b.cmp(&a).reverse(), "Ord is antisymmetric, {ctx}");
    assert_eq!(a.partial_cmp(&b), Some(a.cmp(&b)), "PartialOrd agrees with Ord, {ctx}");
    #[cfg(feature = "std")]
    if expected {
        use std::{collections::hash_map::RandomState, hash::BuildHasher};
        let state = RandomState::new();
        assert_eq!(state.hash_one(&a), state.hash_one(&b), "equal URNs hash equally, {ctx}");
    }
}

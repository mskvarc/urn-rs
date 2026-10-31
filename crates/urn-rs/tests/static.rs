//! Checks the `urn!` re-export under the `static` feature.

use urn_rs::{UrnSlice, urn};

const ISBN: UrnSlice<'static> = urn!("urn:isbn:0451450523");

#[test]
fn reexported_macro_matches_runtime_parse() -> Result<(), urn_rs::Error> {
    assert_eq!(ISBN, UrnSlice::try_from("urn:isbn:0451450523")?);
    let urn = urn!("URN:Example:A%2f?=q");
    assert_eq!(urn.as_str(), "urn:example:A%2F?=q");
    assert_eq!(urn.q_pairs().next(), Some(("q", None)));
    Ok(())
}

//! Checks that `urn!` expands to the same slice the runtime parser produces.

use urn_rs::UrnSlice;
use urn_rs_static::urn;

const SIMPLE: UrnSlice<'static> = urn!("urn:example:nss");
static FULL: UrnSlice<'static> = urn!("URN:Example:a%2fb?+r?x?=k=v&q#f/?");

#[test]
fn matches_runtime_parse() -> Result<(), urn_rs::Error> {
    let cases = [
        (SIMPLE, "urn:example:nss"),
        (urn!("urn:example:nss"), "urn:example:nss"),
        (urn!("urn:example:nss?+r"), "urn:example:nss?+r"),
        (urn!("urn:example:nss?=q"), "urn:example:nss?=q"),
        (urn!("urn:example:nss#"), "urn:example:nss#"),
        (urn!("urn:example:nss?+r?=q#f"), "urn:example:nss?+r?=q#f"),
        (urn!("URN:Example:a%2fb?+r?x?=k=v&q#f/?"), "URN:Example:a%2fb?+r?x?=k=v&q#f/?"),
    ];
    for (got, input) in cases {
        let want = UrnSlice::try_from(input)?;
        assert_eq!(got.as_str(), want.as_str(), "{input}");
        assert_eq!(got.nid(), want.nid(), "{input}");
        assert_eq!(got.nss(), want.nss(), "{input}");
        assert_eq!(got.r_component(), want.r_component(), "{input}");
        assert_eq!(got.q_component(), want.q_component(), "{input}");
        assert_eq!(got.f_component(), want.f_component(), "{input}");
    }
    Ok(())
}

#[test]
fn normalizes_at_compile_time() {
    assert_eq!(FULL.as_str(), "urn:example:a%2Fb?+r?x?=k=v&q#f/?");
    assert_eq!(FULL.nid(), "example");
    assert_eq!(FULL.nss(), "a%2Fb");
    assert_eq!(FULL.r_component(), Some("r?x"));
    assert_eq!(FULL.q_component(), Some("k=v&q"));
    assert_eq!(FULL.f_component(), Some("f/?"));
}

#[test]
fn setters_work_on_macro_output() -> Result<(), urn_rs::Error> {
    let mut urn = urn!("urn:example:nss?=q");
    urn.set_r_component(Some("r"))?;
    urn.set_nss("other")?;
    assert_eq!(urn.as_str(), "urn:example:other?+r?=q");
    Ok(())
}

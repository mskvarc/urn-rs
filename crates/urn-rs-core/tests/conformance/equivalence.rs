//! Corpus: pairs of valid URNs and how they compare.
//!
//! The library section was extracted from the test suites of other URN libraries, deduplicated
//! and reviewed against RFC 8141; edit it by hand like the rest. Each `src` names where a case
//! came from, and why when urn-rs disagrees with that source.

use crate::case::{Equiv, differ, same};

#[rustfmt::skip]
pub(crate) const CASES: &[Equiv] = &[
    // RFC 8141 and RFC 2141.
    same("urn:example:a123,z456", "URN:example:a123,z456").src("RFC 8141 §3.2: scheme case"),
    same("urn:example:a123,z456", "urn:EXAMPLE:a123,z456").src("RFC 8141 §3.2: NID case"),
    same("urn:example:a123,z456", "urn:example:a123,z456?+abc").src("RFC 8141 §3.2: r-component ignored"),
    same("urn:example:a123,z456", "urn:example:a123,z456?=xyz").src("RFC 8141 §3.2: q-component ignored"),
    same("urn:example:a123,z456", "urn:example:a123,z456#789").src("RFC 8141 §3.2: f-component ignored"),
    same("urn:example:a123,z456?+abc", "urn:example:a123,z456?=xyz").src("RFC 8141 §3.2"),
    differ("urn:example:a123,z456", "urn:example:a123,z456/foo").src("RFC 8141 §3.2: '/' and what follows count"),
    differ("urn:example:a123,z456/foo", "urn:example:a123,z456/bar").src("RFC 8141 §3.2"),
    same("urn:example:a123%2Cz456", "URN:EXAMPLE:a123%2cz456").src("RFC 8141 §3.2: hex case normalized"),
    differ("urn:example:a123%2Cz456", "urn:example:a123,z456").src("RFC 8141 §3: percent-encoding not decoded"),
    differ("urn:example:A123,z456", "urn:example:a123,z456").src("RFC 8141 §3.2: NSS case-sensitive"),
    differ("urn:example:a123,Z456", "urn:example:a123,z456").src("RFC 8141 §3.2: NSS case-sensitive"),
    same("URN:foo:a123,456", "urn:FOO:a123,456").src("RFC 2141 §5"),
    same("urn:foo:a123%2C456", "URN:FOO:a123%2c456").src("RFC 2141 §5"),
    differ("urn:foo:a123,456", "urn:foo:a123%2C456").src("RFC 2141 §5"),
    same("urn:example:a?+%2f", "urn:example:a?+%2F").src("hex case normalized in r-component too"),
    same("urn:example:a?=x", "urn:example:a?=y").src("RFC 8141 §3: q-component ignored"),
    same("urn:example:a#x", "urn:example:a#y").src("RFC 8141 §3: f-component ignored"),

    // Other URN libraries' test suites.
    differ("URN:EXAMPLE:a123%2cz456", "urn:example:a123,z456").src("urnlib URNTest.java:105"),
    same("URN:FOO:AbC123%2c456", "urn:foo:AbC123%2C456").src("go-urn lexicaleq_test.go:29, go-urn lexicaleq_test.go:30"),
    same("URN:FOO:AbC123%2c456%1f", "urn:foo:AbC123%2C456%1f").src("go-urn lexicaleq_test.go:34, go-urn lexicaleq_test.go:35"),
    same("URN:FOO:a123,456", "URN:foo:a123,456").src("go-urn examples_test.go:38, go-urn examples_test.go:39"),
    differ("URN:ISBN:0-88-73-08-66-X", "urn:isbn:088730866x").src("namespace-specific ISBN rule; urn-rs applies only RFC 8141 §3; URI urn-isbn.t:31"),
    same("URN:example:a123,z456", "urn:EXAMPLE:a123,z456").src("urnlib URNTest.java:76"),
    differ("URN:example:a123,z456", "urn:example:%D0%B0123,z456").src("urnlib URNTest.java:143"),
    differ("URN:example:a123,z456", "urn:example:A123,z456").src("urnlib URNTest.java:124"),
    differ("URN:example:a123,z456", "urn:example:a123,Z456").src("urnlib URNTest.java:124"),
    differ("URN:foo:a123,456", "urn:foo:A123,456").src("go-urn lexicaleq_test.go:59, go-urn lexicaleq_test.go:60"),
    same("URN:foo:a123,456", "urn:foo:a123,456").src("go-urn lexicaleq_test.go:39, go-urn lexicaleq_test.go:40"),
    same("uRn:eXaMpLe:%3d%3a?=aoiwnfuafo", "urn:example:%3D%3a").src("urn-upstream lib.rs:435, urn-upstream lib.rs:436"),
    same("urn:A-VALID-NID:x", "urn:a-valid-nid:x").src("urnlib NamespaceIdentifierTest.java:96"),
    differ("urn:EXAMPLE:a123,z456", "urn:example:%D0%B0123,z456").src("urnlib URNTest.java:143"),
    differ("urn:EXAMPLE:a123,z456", "urn:example:A123,z456").src("urnlib URNTest.java:124"),
    differ("urn:EXAMPLE:a123,z456", "urn:example:a123,Z456").src("urnlib URNTest.java:124"),
    same("urn:EXAMPLE:foo", "urn:example:foo").src("urn-lib Rfc8141Urn.ts:63"),
    differ("urn:FOO:a123,456", "urn:foo:A123,456").src("go-urn lexicaleq_test.go:69, go-urn lexicaleq_test.go:70"),
    same("urn:FOO:a123,456", "urn:foo:a123,456").src("go-urn lexicaleq_test.go:49, go-urn lexicaleq_test.go:50"),
    same("urn:ISBN:0451450523", "urn:isbn:0451450523").src("urn-lib Rfc2141Urn.ts:33"),
    same("urn:ciao:%2E", "urn:ciao:%2e").src("go-urn lexicaleq_test.go:54, go-urn lexicaleq_test.go:55"),
    differ("urn:example:%D0%B0123,z456", "urn:example:a123,z456").src("urnlib URNTest.java:143"),
    differ("urn:example:A123,z456", "urn:example:a123,Z456").src("go-urn lexicaleq_test.go:79, go-urn lexicaleq_test.go:80"),
    differ("urn:example:a123,z456", "urn:example:a123,z456/bar").src("urnlib URN_8141Test.java:66"),
    differ("urn:example:a123,z456", "urn:example:a123,z456/baz").src("urnlib URN_8141Test.java:66"),
    same("urn:example:a123,z456#789", "urn:example:a123,z456?+abc").src("urnlib URN_8141Test.java:48"),
    same("urn:example:a123,z456#789", "urn:example:a123,z456?=xyz").src("go-urn lexicaleq_test.go:97, go-urn lexicaleq_test.go:98 +1"),
    differ("urn:example:a123,z456/bar", "urn:example:a123,z456/baz").src("urnlib URN_8141Test.java:66"),
    differ("urn:example:a123,z456/baz", "urn:example:a123,z456/foo").src("urnlib URN_8141Test.java:66"),
    differ("urn:foo:A123,456", "urn:foo:a123,456").src("go-urn lexicaleq_test.go:64, go-urn lexicaleq_test.go:65"),
    same("urn:foo:bar", "urn:foo:bar?+foo=bar?=bar=baz#foobar").src("tobias-urn UrnTest.php:35"),
    differ("urn:foo:bar", "urn:foo:baz").src("tobias-urn UrnTest.php:33, tobias-urn UrnTest.php:35"),
    differ("urn:tests:example:attributes:123?=key=value%3Asubvalue#example.org", "urn:tests:example:attributes:234?=key=value%3Asubvalue#example.org").src("urnparse test_urn.py:49, urnparse test_urn.py:55"),
];

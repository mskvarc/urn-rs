//! Case types for the corpus tables, with `const fn` builders so each row reads as one line.

use urn_rs::Error;

/// A URN that must parse. `normalized` is the expected `as_str()`; components are as returned by
/// the accessors (still percent-encoded, with hex digits uppercased).
pub(crate) struct Valid {
    pub input: &'static str,
    pub normalized: &'static str,
    pub nid: &'static str,
    pub nss: &'static str,
    pub r: Option<&'static str>,
    pub q: Option<&'static str>,
    pub f: Option<&'static str>,
    pub source: &'static str,
}

/// Start a valid case whose input is already normalized and has no r/q/f-components.
pub(crate) const fn ok(input: &'static str, nid: &'static str, nss: &'static str) -> Valid {
    Valid {
        input,
        normalized: input,
        nid,
        nss,
        r: None,
        q: None,
        f: None,
        source: "",
    }
}

impl Valid {
    /// The expected normalized form, when it differs from the input.
    pub(crate) const fn norm(mut self, normalized: &'static str) -> Self {
        self.normalized = normalized;
        self
    }
    pub(crate) const fn r(mut self, r: &'static str) -> Self {
        self.r = Some(r);
        self
    }
    pub(crate) const fn q(mut self, q: &'static str) -> Self {
        self.q = Some(q);
        self
    }
    pub(crate) const fn f(mut self, f: &'static str) -> Self {
        self.f = Some(f);
        self
    }
    pub(crate) const fn src(mut self, source: &'static str) -> Self {
        self.source = source;
        self
    }
}

/// A string that must be rejected with `error`.
pub(crate) struct Invalid {
    pub input: &'static str,
    pub error: Error,
    pub source: &'static str,
}

pub(crate) const fn bad(input: &'static str, error: Error) -> Invalid {
    Invalid { input, error, source: "" }
}

impl Invalid {
    pub(crate) const fn src(mut self, source: &'static str) -> Self {
        self.source = source;
        self
    }
}

/// Two valid URNs and how they compare. `rfc_equal` is the default RFC 8141 §3 equivalence (scheme,
/// NID and NSS only); `exact_equal` is the comparison with the `exact-eq` feature, which also
/// covers the r/q/f-components.
pub(crate) struct Equiv {
    pub a: &'static str,
    pub b: &'static str,
    pub rfc_equal: bool,
    pub exact_equal: bool,
    pub source: &'static str,
}

/// Equal under both comparisons: the two differ only in case or percent-encoding normalization.
pub(crate) const fn same(a: &'static str, b: &'static str) -> Equiv {
    Equiv {
        a,
        b,
        rfc_equal: true,
        exact_equal: true,
        source: "",
    }
}

/// Equal under RFC 8141 §3, but different with `exact-eq`: they differ only in r/q/f-components.
pub(crate) const fn rfc_same(a: &'static str, b: &'static str) -> Equiv {
    Equiv {
        a,
        b,
        rfc_equal: true,
        exact_equal: false,
        source: "",
    }
}

/// Different under both comparisons.
pub(crate) const fn differ(a: &'static str, b: &'static str) -> Equiv {
    Equiv {
        a,
        b,
        rfc_equal: false,
        exact_equal: false,
        source: "",
    }
}

impl Equiv {
    pub(crate) const fn src(mut self, source: &'static str) -> Self {
        self.source = source;
        self
    }
}

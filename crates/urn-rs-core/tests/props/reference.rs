//! Reference URN parser, written straight from the RFC 8141 ABNF (section 2) to serve as the oracle
//! for the property tests. It favours being obviously right over being fast: no lookup tables, no
//! batching, no in-place normalization.
//!
//! ```text
//! namestring    = assigned-name [ rq-components ] [ "#" f-component ]
//! assigned-name = "urn" ":" NID ":" NSS
//! NID           = (alphanum) 0*30(ldh) (alphanum)
//! NSS           = pchar *(pchar / "/")
//! rq-components = [ "?+" r-component ] [ "?=" q-component ]
//! r-component   = pchar *( pchar / "/" / "?" )
//! q-component   = pchar *( pchar / "/" / "?" )
//! f-component   = fragment               ; *( pchar / "/" / "?" )
//! pchar         = unreserved / pct-encoded / sub-delims / ":" / "@"
//! ```
//!
//! The ABNF alone is ambiguous where an r-component contains `?=`. RFC 8141 section 2 reads `?=`
//! as the start of the q-component, so a `?` inside an r-component may not be followed by `=`.
//!
//! Errors name the component where parsing stopped: input left over after a component counts
//! against that component, and a missing NSS separator after a well-formed NID counts against the
//! NSS.

use urn_rs::Error;

/// A URN component after the NID. Decides where `/` and `?` may appear and whether it may be empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Nss,
    R,
    Q,
    F,
}

impl Kind {
    /// The error for an invalid component of this kind.
    pub(crate) const fn error(self) -> Error {
        match self {
            Self::Nss => Error::InvalidNss,
            Self::R => Error::InvalidRComponent,
            Self::Q => Error::InvalidQComponent,
            Self::F => Error::InvalidFComponent,
        }
    }
}

/// A parsed URN in normalized form: scheme and NID lowercased, percent-encoded hex digits
/// uppercased. Components are still percent-encoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Parts {
    pub normalized: String,
    pub nid: String,
    pub nss: String,
    pub r: Option<String>,
    pub q: Option<String>,
    pub f: Option<String>,
}

impl Parts {
    /// The string that `PartialEq`, `Ord` and `Hash` compare: the whole normalized URN with
    /// `exact-eq`, otherwise only the scheme, NID and NSS (RFC 8141 section 3).
    pub(crate) fn eq_key(&self) -> String {
        if cfg!(feature = "exact-eq") {
            self.normalized.clone()
        } else {
            format!("urn:{}:{}", self.nid, self.nss)
        }
    }
}

/// `unreserved / sub-delims / ":" / "@"`: every pchar except the pct-encoded triplet.
pub(crate) fn is_plain_pchar(c: char) -> bool {
    c.is_ascii_alphanumeric() || "-._~!$&'()*+,;=:@".contains(c)
}

/// Byte length of the pchar at the start of `s`, if `s` starts with one.
fn pchar_len(s: &str) -> Option<usize> {
    let mut chars = s.chars();
    match chars.next()? {
        '%' => {
            let hi = chars.next()?;
            let lo = chars.next()?;
            (hi.is_ascii_hexdigit() && lo.is_ascii_hexdigit()).then_some(3)
        }
        c if is_plain_pchar(c) => Some(1),
        _ => None,
    }
}

/// Byte length of the longest prefix of `s` that matches the `kind` rule, ignoring whether the
/// rule allows an empty match (see [`is_component`]).
fn component_len(s: &str, kind: Kind) -> usize {
    let mut len = 0;
    loop {
        let rest = &s[len..];
        let first = len == 0;
        let step = pchar_len(rest).or_else(|| match rest.chars().next()? {
            // Only the f-component may start with `/` or `?`; the others start with a pchar.
            '/' if kind == Kind::F || !first => Some(1),
            '?' if kind == Kind::F || (!first && kind == Kind::Q) => Some(1),
            '?' if !first && kind == Kind::R && !rest.starts_with("?=") => Some(1),
            _ => None,
        });
        match step {
            Some(n) => len += n,
            None => return len,
        }
    }
}

/// Whether the whole of `s` is a valid `kind` component. Only the f-component may be empty.
pub(crate) fn is_component(s: &str, kind: Kind) -> bool {
    component_len(s, kind) == s.len() && (kind == Kind::F || !s.is_empty())
}

/// `NID = (alphanum) 0*30(ldh) (alphanum)`, minus the NID `urn` that RFC 2141 section 2 reserves.
pub(crate) fn is_nid(s: &str) -> bool {
    let ldh = |c: char| c.is_ascii_alphanumeric() || c == '-';
    (2..=32).contains(&s.len()) && s.chars().all(ldh) && !s.starts_with('-') && !s.ends_with('-') && !s.eq_ignore_ascii_case("urn")
}

/// Uppercase the two hex digits after every `%`. `s` must be a valid component.
fn upper_hex(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for (i, piece) in s.split('%').enumerate() {
        if i == 0 {
            out.push_str(piece);
        } else {
            let (hex, rest) = piece.split_at(2);
            out.push('%');
            out.push_str(&hex.to_ascii_uppercase());
            out.push_str(rest);
        }
    }
    out
}

/// Parse `s` as a URN.
pub(crate) fn parse(s: &str) -> Result<Parts, Error> {
    let scheme = s.get(..4).ok_or(Error::InvalidScheme)?;
    if !scheme.eq_ignore_ascii_case("urn:") {
        return Err(Error::InvalidScheme);
    }
    let rest = &s[4..];
    let Some((nid, after)) = rest.split_once(':') else {
        return Err(if is_nid(rest) { Error::InvalidNss } else { Error::InvalidNid });
    };
    if !is_nid(nid) {
        return Err(Error::InvalidNid);
    }

    let nss_len = component_len(after, Kind::Nss);
    if nss_len == 0 {
        return Err(Error::InvalidNss);
    }
    let nss = &after[..nss_len];
    let mut at = nss_len;
    let mut last = Kind::Nss;

    // Each optional component: its prefix, then a match of its rule that may be empty only for
    // the f-component.
    let mut optional = |prefix: &str, kind: Kind| -> Result<Option<&str>, Error> {
        if !after[at..].starts_with(prefix) {
            return Ok(None);
        }
        let start = at + prefix.len();
        let len = component_len(&after[start..], kind);
        if len == 0 && kind != Kind::F {
            return Err(kind.error());
        }
        at = start + len;
        last = kind;
        Ok(Some(&after[start..at]))
    };
    let r = optional("?+", Kind::R)?;
    let q = optional("?=", Kind::Q)?;
    let f = optional("#", Kind::F)?;
    if at != after.len() {
        return Err(last.error());
    }

    let nid = nid.to_ascii_lowercase();
    Ok(Parts {
        normalized: format!("urn:{nid}:{}", upper_hex(after)),
        nid,
        nss: upper_hex(nss),
        r: r.map(upper_hex),
        q: q.map(upper_hex),
        f: f.map(upper_hex),
    })
}

#[cfg(feature = "alloc")]
/// Percent-decode `s` as a `kind` component: `None` if `s` isn't a valid `kind` component or
/// doesn't decode to UTF-8.
pub(crate) fn decode(s: &str, kind: Kind) -> Option<String> {
    String::from_utf8(decode_bytes(s, kind)?).ok()
}

#[cfg(feature = "alloc")]
/// Percent-decode `s` as a `kind` component to bytes: `None` if `s` isn't a valid `kind`
/// component.
pub(crate) fn decode_bytes(s: &str, kind: Kind) -> Option<Vec<u8>> {
    if !is_component(s, kind) {
        return None;
    }
    let mut bytes = Vec::new();
    let mut rest = s;
    while let Some(c) = rest.chars().next() {
        if c == '%' {
            bytes.push(u8::from_str_radix(&rest[1..3], 16).ok()?);
            rest = &rest[3..];
        } else {
            let mut buf = [0; 4];
            bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            rest = &rest[c.len_utf8()..];
        }
    }
    Some(bytes)
}

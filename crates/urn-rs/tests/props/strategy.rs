//! Input generators. Most inputs are built from the grammar, so they get past the scheme and NID
//! and exercise the component parsers. Some are then edited at random to land just outside the
//! grammar, and some are arbitrary strings.

use proptest::{collection::vec, prelude::*, sample::Index, strategy::Union};

use crate::reference::Kind;

/// Characters that sit at the parser's decision points, plus non-ASCII and control characters.
const TRICKY: &[char] = &[
    '?', '+', '=', '#', '%', '/', ':', '-', '~', '&', '@', ' ', 'A', 'a', 'F', 'f', 'g', 'G', '0', '9', 'é', '€', '😀', '\0', '\u{7f}',
];

/// One pchar: a plain character, or a pct-encoded triplet with hex digits in either case.
fn pchar() -> impl Strategy<Value = String> {
    prop_oneof![
        8 => "[A-Za-z0-9._~!$&'()*+,;=:@-]",
        2 => "%[0-9A-Fa-f]{2}",
    ]
}

/// One piece after the first character of a `kind` component.
fn piece(kind: Kind) -> BoxedStrategy<String> {
    let mut options = vec![(16, pchar().boxed()), (1, Just("/".to_owned()).boxed())];
    if kind != Kind::Nss {
        options.push((1, Just("?".to_owned()).boxed()));
    }
    Union::new_weighted(options).boxed()
}

/// A `kind` component that is valid except where `?` lands before `=` in an r-component. Long
/// enough to cross the 8-byte batches of the parser's fast paths.
pub(crate) fn component(kind: Kind) -> BoxedStrategy<String> {
    let rest = vec(piece(kind), 0..24).prop_map(|pieces| pieces.concat());
    if kind == Kind::F {
        rest.boxed()
    } else {
        (pchar(), rest).prop_map(|(first, rest)| first + &rest).boxed()
    }
}

/// A NID-shaped string: 2 to 32 letters, digits and `-`, not starting or ending with `-`.
pub(crate) fn nid() -> impl Strategy<Value = String> {
    "[A-Za-z0-9][A-Za-z0-9-]{0,30}[A-Za-z0-9]"
}

/// A string of characters from [`TRICKY`] and plain ASCII.
pub(crate) fn tricky() -> impl Strategy<Value = String> {
    vec(prop_oneof![prop::sample::select(TRICKY), any::<char>(), prop::char::range('a', 'z')], 0..12).prop_map(|chars| chars.into_iter().collect())
}

/// A value for a NID slot: usually valid, sometimes not.
pub(crate) fn nid_value() -> impl Strategy<Value = String> {
    prop_oneof![
        6 => nid(),
        1 => Just("urn".to_owned()),
        1 => "[A-Za-z0-9-]{0,34}",
        1 => tricky(),
    ]
}

/// Ways to land just outside a component's grammar: a cut-off or non-hex `%` triplet at the end,
/// or a leading `?` or `/` (valid only in the f-component).
const NEAR_MISSES: &[(&str, &str)] = &[("", "%"), ("", "%4"), ("", "%a"), ("", "%gA"), ("", "%Ag"), ("?", ""), ("/", "")];

/// A value for a `kind` component slot: usually valid, sometimes not.
pub(crate) fn component_value(kind: Kind) -> impl Strategy<Value = String> {
    prop_oneof![
        6 => component(kind),
        1 => (component(kind), prop::sample::select(NEAR_MISSES)).prop_map(|(s, (before, after))| format!("{before}{s}{after}")),
        1 => (component(kind), tricky(), any::<Index>()).prop_map(|(s, t, at)| insert(&s, &t, at)),
        1 => tricky(),
        1 => Just(String::new()),
    ]
}

/// A value for an optional component slot.
pub(crate) fn optional_value(kind: Kind) -> impl Strategy<Value = Option<String>> {
    prop::option::weighted(0.6, component_value(kind))
}

/// The pieces of a URN, as they appear in the input (not normalized).
#[derive(Clone, Debug)]
pub(crate) struct Components {
    pub scheme: String,
    pub nid: String,
    pub nss: String,
    pub r: Option<String>,
    pub q: Option<String>,
    pub f: Option<String>,
}

impl Components {
    /// The URN string these pieces make.
    pub(crate) fn render(&self) -> String {
        let mut s = format!("{}:{}:{}", self.scheme, self.nid, self.nss);
        for (prefix, value) in [("?+", &self.r), ("?=", &self.q), ("#", &self.f)] {
            if let Some(value) = value {
                s.push_str(prefix);
                s.push_str(value);
            }
        }
        s
    }

    /// The same URN with the case of letters in the scheme and NID, and of hex digits in
    /// pct-encoded triplets, flipped where `mask` has a bit set. Normalizes to the same string.
    pub(crate) fn recase(&self, mask: u64) -> Self {
        let flip = |i: usize, c: char| {
            if mask >> (i % 64) & 1 == 1 {
                if c.is_ascii_uppercase() {
                    c.to_ascii_lowercase()
                } else {
                    c.to_ascii_uppercase()
                }
            } else {
                c
            }
        };
        let recase_letters = |s: &str| s.chars().enumerate().map(|(i, c)| flip(i, c)).collect::<String>();
        let recase_hex = |s: &str| {
            let mut out = String::with_capacity(s.len());
            let mut hex_left = 0;
            for (i, c) in s.chars().enumerate() {
                if hex_left > 0 && c.is_ascii_hexdigit() {
                    out.push(flip(i, c));
                    hex_left -= 1;
                } else {
                    hex_left = if c == '%' { 2 } else { 0 };
                    out.push(c);
                }
            }
            out
        };
        Self {
            scheme: recase_letters(&self.scheme),
            nid: recase_letters(&self.nid),
            nss: recase_hex(&self.nss),
            r: self.r.as_deref().map(recase_hex),
            q: self.q.as_deref().map(recase_hex),
            f: self.f.as_deref().map(recase_hex),
        }
    }
}

/// URN pieces that follow the grammar (with the rare exception noted on [`component`]).
pub(crate) fn components() -> impl Strategy<Value = Components> {
    let optional = |kind| prop::option::weighted(0.4, component(kind));
    (
        "[uU][rR][nN]",
        nid(),
        component(Kind::Nss),
        optional(Kind::R),
        optional(Kind::Q),
        optional(Kind::F),
    )
        .prop_map(|(scheme, nid, nss, r, q, f)| Components { scheme, nid, nss, r, q, f })
}

/// Insert `what` into `s` at a character boundary chosen by `at`.
fn insert(s: &str, what: &str, at: Index) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    let i = at.index(chars.len() + 1);
    chars.splice(i..i, what.chars());
    chars.into_iter().collect()
}

/// One random edit: insert, replace or delete a character.
#[derive(Clone, Debug)]
pub(crate) enum Edit {
    Insert(Index, char),
    Replace(Index, char),
    Delete(Index),
}

fn edit() -> impl Strategy<Value = Edit> {
    let c = || prop_oneof![3 => prop::sample::select(TRICKY), 1 => any::<char>()];
    prop_oneof![
        (any::<Index>(), c()).prop_map(|(i, c)| Edit::Insert(i, c)),
        (any::<Index>(), c()).prop_map(|(i, c)| Edit::Replace(i, c)),
        any::<Index>().prop_map(Edit::Delete),
    ]
}

fn apply(s: &str, edits: &[Edit]) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    for edit in edits {
        match *edit {
            Edit::Insert(i, c) => {
                let i = i.index(chars.len() + 1);
                chars.insert(i, c);
            }
            Edit::Replace(i, c) if !chars.is_empty() => {
                let i = i.index(chars.len());
                chars[i] = c;
            }
            Edit::Delete(i) if !chars.is_empty() => {
                let i = i.index(chars.len());
                chars.remove(i);
            }
            Edit::Replace(..) | Edit::Delete(_) => {}
        }
    }
    chars.into_iter().collect()
}

/// A string to parse: a grammatical URN, a URN with a few random edits, or an arbitrary string.
pub(crate) fn urn_like() -> impl Strategy<Value = String> {
    prop_oneof![
        4 => components().prop_map(|c| c.render()),
        2 => (components(), any::<u64>()).prop_map(|(c, mask)| c.recase(mask).render()),
        4 => (components(), vec(edit(), 1..4)).prop_map(|(c, edits)| apply(&c.render(), &edits)),
        1 => tricky().prop_map(|t| format!("urn:{t}")),
        1 => any::<String>(),
    ]
}

/// Two URNs to compare: two spellings of the same URN, the same assigned name with other
/// r/q/f-components, or two unrelated URNs.
pub(crate) fn urn_pair() -> impl Strategy<Value = (String, String)> {
    let optional = |kind| prop::option::weighted(0.4, component(kind));
    prop_oneof![
        (components(), any::<u64>()).prop_map(|(c, mask)| (c.render(), c.recase(mask).render())),
        (components(), optional(Kind::R), optional(Kind::Q), optional(Kind::F), any::<u64>()).prop_map(|(a, r, q, f, mask)| {
            let b = Components { r, q, f, ..a.recase(mask) };
            (a.render(), b.render())
        }),
        (components(), components()).prop_map(|(a, b)| (a.render(), b.render())),
    ]
}

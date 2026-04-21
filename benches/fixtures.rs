#![allow(dead_code)]

pub const MINIMAL: &str = "urn:example:1234:5678";
pub const NORMALIZED: &str = "urn:example:foo-bar-baz";
pub const NEEDS_NORM: &str = "uRn:eXaMpLe:Foo-Bar";
pub const PERCENT_HEAVY: &str = "urn:example:%3d%3a%2f%40%21";
pub const ALL_COMPONENTS: &str = "urn:example:foo?+res:x?=a=1&b=2#frag";
pub const NBN: &str = "urn:nbn:de:bvb:19-146642";
pub const WEATHER: &str = "urn:example:weather?=op=map&lat=39.56&lon=-104.85&datetime=1969-07-21T02:56:15Z";
pub const MIXED_CASE_PCT: &str = "uRn:eXaMpLe:%3d%3a?=aoiwnfuafo";

pub const INVALID_DOUBLE_COLON: &str = "urn::";
pub const INVALID_NOT_URN: &str = "not-a-urn";
pub const INVALID_NID_SPACE: &str = "urn:bad nid:x";
pub const INVALID_BAD_NID_DASH: &str = "urn:-example:abcd";
pub const INVALID_BAD_NSS_SLASH: &str = "urn:example:/abcd";

pub const UNICODE_PLAIN: &str = "naïve café 日本";
pub const ASCII_ALNUM: &str = "abcdefgHIJKLMNopqrstuvWXYZ0123456789";
pub const RESERVED_HEAVY: &str = "foo bar/baz?qux=1&z=2#frag~tilde %percent";

pub fn long_nss(len: usize) -> String {
    let palette = b"ABCdef012-._!$&'()*+,;=:@%20xyz";
    let mut s = String::with_capacity(len);
    let mut i = 0usize;
    while s.len() < len {
        s.push(palette[i % palette.len()] as char);
        i += 1;
    }
    s
}

pub fn long_urn(nss_len: usize) -> String {
    let mut u = String::from("urn:example:");
    let palette = b"ABCdef012abcXYZ";
    let mut i = 0usize;
    while u.len() < nss_len + 12 {
        u.push(palette[i % palette.len()] as char);
        i += 1;
    }
    u
}

pub fn all_pct_triplets(len_hint: usize) -> String {
    let triplets = ["%20", "%3A", "%2F", "%40", "%21", "%7E"];
    let mut s = String::new();
    let mut i = 0usize;
    while s.len() < len_hint {
        s.push_str(triplets[i % triplets.len()]);
        i += 1;
    }
    s
}

/// r-component payload with many `?` — exercises lookahead path in encode().
pub fn r_component_many_q(len: usize) -> String {
    let palette = b"abcd?efg?hij?klmn?";
    let mut s = String::with_capacity(len);
    let mut i = 0usize;
    while s.len() < len {
        s.push(palette[i % palette.len()] as char);
        i += 1;
    }
    s
}

pub fn mixed_pct(len_hint: usize) -> String {
    let mut s = String::new();
    let mut i = 0usize;
    while s.len() < len_hint {
        if i % 5 == 0 {
            s.push_str("%2a");
        } else {
            s.push((b'a' + (i as u8 % 26)) as char);
        }
        i += 1;
    }
    s
}

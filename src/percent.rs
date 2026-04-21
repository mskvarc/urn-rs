//! This module contains functions for percent-encoding and decoding various components of a URN.
#![cfg_attr(not(feature = "alloc"), allow(clippy::redundant_pub_crate))]

#[cfg(feature = "alloc")]
use crate::Error;
#[cfg(feature = "alloc")]
use crate::tables::HEX_VAL;
use crate::{
    Result,
    TriCow,
    tables::{BYTE_CLASS, HEX, PLAIN_ENC_NSS, PLAIN_ENC_RQF, PLAIN_PARSE},
};
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{string::String, vec::Vec};

/// Different components are percent-encoded differently...
#[derive(Copy, Clone)]
enum PctEncoded {
    Nss,
    RComponent,
    QComponent,
    FComponent,
}

/// Scan a contiguous run of plain-pchar bytes starting at `i`, advancing 8 at a
/// time via parallel table lookups. Returns the first index that is either out
/// of bounds or a non-plain byte. The bulk path keeps 8 independent loads in
/// flight so modern cores can saturate their load ports even though we don't
/// use explicit SIMD intrinsics.
#[inline]
fn scan_plain_run(bytes: &[u8], mut i: usize) -> usize {
    while i + 8 <= bytes.len() {
        // `try_into` on an 8-byte slice lowers to a single unaligned load.
        let c: [u8; 8] = bytes[i..i + 8].try_into().unwrap();
        let mut mask: u32 = 0;
        // Unrolled so each table lookup is independent.
        for k in 0..8 {
            if BYTE_CLASS[c[k] as usize] & PLAIN_PARSE == 0 {
                mask |= 1 << k;
            }
        }
        if mask == 0 {
            i += 8;
        } else {
            return i + mask.trailing_zeros() as usize;
        }
    }
    while i < bytes.len() && BYTE_CLASS[bytes[i] as usize] & PLAIN_PARSE != 0 {
        i += 1;
    }
    i
}

/// Parse and normalize percent-encoded string. Returns the end.
fn parse(s: &mut TriCow, start: usize, kind: PctEncoded) -> Result<usize> {
    let mut bytes = s.as_bytes();
    let mut i = start;
    while i < bytes.len() {
        // Bulk-skip plain pchar runs; drops to scalar for specials (%, ?, /) and
        // for bytes outside the pchar set.
        i = scan_plain_run(bytes, i);
        if i >= bytes.len() {
            break;
        }
        let ch = bytes[i];
        match ch {
            b'?' => match kind {
                PctEncoded::FComponent => {}
                PctEncoded::QComponent if i != start => {}
                PctEncoded::RComponent if i != start && bytes.get(i + 1) != Some(&b'=') => {}
                _ => return Ok(i),
            },
            b'/' => match kind {
                PctEncoded::FComponent => {}
                _ if i != start => {}
                _ => return Ok(i),
            },
            b'%' => {
                if i + 2 < bytes.len() && BYTE_CLASS[bytes[i + 1] as usize] & HEX != 0 && BYTE_CLASS[bytes[i + 2] as usize] & HEX != 0 {
                    // percent encoding must be normalized by uppercasing it
                    s.make_uppercase(i + 1..i + 3)?;
                    // Re-bind: make_uppercase may have promoted Borrowed -> Owned,
                    // invalidating the prior byte slice view.
                    bytes = s.as_bytes();
                    i += 3;
                    continue;
                }
                return Ok(i);
            }
            _ => return Ok(i),
        }
        i += 1;
    }
    // this was the last component!
    Ok(s.len())
}

/// Returns the NSS end
pub(crate) fn parse_nss(s: &mut TriCow, start: usize) -> Result<usize> {
    parse(s, start, PctEncoded::Nss)
}
/// Returns the r-component end
pub(crate) fn parse_r_component(s: &mut TriCow, start: usize) -> Result<usize> {
    parse(s, start, PctEncoded::RComponent)
}
/// Returns the q-component end
pub(crate) fn parse_q_component(s: &mut TriCow, start: usize) -> Result<usize> {
    parse(s, start, PctEncoded::QComponent)
}
/// Returns the f-component end
pub(crate) fn parse_f_component(s: &mut TriCow, start: usize) -> Result<usize> {
    parse(s, start, PctEncoded::FComponent)
}

/// Iterator that percent-decodes a component byte-by-byte without allocating.
///
/// Yields `Ok(byte)` for each decoded byte. Yields `Err(component_error)` and ends
/// iteration on the first validation failure. Callers that want the decoded bytes as
/// a `String` must validate UTF-8 themselves (e.g. via `String::from_utf8`).
#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
pub struct DecodeIter<'a> {
    bytes: &'a [u8],
    i: usize,
    kind: PctEncoded,
    err: Error,
    done: bool,
}

#[cfg(feature = "alloc")]
impl<'a> DecodeIter<'a> {
    const fn new(s: &'a str, kind: PctEncoded, err: Error) -> Self {
        Self {
            bytes: s.as_bytes(),
            i: 0,
            kind,
            err,
            done: false,
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a> Iterator for DecodeIter<'a> {
    type Item = Result<u8>;
    fn next(&mut self) -> Option<Result<u8>> {
        if self.done || self.i >= self.bytes.len() {
            return None;
        }
        let i = self.i;
        let ch = self.bytes[i];
        let fail = |this: &mut Self| {
            this.done = true;
            Some(Err(this.err))
        };
        let cls = BYTE_CLASS[ch as usize];
        if cls & PLAIN_PARSE != 0 {
            self.i = i + 1;
            return Some(Ok(ch));
        }
        match ch {
            b'?' => match self.kind {
                PctEncoded::FComponent => {}
                PctEncoded::QComponent if i != 0 => {}
                PctEncoded::RComponent if i != 0 && self.bytes.get(i + 1) != Some(&b'=') => {}
                _ => return fail(self),
            },
            b'/' => match self.kind {
                PctEncoded::FComponent => {}
                _ if i != 0 => {}
                _ => return fail(self),
            },
            b'%' => {
                if i + 2 >= self.bytes.len() {
                    return fail(self);
                }
                let hi = HEX_VAL[self.bytes[i + 1] as usize];
                let lo = HEX_VAL[self.bytes[i + 2] as usize];
                if hi == 0xFF || lo == 0xFF {
                    return fail(self);
                }
                self.i = i + 3;
                return Some(Ok((hi << 4) | lo));
            }
            _ => return fail(self),
        }
        self.i = i + 1;
        Some(Ok(ch))
    }
}

#[cfg(feature = "alloc")]
fn decode(s: &str, kind: PctEncoded) -> Option<String> {
    let mut ret = Vec::with_capacity(s.len());
    // Error value unused: we only surface success/failure via Option here.
    for byte in DecodeIter::new(s, kind, Error::InvalidNss) {
        ret.push(byte.ok()?);
    }
    String::from_utf8(ret).ok()
}

/// Percent-decode a NSS according to the RFC
///
/// ```
/// # use urn::Urn; fn test_main() -> Result<(), urn::Error> {
/// let urn = Urn::try_from("urn:example:string%20with%20spaces")?;
///
/// assert_eq!(
///     urn::percent::decode_nss(urn.nss())?,
///     "string with spaces"
/// );
/// # Ok(()) } test_main().unwrap();
/// ```
///
/// # Errors
/// Returns [`Error::InvalidNss`] in case of a validation failure.
#[cfg(feature = "alloc")]
pub fn decode_nss(s: &str) -> Result<String> {
    decode(s, PctEncoded::Nss).ok_or(Error::InvalidNss)
}
/// Percent-decode an r-component according to the RFC
///
/// ```
/// # use urn::Urn; fn test_main() -> Result<(), urn::Error> {
/// let urn = Urn::try_from("urn:example:nss?+this%20is%20the%20r-component!")?;
///
/// assert_eq!(
///     urn::percent::decode_r_component(urn.r_component().unwrap())?,
///     "this is the r-component!"
/// );
/// # Ok(()) } test_main().unwrap();
/// ```
///
/// # Errors
/// Returns [`Error::InvalidRComponent`] in case of a validation failure.
#[cfg(feature = "alloc")]
pub fn decode_r_component(s: &str) -> Result<String> {
    decode(s, PctEncoded::RComponent).ok_or(Error::InvalidRComponent)
}
/// Percent-decode a q-component according to the RFC
///
/// ```
/// # use urn::Urn; fn test_main() -> Result<(), urn::Error> {
/// let urn = Urn::try_from("urn:example:nss?=this%20is%20the%20q-component!")?;
///
/// assert_eq!(
///     urn::percent::decode_q_component(urn.q_component().unwrap())?,
///     "this is the q-component!"
/// );
/// # Ok(()) } test_main().unwrap();
/// ```
///
/// # Errors
/// Returns [`Error::InvalidQComponent`] in case of a validation failure.
#[cfg(feature = "alloc")]
pub fn decode_q_component(s: &str) -> Result<String> {
    decode(s, PctEncoded::QComponent).ok_or(Error::InvalidQComponent)
}
/// Percent-decode an f-component according to the RFC
///
/// ```
/// # use urn::Urn; fn test_main() -> Result<(), urn::Error> {
/// let urn = Urn::try_from("urn:example:nss#f-component%20test")?;
///
/// assert_eq!(
///     urn::percent::decode_f_component(urn.f_component().unwrap())?,
///     "f-component test"
/// );
/// # Ok(()) } test_main().unwrap();
/// ```
///
/// # Errors
/// Returns [`Error::InvalidFComponent`] in case of a validation failure.
#[cfg(feature = "alloc")]
pub fn decode_f_component(s: &str) -> Result<String> {
    decode(s, PctEncoded::FComponent).ok_or(Error::InvalidFComponent)
}

/// Percent-decode an NSS byte-by-byte without allocating.
///
/// The iterator yields `Ok(byte)` for each decoded byte, or `Err(Error::InvalidNss)`
/// once a validation failure is encountered (after which no further items are produced).
///
/// ```
/// # use urn::Urn; fn test_main() -> Result<(), urn::Error> {
/// let urn = Urn::try_from("urn:example:string%20with%20spaces")?;
/// let bytes: Result<Vec<u8>, _> = urn::percent::decode_nss_iter(urn.nss()).collect();
/// assert_eq!(bytes?, b"string with spaces");
/// # Ok(()) } test_main().unwrap();
/// ```
#[cfg(feature = "alloc")]
pub fn decode_nss_iter(s: &str) -> DecodeIter<'_> {
    DecodeIter::new(s, PctEncoded::Nss, Error::InvalidNss)
}

/// Percent-decode an r-component byte-by-byte without allocating. See [`decode_nss_iter`].
#[cfg(feature = "alloc")]
pub fn decode_r_component_iter(s: &str) -> DecodeIter<'_> {
    DecodeIter::new(s, PctEncoded::RComponent, Error::InvalidRComponent)
}

/// Percent-decode a q-component byte-by-byte without allocating. See [`decode_nss_iter`].
#[cfg(feature = "alloc")]
pub fn decode_q_component_iter(s: &str) -> DecodeIter<'_> {
    DecodeIter::new(s, PctEncoded::QComponent, Error::InvalidQComponent)
}

/// Percent-decode an f-component byte-by-byte without allocating. See [`decode_nss_iter`].
#[cfg(feature = "alloc")]
pub fn decode_f_component_iter(s: &str) -> DecodeIter<'_> {
    DecodeIter::new(s, PctEncoded::FComponent, Error::InvalidFComponent)
}

#[cfg(feature = "alloc")]
const fn to_hex(n: u8) -> [u8; 2] {
    let a = (n & 0xF0) >> 4;
    let b = n & 0xF;
    let a = if a < 10 { b'0' + a } else { b'A' + (a - 10) };
    let b = if b < 10 { b'0' + b } else { b'A' + (b - 10) };
    [a, b]
}

#[cfg(feature = "alloc")]
fn encode(s: &str, kind: PctEncoded) -> String {
    let bytes = s.as_bytes();
    let mut ret = String::with_capacity(bytes.len());
    let plain_mask = match kind {
        PctEncoded::Nss => PLAIN_ENC_NSS,
        _ => PLAIN_ENC_RQF,
    };
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b < 0x80 {
            let cls = BYTE_CLASS[b as usize];
            let allowed = cls & plain_mask != 0
                || match b {
                    b'?' => match kind {
                        PctEncoded::FComponent => true,
                        PctEncoded::QComponent => i != 0,
                        PctEncoded::RComponent => i != 0 && bytes.get(i + 1) != Some(&b'='),
                        PctEncoded::Nss => false,
                    },
                    b'/' => match kind {
                        PctEncoded::FComponent => true,
                        PctEncoded::RComponent | PctEncoded::QComponent => i != 0,
                        PctEncoded::Nss => false,
                    },
                    _ => false,
                };
            if allowed {
                // SAFETY: `b < 0x80`, so it's a valid single-byte UTF-8 scalar.
                ret.push(b as char);
            } else {
                let hex = to_hex(b);
                let triplet = [b'%', hex[0], hex[1]];
                // SAFETY: `to_hex` returns ASCII hex digits; '%' is ASCII. Three-byte
                // array is therefore valid UTF-8.
                ret.push_str(unsafe { core::str::from_utf8_unchecked(&triplet) });
            }
            i += 1;
        } else {
            // Non-ASCII: part of a multi-byte UTF-8 sequence. Find the full sequence
            // (continuation bytes have pattern 10xxxxxx) and percent-encode all of
            // its bytes in one batched push.
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i] & 0xC0) == 0x80 {
                i += 1;
            }
            // Max UTF-8 scalar is 4 bytes, so 4 * 3 = 12 encoded bytes suffice.
            let mut buf = [0u8; 12];
            let seq = &bytes[start..i];
            for (j, &byte) in seq.iter().enumerate() {
                let hex = to_hex(byte);
                buf[j * 3] = b'%';
                buf[j * 3 + 1] = hex[0];
                buf[j * 3 + 2] = hex[1];
            }
            let len = seq.len() * 3;
            // SAFETY: `to_hex` returns ASCII hex digits; '%' is ASCII. The written
            // prefix is therefore valid UTF-8.
            ret.push_str(unsafe { core::str::from_utf8_unchecked(&buf[..len]) });
        }
    }
    ret
}

/// Percent-decode a NSS according to the RFC
///
/// ```
/// # use urn::UrnBuilder; fn test_main() -> Result<(), urn::Error> {
/// assert_eq!(
///     UrnBuilder::new("example", &urn::percent::encode_nss("test nss")?)
///         .build()?
///         .as_str(),
///     "urn:example:test%20nss"
/// );
/// # Ok(()) } test_main().unwrap();
/// ```
///
/// # Errors
/// Returns [`Error::InvalidNss`] when attempting to encode an empty string.
#[cfg(feature = "alloc")]
pub fn encode_nss(s: &str) -> Result<String> {
    if s.is_empty() {
        return Err(Error::InvalidNss);
    }
    Ok(encode(s, PctEncoded::Nss))
}
/// Percent-decode an r-component according to the RFC
///
/// ```
/// # use urn::UrnBuilder; fn test_main() -> Result<(), urn::Error> {
/// assert_eq!(
///     UrnBuilder::new("example", "nss")
///         .r_component(Some(&urn::percent::encode_r_component("😂😂💯")?))
///         .build()?
///         .as_str(),
///     "urn:example:nss?+%F0%9F%98%82%F0%9F%98%82%F0%9F%92%AF"
/// );
/// # Ok(()) } test_main().unwrap();
/// ```
///
/// # Errors
/// Returns [`Error::InvalidRComponent`] when attempting to encode an empty string.
#[cfg(feature = "alloc")]
pub fn encode_r_component(s: &str) -> Result<String> {
    if s.is_empty() {
        return Err(Error::InvalidRComponent);
    }
    Ok(encode(s, PctEncoded::RComponent))
}
/// Percent-decode a q-component according to the RFC
///
/// ```
/// # use urn::UrnBuilder; fn test_main() -> Result<(), urn::Error> {
/// assert_eq!(
///     UrnBuilder::new("example", "nss")
///         .q_component(Some(&urn::percent::encode_q_component("~q component~")?))
///         .build()?
///         .as_str(),
///     "urn:example:nss?=%7Eq%20component%7E"
/// );
/// # Ok(()) } test_main().unwrap();
/// ```
///
/// # Errors
/// Returns [`Error::InvalidQComponent`] when attempting to encode an empty string.
#[cfg(feature = "alloc")]
pub fn encode_q_component(s: &str) -> Result<String> {
    if s.is_empty() {
        return Err(Error::InvalidQComponent);
    }
    Ok(encode(s, PctEncoded::QComponent))
}
/// Percent-decode an f-component according to the RFC
///
/// ```
/// # use urn::UrnBuilder; fn test_main() -> Result<(), urn::Error> {
/// assert_eq!(
///     UrnBuilder::new("example", "nss")
///         .f_component(Some(&urn::percent::encode_f_component("f-component (pretty much a fragment)")?))
///         .build()?
///         .as_str(),
///     "urn:example:nss#f-component%20(pretty%20much%20a%20fragment)"
/// );
/// # Ok(()) } test_main().unwrap();
/// ```
///
/// # Errors
/// None, this function returns a `Result` for API consistency. If the URN standard gets extended
/// in the future, this may return `Error::InvalidFComponent`.
#[cfg(feature = "alloc")]
pub fn encode_f_component(s: &str) -> Result<String> {
    // fragment is allowed to be empty
    Ok(encode(s, PctEncoded::FComponent))
}

#[cfg(test)]
mod swar_tests {
    use super::{BYTE_CLASS, PLAIN_PARSE, scan_plain_run};

    fn scan_plain_scalar(bytes: &[u8], mut i: usize) -> usize {
        while i < bytes.len() && BYTE_CLASS[bytes[i] as usize] & PLAIN_PARSE != 0 {
            i += 1;
        }
        i
    }

    #[test]
    fn swar_matches_scalar_all_prefixes() {
        // Deterministic pseudo-random buffer mixing plain and non-plain bytes.
        let mut buf = [0u8; 1024];
        let mut x: u32 = 0x1234_5678;
        for b in &mut buf {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *b = (x >> 16) as u8;
        }
        for len in 0..=buf.len() {
            for start in 0..=len {
                let a = scan_plain_run(&buf[..len], start);
                let b = scan_plain_scalar(&buf[..len], start);
                assert_eq!(a, b, "mismatch at len={len} start={start}");
            }
        }
    }

    #[test]
    fn swar_boundary_cases() {
        let all_plain = vec![b'A'; 33];
        assert_eq!(scan_plain_run(&all_plain, 0), 33);
        // Non-plain at every position across the 8-byte window boundary.
        for pos in 0..20 {
            let mut v = vec![b'A'; 20];
            v[pos] = b'#';
            assert_eq!(scan_plain_run(&v, 0), pos);
        }
    }
}

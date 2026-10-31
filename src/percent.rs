//! This module contains functions for percent-encoding and decoding various components of a URN.

use crate::{
    Error,
    Result,
    component::{PctEncoded, scan_run},
    tables::{BYTE_CLASS, HEX_VAL, PLAIN_ENC_NSS, PLAIN_ENC_RQF, PLAIN_PARSE},
};
#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};
use core::str::from_utf8_unchecked;

/// Iterator that percent-decodes a component byte-by-byte without allocating.
///
/// Yields `Ok(byte)` for each decoded byte. Yields `Err(component_error)` and ends
/// iteration on the first validation failure. Callers that want the decoded bytes as
/// a `String` must validate UTF-8 themselves (e.g. via `String::from_utf8`).
pub struct DecodeIter<'a> {
    bytes: &'a [u8],
    i: usize,
    kind: PctEncoded,
    err: Error,
    done: bool,
}

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

impl Iterator for DecodeIter<'_> {
    type Item = Result<u8>;
    fn next(&mut self) -> Option<Result<u8>> {
        if self.done {
            return None;
        }
        if self.i >= self.bytes.len() {
            // The NSS, r- and q-components need at least one character; the f-component may be empty.
            let may_be_empty = match self.kind {
                PctEncoded::FComponent => true,
                PctEncoded::Nss | PctEncoded::RComponent | PctEncoded::QComponent => false,
            };
            if self.bytes.is_empty() && !may_be_empty {
                self.done = true;
                return Some(Err(self.err));
            }
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
                PctEncoded::Nss | PctEncoded::RComponent | PctEncoded::QComponent => return fail(self),
            },
            b'/' => match self.kind {
                PctEncoded::FComponent => {}
                _ if i != 0 => {}
                PctEncoded::Nss | PctEncoded::RComponent | PctEncoded::QComponent => return fail(self),
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
/// # use urn_rs::Urn; fn test_main() -> Result<(), urn_rs::Error> {
/// let urn = Urn::try_from("urn:example:string%20with%20spaces")?;
///
/// assert_eq!(
///     urn_rs::percent::decode_nss(urn.nss())?,
///     "string with spaces"
/// );
/// # Ok(()) } test_main().unwrap();
/// ```
///
/// # Errors
/// Returns [`Error::InvalidNss`] in case of a validation failure.
pub fn decode_nss(s: &str) -> Result<String> {
    decode(s, PctEncoded::Nss).ok_or(Error::InvalidNss)
}
/// Percent-decode an r-component according to the RFC
///
/// ```
/// # use urn_rs::Urn; fn test_main() -> Result<(), urn_rs::Error> {
/// let urn = Urn::try_from("urn:example:nss?+this%20is%20the%20r-component!")?;
///
/// assert_eq!(
///     urn_rs::percent::decode_r_component(urn.r_component().unwrap())?,
///     "this is the r-component!"
/// );
/// # Ok(()) } test_main().unwrap();
/// ```
///
/// # Errors
/// Returns [`Error::InvalidRComponent`] in case of a validation failure.
pub fn decode_r_component(s: &str) -> Result<String> {
    decode(s, PctEncoded::RComponent).ok_or(Error::InvalidRComponent)
}
/// Percent-decode a q-component according to the RFC
///
/// ```
/// # use urn_rs::Urn; fn test_main() -> Result<(), urn_rs::Error> {
/// let urn = Urn::try_from("urn:example:nss?=this%20is%20the%20q-component!")?;
///
/// assert_eq!(
///     urn_rs::percent::decode_q_component(urn.q_component().unwrap())?,
///     "this is the q-component!"
/// );
/// # Ok(()) } test_main().unwrap();
/// ```
///
/// # Errors
/// Returns [`Error::InvalidQComponent`] in case of a validation failure.
pub fn decode_q_component(s: &str) -> Result<String> {
    decode(s, PctEncoded::QComponent).ok_or(Error::InvalidQComponent)
}
/// Percent-decode an f-component according to the RFC
///
/// ```
/// # use urn_rs::Urn; fn test_main() -> Result<(), urn_rs::Error> {
/// let urn = Urn::try_from("urn:example:nss#f-component%20test")?;
///
/// assert_eq!(
///     urn_rs::percent::decode_f_component(urn.f_component().unwrap())?,
///     "f-component test"
/// );
/// # Ok(()) } test_main().unwrap();
/// ```
///
/// # Errors
/// Returns [`Error::InvalidFComponent`] in case of a validation failure.
pub fn decode_f_component(s: &str) -> Result<String> {
    decode(s, PctEncoded::FComponent).ok_or(Error::InvalidFComponent)
}

/// Percent-decode an NSS byte-by-byte without allocating.
///
/// The iterator yields `Ok(byte)` for each decoded byte, or `Err(Error::InvalidNss)`
/// once a validation failure is encountered (after which no further items are produced).
///
/// ```
/// # use urn_rs::Urn; fn test_main() -> Result<(), urn_rs::Error> {
/// let urn = Urn::try_from("urn:example:string%20with%20spaces")?;
/// let bytes: Result<Vec<u8>, _> = urn_rs::percent::decode_nss_iter(urn.nss()).collect();
/// assert_eq!(bytes?, b"string with spaces");
/// # Ok(()) } test_main().unwrap();
/// ```
#[must_use]
pub const fn decode_nss_iter(s: &str) -> DecodeIter<'_> {
    DecodeIter::new(s, PctEncoded::Nss, Error::InvalidNss)
}

/// Percent-decode an r-component byte-by-byte without allocating. See [`decode_nss_iter`].
#[must_use]
pub const fn decode_r_component_iter(s: &str) -> DecodeIter<'_> {
    DecodeIter::new(s, PctEncoded::RComponent, Error::InvalidRComponent)
}

/// Percent-decode a q-component byte-by-byte without allocating. See [`decode_nss_iter`].
#[must_use]
pub const fn decode_q_component_iter(s: &str) -> DecodeIter<'_> {
    DecodeIter::new(s, PctEncoded::QComponent, Error::InvalidQComponent)
}

/// Percent-decode an f-component byte-by-byte without allocating. See [`decode_nss_iter`].
#[must_use]
pub const fn decode_f_component_iter(s: &str) -> DecodeIter<'_> {
    DecodeIter::new(s, PctEncoded::FComponent, Error::InvalidFComponent)
}

const fn to_hex(n: u8) -> [u8; 2] {
    let a = (n & 0xF0) >> 4;
    let b = n & 0xF;
    let a = if a < 10 { b'0' + a } else { b'A' + (a - 10) };
    let b = if b < 10 { b'0' + b } else { b'A' + (b - 10) };
    [a, b]
}

fn encode(s: &str, kind: PctEncoded) -> String {
    let bytes = s.as_bytes();
    let mut ret = String::with_capacity(bytes.len());
    let plain_mask = match kind {
        PctEncoded::Nss => PLAIN_ENC_NSS,
        PctEncoded::RComponent | PctEncoded::QComponent | PctEncoded::FComponent => PLAIN_ENC_RQF,
    };
    let mut i = 0;
    while i < bytes.len() {
        let run_end = scan_run(bytes, i, plain_mask);
        if run_end > i {
            // SAFETY: `plain_mask` bits in `BYTE_CLASS` are only set for ASCII
            // bytes (< 0x80), so the run is valid UTF-8.
            ret.push_str(unsafe { from_utf8_unchecked(&bytes[i..run_end]) });
            i = run_end;
            if i >= bytes.len() {
                break;
            }
        }
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
                ret.push_str(unsafe { from_utf8_unchecked(&triplet) });
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
            ret.push_str(unsafe { from_utf8_unchecked(&buf[..len]) });
        }
    }
    ret
}

/// Iterator that percent-encodes a component byte-by-byte without allocating.
///
/// Yields one ASCII byte at a time. Bytes that are allowed in their component
/// (per RFC 8141) pass through as-is; every other byte is replaced by its
/// `%XX` triplet (three bytes). Multi-byte UTF-8 scalars are encoded by
/// percent-encoding each of their bytes in turn.
pub struct EncodeIter<'a> {
    bytes: &'a [u8],
    i: usize,
    kind: PctEncoded,
    pending: [u8; 3],
    pending_len: u8,
    pending_pos: u8,
}

impl<'a> EncodeIter<'a> {
    const fn new(s: &'a str, kind: PctEncoded) -> Self {
        Self {
            bytes: s.as_bytes(),
            i: 0,
            kind,
            pending: [0; 3],
            pending_len: 0,
            pending_pos: 0,
        }
    }
}

impl Iterator for EncodeIter<'_> {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        if self.pending_pos < self.pending_len {
            let b = self.pending[self.pending_pos as usize];
            self.pending_pos += 1;
            return Some(b);
        }
        if self.i >= self.bytes.len() {
            return None;
        }
        let b = self.bytes[self.i];
        let plain_mask = match self.kind {
            PctEncoded::Nss => PLAIN_ENC_NSS,
            PctEncoded::RComponent | PctEncoded::QComponent | PctEncoded::FComponent => PLAIN_ENC_RQF,
        };
        if b < 0x80 {
            let cls = BYTE_CLASS[b as usize];
            let allowed = cls & plain_mask != 0
                || match b {
                    b'?' => match self.kind {
                        PctEncoded::FComponent => true,
                        PctEncoded::QComponent => self.i != 0,
                        PctEncoded::RComponent => self.i != 0 && self.bytes.get(self.i + 1) != Some(&b'='),
                        PctEncoded::Nss => false,
                    },
                    b'/' => match self.kind {
                        PctEncoded::FComponent => true,
                        PctEncoded::RComponent | PctEncoded::QComponent => self.i != 0,
                        PctEncoded::Nss => false,
                    },
                    _ => false,
                };
            self.i += 1;
            if allowed {
                return Some(b);
            }
        } else {
            self.i += 1;
        }
        let hex = to_hex(b);
        self.pending = [b'%', hex[0], hex[1]];
        self.pending_len = 3;
        self.pending_pos = 1;
        Some(b'%')
    }
}

/// Percent-encode an NSS byte-by-byte without allocating. See [`encode_nss`].
#[must_use]
pub const fn encode_nss_iter(s: &str) -> EncodeIter<'_> {
    EncodeIter::new(s, PctEncoded::Nss)
}

/// Percent-encode an r-component byte-by-byte without allocating. See [`encode_r_component`].
#[must_use]
pub const fn encode_r_component_iter(s: &str) -> EncodeIter<'_> {
    EncodeIter::new(s, PctEncoded::RComponent)
}

/// Percent-encode a q-component byte-by-byte without allocating. See [`encode_q_component`].
#[must_use]
pub const fn encode_q_component_iter(s: &str) -> EncodeIter<'_> {
    EncodeIter::new(s, PctEncoded::QComponent)
}

/// Percent-encode an f-component byte-by-byte without allocating. See [`encode_f_component`].
#[must_use]
pub const fn encode_f_component_iter(s: &str) -> EncodeIter<'_> {
    EncodeIter::new(s, PctEncoded::FComponent)
}

/// Percent-decode a NSS according to the RFC
///
/// ```
/// # use urn_rs::UrnBuilder; fn test_main() -> Result<(), urn_rs::Error> {
/// assert_eq!(
///     UrnBuilder::new("example", &urn_rs::percent::encode_nss("test nss")?)
///         .build()?
///         .as_str(),
///     "urn:example:test%20nss"
/// );
/// # Ok(()) } test_main().unwrap();
/// ```
///
/// # Errors
/// Returns [`Error::InvalidNss`] when attempting to encode an empty string.
pub fn encode_nss(s: &str) -> Result<String> {
    if s.is_empty() {
        return Err(Error::InvalidNss);
    }
    Ok(encode(s, PctEncoded::Nss))
}
/// Percent-decode an r-component according to the RFC
///
/// ```
/// # use urn_rs::UrnBuilder; fn test_main() -> Result<(), urn_rs::Error> {
/// assert_eq!(
///     UrnBuilder::new("example", "nss")
///         .r_component(Some(&urn_rs::percent::encode_r_component("😂😂💯")?))
///         .build()?
///         .as_str(),
///     "urn:example:nss?+%F0%9F%98%82%F0%9F%98%82%F0%9F%92%AF"
/// );
/// # Ok(()) } test_main().unwrap();
/// ```
///
/// # Errors
/// Returns [`Error::InvalidRComponent`] when attempting to encode an empty string.
pub fn encode_r_component(s: &str) -> Result<String> {
    if s.is_empty() {
        return Err(Error::InvalidRComponent);
    }
    Ok(encode(s, PctEncoded::RComponent))
}
/// Percent-decode a q-component according to the RFC
///
/// ```
/// # use urn_rs::UrnBuilder; fn test_main() -> Result<(), urn_rs::Error> {
/// assert_eq!(
///     UrnBuilder::new("example", "nss")
///         .q_component(Some(&urn_rs::percent::encode_q_component("~q component~")?))
///         .build()?
///         .as_str(),
///     "urn:example:nss?=%7Eq%20component%7E"
/// );
/// # Ok(()) } test_main().unwrap();
/// ```
///
/// # Errors
/// Returns [`Error::InvalidQComponent`] when attempting to encode an empty string.
pub fn encode_q_component(s: &str) -> Result<String> {
    if s.is_empty() {
        return Err(Error::InvalidQComponent);
    }
    Ok(encode(s, PctEncoded::QComponent))
}
/// Percent-decode an f-component according to the RFC
///
/// ```
/// # use urn_rs::UrnBuilder; fn test_main() -> Result<(), urn_rs::Error> {
/// assert_eq!(
///     UrnBuilder::new("example", "nss")
///         .f_component(Some(&urn_rs::percent::encode_f_component("f-component (pretty much a fragment)")?))
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
pub fn encode_f_component(s: &str) -> Result<String> {
    // fragment is allowed to be empty
    Ok(encode(s, PctEncoded::FComponent))
}

//! Syntax of the URN components after the NID: parsing with in-place normalization, validating
//! a standalone value (for the builder and the setters), and the plain-run scanner the
//! percent-encoder shares.

use crate::{
    Result,
    TriCow,
    tables::{BYTE_CLASS, HEX, PLAIN_PARSE},
};
use core::ops::Range;

/// Different components are percent-encoded differently...
#[derive(Copy, Clone)]
pub(crate) enum PctEncoded {
    Nss,
    RComponent,
    QComponent,
    FComponent,
}

/// Scan a contiguous run of bytes whose `BYTE_CLASS` entry has a bit of `mask` set, starting at
/// `i`, advancing 8 at a time via parallel table lookups. Returns the first index that is either
/// out of bounds or outside the run. The bulk path keeps 8 independent loads in flight so modern
/// cores can saturate their load ports even though we don't use explicit SIMD intrinsics.
///
/// The parser scans with `PLAIN_PARSE`; the encoder with `PLAIN_ENC_NSS` / `PLAIN_ENC_RQF`, whose
/// bits are only set for ASCII bytes, so a run it gets back is valid UTF-8.
#[inline]
pub(crate) fn scan_run(bytes: &[u8], mut i: usize, mask: u8) -> usize {
    while i + 8 <= bytes.len() {
        // 8-byte copy lowers to a single unaligned load.
        let mut c = [0u8; 8];
        c.copy_from_slice(&bytes[i..i + 8]);
        let mut outside: u32 = 0;
        // Unrolled so each table lookup is independent.
        for k in 0..8 {
            if BYTE_CLASS[c[k] as usize] & mask == 0 {
                outside |= 1 << k;
            }
        }
        if outside == 0 {
            i += 8;
        } else {
            return i + outside.trailing_zeros() as usize;
        }
    }
    while i < bytes.len() && BYTE_CLASS[bytes[i] as usize] & mask != 0 {
        i += 1;
    }
    i
}

/// Scan `bytes[start..]` for any well-formed `%xx` triplet with a lowercase hex
/// digit — the only shape that requires uppercase normalization during parse.
fn scan_needs_hex_upper(bytes: &[u8], start: usize) -> bool {
    let mut i = start;
    while i + 2 < bytes.len() {
        if bytes[i] == b'%' && BYTE_CLASS[bytes[i + 1] as usize] & HEX != 0 && BYTE_CLASS[bytes[i + 2] as usize] & HEX != 0 {
            if bytes[i + 1].is_ascii_lowercase() || bytes[i + 2].is_ascii_lowercase() {
                return true;
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    false
}

/// Parse and normalize percent-encoded string. Returns the end.
fn parse(s: &mut TriCow, start: usize, kind: PctEncoded) -> Result<usize> {
    // One-shot: detect any triplet with lowercase hex so we can (a) skip
    // `make_uppercase` calls entirely when not needed, and (b) promote Borrowed
    // -> Owned up front rather than incur the per-triplet scan inside
    // `make_uppercase`'s Borrowed arm.
    let needs_upper = scan_needs_hex_upper(s.as_bytes(), start);
    #[cfg(feature = "alloc")]
    if needs_upper && matches!(s, TriCow::Borrowed(_)) {
        s.ensure_owned();
    }
    let mut bytes = s.as_bytes();
    let mut i = start;
    while i < bytes.len() {
        // Bulk-skip plain pchar runs; drops to scalar for specials (%, ?, /) and
        // for bytes outside the pchar set.
        i = scan_run(bytes, i, PLAIN_PARSE);
        if i >= bytes.len() {
            break;
        }
        let ch = bytes[i];
        match ch {
            b'?' => match kind {
                PctEncoded::FComponent => {}
                PctEncoded::QComponent if i != start => {}
                PctEncoded::RComponent if i != start && bytes.get(i + 1) != Some(&b'=') => {}
                PctEncoded::Nss | PctEncoded::RComponent | PctEncoded::QComponent => return Ok(i),
            },
            b'/' => match kind {
                PctEncoded::FComponent => {}
                _ if i != start => {}
                PctEncoded::Nss | PctEncoded::RComponent | PctEncoded::QComponent => return Ok(i),
            },
            b'%' => {
                if i + 2 < bytes.len() && BYTE_CLASS[bytes[i + 1] as usize] & HEX != 0 && BYTE_CLASS[bytes[i + 2] as usize] & HEX != 0 {
                    if needs_upper {
                        // s is Owned/MutBorrowed; uppercase in place, no promotion.
                        s.make_uppercase(i + 1..i + 3)?;
                        bytes = s.as_bytes();
                    }
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

/// Validate a percent-encoded component without mutating. Mirrors `parse()` logic
/// byte-for-byte, but records whether any `%xx` triplet has an ASCII-lowercase hex
/// digit (meaning the caller must normalize) rather than uppercasing in place.
/// Returns `(end, needs_norm)`.
fn validate(s: &str, start: usize, kind: PctEncoded) -> (usize, bool) {
    let bytes = s.as_bytes();
    let mut i = start;
    let mut needs_norm = false;
    while i < bytes.len() {
        i = scan_run(bytes, i, PLAIN_PARSE);
        if i >= bytes.len() {
            break;
        }
        let ch = bytes[i];
        match ch {
            b'?' => match kind {
                PctEncoded::FComponent => {}
                PctEncoded::QComponent if i != start => {}
                PctEncoded::RComponent if i != start && bytes.get(i + 1) != Some(&b'=') => {}
                PctEncoded::Nss | PctEncoded::RComponent | PctEncoded::QComponent => return (i, needs_norm),
            },
            b'/' => match kind {
                PctEncoded::FComponent => {}
                _ if i != start => {}
                PctEncoded::Nss | PctEncoded::RComponent | PctEncoded::QComponent => return (i, needs_norm),
            },
            b'%' => {
                if i + 2 < bytes.len() && BYTE_CLASS[bytes[i + 1] as usize] & HEX != 0 && BYTE_CLASS[bytes[i + 2] as usize] & HEX != 0 {
                    if bytes[i + 1].is_ascii_lowercase() || bytes[i + 2].is_ascii_lowercase() {
                        needs_norm = true;
                    }
                    i += 3;
                    continue;
                }
                return (i, needs_norm);
            }
            _ => return (i, needs_norm),
        }
        i += 1;
    }
    (bytes.len(), needs_norm)
}

#[inline]
pub(crate) fn validate_nss(s: &str) -> (usize, bool) {
    validate(s, 0, PctEncoded::Nss)
}
#[inline]
pub(crate) fn validate_r_component(s: &str) -> (usize, bool) {
    validate(s, 0, PctEncoded::RComponent)
}
#[inline]
pub(crate) fn validate_q_component(s: &str) -> (usize, bool) {
    validate(s, 0, PctEncoded::QComponent)
}
#[inline]
pub(crate) fn validate_f_component(s: &str) -> (usize, bool) {
    validate(s, 0, PctEncoded::FComponent)
}

/// Uppercase the hex digits of every `%xx` triplet in `range`. Caller must have
/// validated the range — malformed `%` tails are ignored rather than rejected.
pub(crate) fn normalize_range(s: &mut TriCow, range: Range<usize>) -> Result<()> {
    let mut i = range.start;
    while i + 2 < range.end {
        if s.as_bytes()[i] == b'%' {
            s.make_uppercase(i + 1..i + 3)?;
            i += 3;
        } else {
            i += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod swar_tests {
    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;

    use proptest::{collection::vec, prelude::*};

    use super::{scan_needs_hex_upper, scan_run};
    use crate::tables::{BYTE_CLASS, PLAIN_ENC_NSS, PLAIN_ENC_RQF, PLAIN_PARSE};

    /// Scalar reference for [`scan_run`]: one byte at a time.
    fn scan_scalar(bytes: &[u8], mut i: usize, mask: u8) -> usize {
        while i < bytes.len() && BYTE_CLASS[bytes[i] as usize] & mask != 0 {
            i += 1;
        }
        i
    }

    /// Scalar reference for [`scan_needs_hex_upper`]: any `%xx` triplet with a lowercase hex digit.
    fn needs_hex_upper_scalar(bytes: &[u8], start: usize) -> bool {
        bytes[start..]
            .windows(3)
            .any(|w| w[0] == b'%' && w[1].is_ascii_hexdigit() && w[2].is_ascii_hexdigit() && (w[1].is_ascii_lowercase() || w[2].is_ascii_lowercase()))
    }

    /// Bytes weighted towards the classes the scanners tell apart, with runs long enough to cross
    /// several 8-byte batches.
    fn bytes() -> impl Strategy<Value = Vec<u8>> {
        let byte = prop_oneof![
            4 => prop::sample::select(b"aAzZ09-._~!$&'()*+,;=:@".as_slice()),
            2 => prop::sample::select(b"%%%abcdefABCDEF".as_slice()),
            1 => prop::sample::select(b"?/# \"<>[]^`{|}\x00\x7f\x80\xc3\xff".as_slice()),
            1 => any::<u8>(),
        ];
        vec(byte, 0..64)
    }

    proptest! {
        #[test]
        fn scan_run_matches_scalar(bytes in bytes(), start in any::<prop::sample::Index>()) {
            let start = start.index(bytes.len() + 1);
            for mask in [PLAIN_PARSE, PLAIN_ENC_NSS, PLAIN_ENC_RQF] {
                prop_assert_eq!(scan_run(&bytes, start, mask), scan_scalar(&bytes, start, mask));
            }
        }

        #[test]
        fn scan_needs_hex_upper_matches_scalar(bytes in bytes(), start in any::<prop::sample::Index>()) {
            let start = start.index(bytes.len() + 1);
            prop_assert_eq!(scan_needs_hex_upper(&bytes, start), needs_hex_upper_scalar(&bytes, start));
        }
    }

    #[test]
    fn swar_matches_scalar_all_prefixes() {
        // Deterministic pseudo-random buffer mixing plain and non-plain bytes.
        let mut buf = [0u8; 1024];
        let mut x: u32 = 0x1234_5678;
        for b in &mut buf {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            *b = x.to_le_bytes()[2];
        }
        for len in 0..=buf.len() {
            for start in 0..=len {
                let a = scan_run(&buf[..len], start, PLAIN_PARSE);
                let b = scan_scalar(&buf[..len], start, PLAIN_PARSE);
                assert_eq!(a, b, "mismatch at len={len} start={start}");
            }
        }
    }

    #[test]
    fn swar_boundary_cases() {
        let all_plain = [b'A'; 33];
        assert_eq!(scan_run(&all_plain, 0, PLAIN_PARSE), 33);
        // Non-plain at every position across the 8-byte window boundary.
        for pos in 0..20 {
            let mut v = [b'A'; 20];
            v[pos] = b'#';
            assert_eq!(scan_run(&v, 0, PLAIN_PARSE), pos);
        }
    }
}

//! Private byte-class lookup tables used by the parse/encode/decode hot paths.
//!
//! The `BYTE_CLASS` table assigns a bitmask to each possible byte value (0..=255)
//! encoding which character classes it belongs to. Hot loops replace per-byte
//! `match` arms and `u8::is_ascii_*` method calls with a single table load plus
//! a bitwise AND against the relevant mask.
//!
//! The `HEX_VAL` table maps ASCII hex digits to their numeric value (0x00..=0x0F);
//! all other bytes map to `0xFF` so a single `!= 0xFF` test both validates and
//! decodes a hex digit.

// Bit definitions for BYTE_CLASS entries.

/// Plain pchar: every pchar but the pct-encoded triplet, i.e. `unreserved / sub-delims / ":" / "@"`
/// (alphanumeric | `-` | `.` | `_` | `~` | `!` | `$` | `&` | `'` | `(` | `)` | `*` | `+` | `,` |
/// `;` | `=` | `:` | `@`). Every component may hold these as written: the parser accepts them and
/// the encoder leaves them alone.
pub(crate) const PLAIN_PCHAR: u8 = 1 << 0;

/// ASCII hex digit (`0`–`9`, `A`–`F`, `a`–`f`).
pub(crate) const HEX: u8 = 1 << 1;

/// NID byte: alphanumeric or `-`.
pub(crate) const NID: u8 = 1 << 2;

const fn build_byte_class() -> [u8; 256] {
    let mut t = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        #[expect(clippy::cast_possible_truncation, reason = "the loop keeps `i` below 256")]
        let b = i as u8;
        let mut m = 0u8;
        let is_alpha = b.is_ascii_alphabetic();
        let is_digit = b.is_ascii_digit();
        let is_alnum = is_alpha || is_digit;
        let is_hex = is_digit || matches!(b, b'A'..=b'F' | b'a'..=b'f');

        if is_alnum
            || matches!(
                b,
                b'-' | b'.' | b'_' | b'~' | b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' | b':' | b'@'
            )
        {
            m |= PLAIN_PCHAR;
        }

        if is_hex {
            m |= HEX;
        }

        if is_alnum || b == b'-' {
            m |= NID;
        }

        t[i] = m;
        i += 1;
    }
    t
}

pub(crate) static BYTE_CLASS: [u8; 256] = build_byte_class();

#[cfg(feature = "alloc")]
const fn build_hex_val() -> [u8; 256] {
    let mut t = [0xFFu8; 256];
    let mut i = 0;
    while i < 256 {
        #[expect(clippy::cast_possible_truncation, reason = "the loop keeps `i` below 256")]
        let b = i as u8;
        let v = match b {
            b'0'..=b'9' => b - b'0',
            b'A'..=b'F' => b - b'A' + 10,
            b'a'..=b'f' => b - b'a' + 10,
            _ => 0xFF,
        };
        t[i] = v;
        i += 1;
    }
    t
}

/// Hex digit value table: index by byte, get numeric value 0..=15. Non-hex bytes
/// map to `0xFF` so `HEX_VAL[b] != 0xFF` is a combined validate+decode check.
#[cfg(feature = "alloc")]
pub(crate) static HEX_VAL: [u8; 256] = build_hex_val();

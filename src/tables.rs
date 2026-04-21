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

/// pchar set accepted by `parse()` in every kind (alphanumeric | `-` | `.` | `_` |
/// `~` | `!` | `$` | `&` | `'` | `(` | `)` | `*` | `+` | `,` | `;` | `=` | `:` | `@`).
pub(crate) const PLAIN_PARSE: u8 = 1 << 0;

/// Encoder "no percent-encode" set for r/q/f components:
/// alphanumeric | `-` | `.` | `_` | `!` | `$` | `&` | `'` | `(` | `)` | `*` | `+`
/// | `,` | `;` | `=` | `:` | `@`. (Note: `~` is *not* included — the encoder
/// percent-encodes it for RFC 2141 compatibility.)
pub(crate) const PLAIN_ENC_RQF: u8 = 1 << 1;

/// Encoder "no percent-encode" set for NSS: same as `PLAIN_ENC_RQF` but also
/// excludes `&`.
pub(crate) const PLAIN_ENC_NSS: u8 = 1 << 2;

/// ASCII hex digit (`0`–`9`, `A`–`F`, `a`–`f`).
pub(crate) const HEX: u8 = 1 << 3;

/// NID byte: alphanumeric or `-`.
pub(crate) const NID: u8 = 1 << 4;

const fn build_byte_class() -> [u8; 256] {
    let mut t = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        let b = i as u8;
        let mut m = 0u8;
        let is_alpha = b.is_ascii_alphabetic();
        let is_digit = b.is_ascii_digit();
        let is_alnum = is_alpha || is_digit;
        let is_hex = is_digit || matches!(b, b'A'..=b'F' | b'a'..=b'f');

        // parse plain pchar
        let parse_plain = is_alnum
            || matches!(
                b,
                b'-' | b'.'
                    | b'_'
                    | b'~'
                    | b'!'
                    | b'$'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b';'
                    | b'='
                    | b':'
                    | b'@'
            );
        if parse_plain {
            m |= PLAIN_PARSE;
        }

        // encoder r/q/f plain set (same as parse plain minus `~`)
        let enc_rqf = is_alnum
            || matches!(
                b,
                b'-' | b'.'
                    | b'_'
                    | b'!'
                    | b'$'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b';'
                    | b'='
                    | b':'
                    | b'@'
            );
        if enc_rqf {
            m |= PLAIN_ENC_RQF;
        }

        // NSS encoder plain: RQF minus `&`
        if enc_rqf && b != b'&' {
            m |= PLAIN_ENC_NSS;
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

const fn build_hex_val() -> [u8; 256] {
    let mut t = [0xFFu8; 256];
    let mut i = 0;
    while i < 256 {
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

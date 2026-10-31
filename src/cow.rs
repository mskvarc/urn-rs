#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{borrow::ToOwned, string::String};
use core::{
    ops::{Deref, Range},
    slice::SliceIndex,
};

#[cfg(not(feature = "alloc"))]
use super::Error;
use super::Result;

pub(crate) enum TriCow<'a> {
    #[cfg(feature = "alloc")]
    Owned(String),
    Borrowed(&'a str),
    MutBorrowed(&'a mut str),
}

/// `Owned` clones its buffer and `Borrowed` copies the reference. `MutBorrowed` is copied into a
/// new `Owned` buffer, because a `&'a mut str` can't be shared. Without `alloc` that copy is
/// impossible, so `Clone` is only implemented when `alloc` is enabled.
#[cfg(feature = "alloc")]
impl Clone for TriCow<'_> {
    fn clone(&self) -> Self {
        match self {
            Self::Owned(s) => Self::Owned(s.clone()),
            Self::Borrowed(s) => Self::Borrowed(s),
            Self::MutBorrowed(s) => Self::Owned((**s).to_owned()),
        }
    }
}

impl Deref for TriCow<'_> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        match self {
            #[cfg(feature = "alloc")]
            Self::Owned(s) => s,
            Self::Borrowed(s) => s,
            Self::MutBorrowed(s) => s,
        }
    }
}

/// Return true if any byte in `bytes` is ASCII uppercase (`A`..=`Z`). Uses an
/// 8-byte SWAR batch: for each byte, compute `b & 0x7F` then use two carries to
/// test the range `[0x41, 0x5A]`, masked by bytes whose original high bit was 0.
#[inline]
fn has_ascii_upper(bytes: &[u8]) -> bool {
    let mut chunks = bytes.chunks_exact(8);
    for chunk in &mut chunks {
        let mut arr = [0u8; 8];
        arr.copy_from_slice(chunk);
        let w = u64::from_ne_bytes(arr);
        let low7 = w & 0x7F7F_7F7F_7F7F_7F7F;
        // top bit of each byte set iff low7 >= 0x41
        let ge_a = low7.wrapping_add(0x3F3F_3F3F_3F3F_3F3F) & 0x8080_8080_8080_8080;
        // top bit of each byte set iff low7 >= 0x5B (i.e. > 'Z')
        let gt_z = low7.wrapping_add(0x2525_2525_2525_2525) & 0x8080_8080_8080_8080;
        // in_range = ge_a AND NOT gt_z; XOR works because gt_z ⊆ ge_a.
        let in_range = ge_a ^ gt_z;
        // keep only bytes whose original high bit was 0 (true ASCII)
        let ascii = !w & 0x8080_8080_8080_8080;
        if in_range & ascii != 0 {
            return true;
        }
    }
    chunks.remainder().iter().any(u8::is_ascii_uppercase)
}

impl TriCow<'_> {
    /// Take the buffer as a `String`, moving it when already owned and copying otherwise.
    #[cfg(feature = "alloc")]
    pub(crate) fn into_string(self) -> String {
        match self {
            TriCow::Owned(s) => s,
            TriCow::Borrowed(s) => s.to_owned(),
            TriCow::MutBorrowed(s) => s.to_owned(),
        }
    }

    /// Promote `Borrowed` to `Owned`; no-op for `Owned` / `MutBorrowed`. Requires
    /// `alloc` since `Borrowed` can only be upgraded by copying into a `String`.
    #[cfg(feature = "alloc")]
    pub(crate) fn ensure_owned(&mut self) {
        if let TriCow::Borrowed(s) = *self {
            *self = TriCow::Owned(s.to_owned());
        }
    }

    #[cfg_attr(
        feature = "alloc",
        expect(clippy::unnecessary_wraps, reason = "only fails without `alloc`, where it can't reallocate")
    )]
    pub(crate) fn replace_range(&mut self, range: Range<usize>, with: &str) -> Result<()> {
        match self {
            #[cfg(feature = "alloc")]
            TriCow::Owned(s) => {
                s.replace_range(range, with);
                Ok(())
            }
            #[cfg(feature = "alloc")]
            TriCow::Borrowed(s) => {
                let mut s = s.to_owned();
                s.replace_range(range, with);
                *self = TriCow::Owned(s);
                Ok(())
            }
            #[cfg(not(feature = "alloc"))]
            TriCow::Borrowed(_) => Err(Error::AllocRequired),
            TriCow::MutBorrowed(s) => {
                if range.len() == with.len() {
                    // A fresh range instead of `range.clone()`: without `alloc`, `range` isn't
                    // used again and the clone would be redundant.
                    if let Some(slice) = s.get_mut(range.start..range.end) {
                        // SAFETY: `range.len() == with.len()` is checked above, so
                        // `copy_from_slice` can't panic. Both `slice` and `with` originate
                        // from `&str`, so they are valid UTF-8 of identical byte length;
                        // overwriting `slice`'s bytes with `with`'s bytes preserves UTF-8
                        // validity (we're replacing a complete UTF-8 sequence with another
                        // of the same length). `s.get_mut(range)` returned `Some`, which
                        // guarantees `range` falls on char boundaries.
                        unsafe { slice.as_bytes_mut() }.copy_from_slice(with.as_bytes());
                        return Ok(());
                    }
                }
                #[cfg(feature = "alloc")]
                {
                    let mut s = s.to_owned();
                    s.replace_range(range, with);
                    *self = TriCow::Owned(s);
                    Ok(())
                }
                #[cfg(not(feature = "alloc"))]
                Err(Error::AllocRequired)
            }
        }
    }
    /// # Panics
    /// Panics if range isn't at valid character boundaries
    pub(crate) fn make_uppercase<R>(&mut self, range: R) -> Result<()>
    where
        R: Clone + SliceIndex<[u8], Output = [u8]> + SliceIndex<str, Output = str>,
    {
        self.ascii_case(range, |b| b.iter().any(u8::is_ascii_lowercase), str::make_ascii_uppercase)
    }

    /// # Panics
    /// Panics if range isn't at valid character boundaries
    pub(crate) fn make_lowercase<R>(&mut self, range: R) -> Result<()>
    where
        R: Clone + SliceIndex<[u8], Output = [u8]> + SliceIndex<str, Output = str>,
    {
        // Non-ASCII input isn't rejected here; validation fails on it later.
        self.ascii_case(range, has_ascii_upper, str::make_ascii_lowercase)
    }

    /// Apply `change` to `range`. A `Borrowed` buffer is only copied into an `Owned` one when
    /// `needs_change` reports that the range would actually change.
    #[cfg_attr(feature = "alloc", expect(clippy::unnecessary_wraps, reason = "only fails without `alloc`, where it can't copy"))]
    fn ascii_case<R>(&mut self, range: R, needs_change: fn(&[u8]) -> bool, change: fn(&mut str)) -> Result<()>
    where
        R: Clone + SliceIndex<[u8], Output = [u8]> + SliceIndex<str, Output = str>,
    {
        let s: &mut str = match self {
            #[cfg(feature = "alloc")]
            TriCow::Owned(s) => s,
            TriCow::MutBorrowed(s) => s,
            TriCow::Borrowed(s) => {
                #[cfg(feature = "alloc")]
                if needs_change(&s.as_bytes()[range.clone()]) {
                    let mut owned = (*s).to_owned();
                    change(&mut owned[range]);
                    *self = TriCow::Owned(owned);
                }
                #[cfg(not(feature = "alloc"))]
                if needs_change(&s.as_bytes()[range]) {
                    return Err(Error::AllocRequired);
                }
                return Ok(());
            }
        };
        change(&mut s[range]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use proptest::{collection::vec, prelude::*};

    use super::has_ascii_upper;

    proptest! {
        /// The 8-byte SWAR batches agree with a byte-at-a-time check, including for bytes with the
        /// high bit set whose low 7 bits fall in `A..=Z`.
        #[test]
        fn has_ascii_upper_matches_scalar(
            bytes in vec(prop_oneof![prop::sample::select(b"@AZ[`az{\xc1\xda\x80\xff".as_slice()), any::<u8>()], 0..40),
        ) {
            prop_assert_eq!(has_ascii_upper(&bytes), bytes.iter().any(u8::is_ascii_uppercase));
        }
    }
}

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{borrow::ToOwned, string::String};
use core::{ops::Deref, slice::SliceIndex};

#[cfg(not(feature = "alloc"))]
use super::Error;
use super::Result;

#[allow(clippy::module_name_repetitions)]
pub enum TriCow<'a> {
    #[cfg(feature = "alloc")]
    Owned(String),
    Borrowed(&'a str),
    MutBorrowed(&'a mut str),
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

impl TriCow<'_> {
    #[allow(clippy::unnecessary_wraps)]
    pub fn replace_range(&mut self, range: core::ops::Range<usize>, with: &str) -> Result<()> {
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
                    #[cfg_attr(not(feature = "alloc"), allow(clippy::redundant_clone))]
                    if let Some(slice) = s.get_mut(range.clone()) {
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
    fn to_mut(&mut self) -> Result<&mut str> {
        match self {
            #[cfg(feature = "alloc")]
            TriCow::Owned(s) => Ok(s.as_mut_str()),
            #[cfg(feature = "alloc")]
            TriCow::Borrowed(s) => {
                *self = TriCow::Owned(s.to_owned());
                if let TriCow::Owned(s) = self {
                    Ok(s.as_mut_str())
                } else {
                    // Just assigned `TriCow::Owned(...)` on the line above; the variant
                    // can't have changed between the assignment and the match.
                    unreachable!("cow isn't owned after making it owned, what happened?")
                }
            }
            #[cfg(not(feature = "alloc"))]
            TriCow::Borrowed(_) => Err(Error::AllocRequired),
            TriCow::MutBorrowed(s) => Ok(s),
        }
    }
    /// # Panics
    /// Panics if range isn't at valid character boundaries
    pub fn make_uppercase<R>(&mut self, range: R) -> Result<()>
    where
        R: Clone + SliceIndex<[u8], Output = [u8]> + SliceIndex<str, Output = str>,
    {
        match self {
            #[cfg(feature = "alloc")]
            TriCow::Owned(s) => {
                s[range].make_ascii_uppercase();
                Ok(())
            }
            TriCow::MutBorrowed(s) => {
                s[range].make_ascii_uppercase();
                Ok(())
            }
            // Only promote Borrowed -> Owned if mutation is actually required.
            TriCow::Borrowed(_) => {
                if self.as_bytes()[range.clone()].iter().any(u8::is_ascii_lowercase) {
                    self.to_mut()?[range].make_ascii_uppercase();
                }
                Ok(())
            }
        }
    }
    /// # Panics
    /// Panics if range isn't at valid character boundaries
    pub fn make_lowercase<R>(&mut self, range: R) -> Result<()>
    where
        R: Clone + SliceIndex<[u8], Output = [u8]> + SliceIndex<str, Output = str>,
    {
        match self {
            #[cfg(feature = "alloc")]
            TriCow::Owned(s) => {
                // if this isn't ascii, it will fail later
                s[range].make_ascii_lowercase();
                Ok(())
            }
            TriCow::MutBorrowed(s) => {
                s[range].make_ascii_lowercase();
                Ok(())
            }
            TriCow::Borrowed(_) => {
                if self.as_bytes()[range.clone()].iter().any(u8::is_ascii_uppercase) {
                    self.to_mut()?[range].make_ascii_lowercase();
                }
                Ok(())
            }
        }
    }
}

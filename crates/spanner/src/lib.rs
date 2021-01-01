//! Operations on ranges.
#![cfg_attr(not(feature = "std"), no_std)]

use core::mem;
use core::ops::Range;
use core::slice;

/// Span between two memory addresses
///
/// This is a half open range, [begin, end)
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Span {
    begin: usize,
    end: usize,
}

impl<T> core::convert::From<&[T]> for Span {
    fn from(s: &[T]) -> Self {
        let r = s.as_ptr_range();
        Span::new(r.start as usize, r.end as usize)
    }
}

impl core::convert::From<Range<usize>> for Span {
    fn from(r: Range<usize>) -> Self {
        Span::new(r.start, r.end)
    }
}

impl core::fmt::Debug for Span {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Span")
            .field("begin", &format_args!("0x{:016x}", self.begin))
            .field("end", &format_args!("0x{:016x}", self.end))
            .finish()
    }
}

impl Span {
    /// Make a new [`Span`]
    pub fn new(begin: usize, end: usize) -> Span {
        assert!(begin <= end, "range had end before begin");
        Span { begin, end }
    }

    /// To what extent does this span intersect the given other span?
    ///
    /// Returns a span if there is a nonzero intersection between the two spans.
    ///
    /// ```rust
    /// use spanner::Span;
    /// let s11 = Span::new(1, 1);
    /// let s12 = Span::new(1, 2);
    /// let s13 = Span::new(1, 3);
    /// let s23 = Span::new(2, 3);
    /// let s24 = Span::new(2, 3);
    /// let s34 = Span::new(3, 4);
    ///
    /// // ranges that don't intersect enough
    /// assert_eq!(s12.intersect(s34), s34.intersect(s12));
    /// assert_eq!(s12.intersect(s34), None);
    /// assert_eq!(s12.intersect(s23), s23.intersect(s12));
    /// assert_eq!(s12.intersect(s23), None);
    /// assert_eq!(s11.intersect(s11), None);
    ///
    /// // ranges that do intersect
    /// assert_eq!(s13.intersect(s24), Some(Span::new(2, 3)));
    /// assert_eq!(s12.intersect(s12), Some(s12));
    /// ```
    pub fn intersect(&self, other: Span) -> Option<Span> {
        // possibilities:
        // [1, 2), [3, 4) => [3, 2), not a range, return None
        // [1, 2), [2, 3) => [2, 2), IS a range but still, return None
        // [1, 3), [2, 4) => [2, 3) is a range
        // [1, 2), [1, 2) => [1, 2) is a range
        // [1, 1), [1, 1) => [1, 1), ????

        let smol_begin = self.begin.max(other.begin);
        let smol_end = self.end.min(other.end);
        if smol_begin >= smol_end {
            None
        } else {
            Some(Span {
                begin: smol_begin,
                end: smol_end,
            })
        }
    }

    /// Merges two contiguous or overlapping [`Span`]s into one. Returns None if
    /// they do not overlap.
    ///
    /// # Examples
    /// ```rust
    /// use spanner::Span;
    /// let s12 = Span::new(1, 2); // [1, 2)
    /// let s23 = Span::new(2, 3); // [2, 3)
    /// let s13 = Span::new(1, 3); // [1, 3)
    /// let s34 = Span::new(3, 4); // [3, 4)
    ///
    /// assert_eq!(Some(s13), s12.merge(s23));
    /// assert_eq!(Some(s13), s23.merge(s12));
    ///
    /// assert_eq!(None, s12.merge(s34));
    /// assert_eq!(None, s34.merge(s12));
    ///
    /// assert_eq!(Some(s13), s13.merge(s13));
    /// assert_eq!(Some(s13), s13.merge(s12).unwrap().merge(s23));
    /// ```
    pub fn merge(&self, other: Span) -> Option<Span> {
        if self.begin.max(other.begin) > self.end.min(other.end) {
            // the ranges are not contiguous
            None
        } else {
            Some(Span {
                begin: self.begin.min(other.begin),
                end: self.end.max(other.end),
            })
        }
    }

    /// Offsets the beginning and ending of the span
    pub fn offset(&self, offs: isize) -> Span {
        Span {
            begin: (self.begin as isize + offs) as usize,
            end: (self.end as isize + offs) as usize,
        }
    }

    /// Gets the start of the range.
    pub fn begin(&self) -> usize {
        self.begin
    }

    /// Gets the end of the range.
    pub fn end(&self) -> usize {
        self.end
    }

    /// Makes a slice of T out of this range. This has the same safety invariants
    /// as [core::slice::from_raw_parts].
    pub unsafe fn as_slice<T>(&self) -> &[T] {
        slice::from_raw_parts(
            self.begin as *const T,
            self.end.wrapping_sub(self.begin) / mem::size_of::<T>(),
        )
    }

    /// Makes a slice of T out of this range. This has the same safety invariants
    /// as [core::slice::from_raw_parts_mut].
    pub unsafe fn as_slice_mut<T>(&self) -> &mut [T] {
        slice::from_raw_parts_mut(
            self.begin as *mut T,
            self.end.wrapping_sub(self.begin) / mem::size_of::<T>(),
        )
    }

    /// Length of the range
    pub fn len(&self) -> usize {
        self.end - self.begin
    }

    /// Does this range contain the given address?
    pub fn contains(&self, addr: usize) -> bool {
        self.begin >= addr && addr < self.end
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

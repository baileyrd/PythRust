//! `range` objects: a sequence (`r` descriptor) of three int slots,
//! `start`, `stop` and a nonzero `step`, standing for the ints they
//! produce. Lengths and positions use wide math: a range over all of
//! `i64` has more elements than `i64::MAX`. Mirrors the interpreter's
//! `Range`.

#![allow(unsafe_code)] // The C ABI below; the logic is safe.

use crate::seq::{range_fields, PtSeq};

/// The bounds of a range. `step` is never 0: `range()` checks it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Range {
    pub(crate) start: i64,
    pub(crate) stop: i64,
    pub(crate) step: i64,
}

impl Range {
    /// The number of elements, as Python computes it.
    pub(crate) fn len(&self) -> i128 {
        let (start, stop, step) = (
            i128::from(self.start),
            i128::from(self.stop),
            i128::from(self.step),
        );
        let n = match step > 0 {
            true => (stop - start + step - 1) / step,
            false => (start - stop - step - 1) / -step,
        };
        n.max(0)
    }

    /// Element `i`, negative counting from the end; `None` out of range.
    pub(crate) fn get(&self, i: i64) -> Option<i64> {
        let (len, i) = (self.len(), i128::from(i));
        let k = if i < 0 { i + len } else { i };
        if !(0..len).contains(&k) {
            return None;
        }
        // `k < len`, so the element lies between start and stop.
        i64::try_from(i128::from(self.start) + k * i128::from(self.step)).ok()
    }

    pub(crate) fn contains(&self, x: i64) -> bool {
        let inside = match self.step > 0 {
            true => self.start <= x && x < self.stop,
            false => self.stop < x && x <= self.start,
        };
        inside && (i128::from(x) - i128::from(self.start)) % i128::from(self.step) == 0
    }

    /// Equal when they produce the same ints, as in Python.
    pub(crate) fn same(&self, other: &Range) -> bool {
        let n = self.len();
        n == other.len()
            && (n == 0 || self.start == other.start && (n == 1 || self.step == other.step))
    }

    /// Python's `repr`: the step only when it is not 1.
    pub(crate) fn repr(&self) -> String {
        match self.step {
            1 => format!("range({}, {})", self.start, self.stop),
            step => format!("range({}, {}, {step})", self.start, self.stop),
        }
    }
}

/// `len(r)`, or -1 when the length does not fit in an int.
///
/// # Safety
///
/// `r` is a live range.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_range_len(r: *const PtSeq) -> i64 {
    // SAFETY: guaranteed by the caller.
    let r = unsafe { range_fields(r) };
    i64::try_from(r.len()).unwrap_or(-1)
}

/// `r[i]`; `*status` is 1, and the result 0, when `i` is out of range.
///
/// # Safety
///
/// `r` is a live range and `status` points to a writable byte.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_range_get(r: *const PtSeq, i: i64, status: *mut u8) -> i64 {
    // SAFETY: guaranteed by the caller.
    let item = unsafe { range_fields(r) }.get(i);
    // SAFETY: guaranteed by the caller.
    unsafe { *status = u8::from(item.is_none()) };
    item.unwrap_or(0)
}

/// `x in r` for an int (or bool) `x`.
///
/// # Safety
///
/// `r` is a live range.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_range_contains(r: *const PtSeq, x: i64) -> u8 {
    // SAFETY: guaranteed by the caller.
    u8::from(unsafe { range_fields(r) }.contains(x))
}

/// `x in r` for a float `x`: only an integral `x` can be in it.
///
/// # Safety
///
/// `r` is a live range.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_range_contains_float(r: *const PtSeq, x: f64) -> u8 {
    let i = x as i64;
    let integral = x.fract() == 0.0 && i as f64 == x;
    // SAFETY: guaranteed by the caller.
    u8::from(integral && unsafe { range_fields(r) }.contains(i))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(start: i64, stop: i64, step: i64) -> Range {
        Range { start, stop, step }
    }

    #[test]
    fn lengths_match_python() {
        assert_eq!(r(0, 10, 3).len(), 4);
        assert_eq!(r(5, 0, 1).len(), 0);
        assert_eq!(r(10, 0, -3).len(), 4);
        assert_eq!(r(i64::MIN, i64::MAX, 1).len(), (1i128 << 64) - 1);
    }

    #[test]
    fn items_count_from_either_end() {
        let down = r(10, 0, -3);
        assert_eq!(down.get(0), Some(10));
        assert_eq!(down.get(-1), Some(1));
        assert_eq!(down.get(4), None);
        assert_eq!(down.get(-5), None);
        assert_eq!(r(i64::MIN, i64::MAX, 1).get(-1), Some(i64::MAX - 1));
    }

    #[test]
    fn membership_follows_the_step() {
        assert!(r(0, 10, 3).contains(9));
        assert!(!r(0, 10, 3).contains(10));
        assert!(!r(0, 10, 3).contains(4));
        assert!(r(10, 0, -3).contains(1));
        assert!(!r(10, 0, -3).contains(0));
        assert!(r(i64::MIN, i64::MAX, i64::MAX).contains(-1));
    }

    #[test]
    fn equality_compares_the_ints_produced() {
        assert!(r(0, 0, 1).same(&r(5, 2, 3)));
        assert!(r(1, 2, 1).same(&r(1, 3, 5)));
        assert!(r(0, 10, 3).same(&r(0, 11, 3)));
        assert!(!r(0, 10, 3).same(&r(0, 10, 2)));
        assert_eq!(r(0, 3, 1).repr(), "range(0, 3)");
        assert_eq!(r(1, 9, 2).repr(), "range(1, 9, 2)");
    }
}

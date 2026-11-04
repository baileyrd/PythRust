//! A float `sum` as CPython 3.12+ computes it: Neumaier's compensated
//! summation. Safe Rust with no dependency, shared by the runtime's ABI
//! (`seq.rs`) and the interpreter, which compiles this file too (D57).

/// `start` plus each of `xs`, in order, with the running error
/// compensated as CPython's `sum` does; a compensation that is zero or
/// not finite is left out, so an infinite or overflowed sum stays one.
pub fn fsum(start: f64, xs: impl IntoIterator<Item = f64>) -> f64 {
    let (mut total, mut comp) = (start, 0.0_f64);
    for x in xs {
        let t = total + x;
        comp += if total.abs() >= x.abs() {
            (total - t) + x
        } else {
            (x - t) + total
        };
        total = t;
    }
    if comp != 0.0 && comp.is_finite() {
        total + comp
    } else {
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compensates_as_cpython_does() {
        // CPython 3.13: sum([0.1] * 10) == 1.0, where a naive sum is not.
        assert_eq!(fsum(0.0, [0.1; 10]), 1.0);
        assert_eq!(fsum(0.0, [1e100, 1.0, -1e100, 1.0]), 2.0);
        assert_eq!(fsum(0.5, []), 0.5);
        assert!(fsum(-0.0, [-0.0]).is_sign_negative());
    }

    #[test]
    fn keeps_infinities_and_nan() {
        assert_eq!(fsum(0.0, [f64::INFINITY, 1.0]), f64::INFINITY);
        assert_eq!(fsum(0.0, [1e308, 1e308]), f64::INFINITY);
        assert!(fsum(0.0, [f64::INFINITY, f64::NEG_INFINITY]).is_nan());
        assert!(fsum(0.0, [f64::NAN, 1.0]).is_nan());
    }
}

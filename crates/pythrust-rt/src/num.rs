//! Numbers: what generated code does not inline. Python's float `repr`,
//! floor division and modulo, `**`, and parsing `int()` and `float()`
//! from strings.
//!
//! These mirror the interpreter (`pythrust-compiler/src/interp`) so that
//! `build` prints what `run` prints; the native corpus test holds them to
//! it. Functions that can fail return the value and write a status byte
//! (0: ok) through a pointer into the caller's stack frame; generated
//! code turns a nonzero status into the runtime error.

#![allow(unsafe_code)] // The status out-pointers below; the logic is safe.

use crate::str::{into_ptr, PtStr};

/// Python's `repr` of a float: shortest round-trip digits, positional
/// for exponents in `-4..16`, scientific otherwise.
pub fn repr(f: f64) -> String {
    if f.is_nan() {
        return "nan".to_string();
    }
    if f.is_infinite() {
        return if f > 0.0 { "inf" } else { "-inf" }.to_string();
    }
    // `{:e}` always yields shortest round-trip digits and an exponent,
    // e.g. "-1.25e-7", so the fallbacks below are unreachable.
    let sci = format!("{f:e}");
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let (sign, mantissa) = match mantissa.strip_prefix('-') {
        Some(m) => ("-", m),
        None => ("", mantissa),
    };
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    if (-4..16).contains(&exp) {
        return format!("{sign}{}", positional(&digits, exp));
    }
    let frac = &digits[1..];
    let dot = if frac.is_empty() { "" } else { "." };
    let exp_sign = if exp < 0 { '-' } else { '+' };
    format!(
        "{sign}{}{dot}{frac}e{exp_sign}{:02}",
        &digits[..1],
        exp.abs()
    )
}

/// Places the decimal point in `digits` (d.ddd times 10**exp).
fn positional(digits: &str, exp: i32) -> String {
    if exp < 0 {
        let zeros = "0".repeat((-exp - 1) as usize);
        return format!("0.{zeros}{digits}");
    }
    let point = exp as usize + 1;
    if digits.len() <= point {
        let zeros = "0".repeat(point - digits.len());
        return format!("{digits}{zeros}.0");
    }
    format!("{}.{}", &digits[..point], &digits[point..])
}

/// CPython's `float_divmod`: floor quotient and divisor-signed remainder.
/// Precondition: `b != 0`.
fn divmod(a: f64, b: f64) -> (f64, f64) {
    let mut m = a % b;
    let mut div = (a - m) / b;
    if m != 0.0 {
        if (b < 0.0) != (m < 0.0) {
            m += b;
            div -= 1.0;
        }
    } else {
        m = 0.0_f64.copysign(b);
    }
    if div == 0.0 {
        return (0.0_f64.copysign(a / b), m);
    }
    let floor = div.floor();
    let q = if div - floor > 0.5 {
        floor + 1.0
    } else {
        floor
    };
    (q, m)
}

/// Why a float `**` failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PowError {
    /// `0.0 ** negative`: `ZeroDivisionError`.
    ZeroToNegative = 1,
    /// `negative ** fractional`, complex in Python: `ValueError`.
    Fractional = 2,
    /// A finite result too large: `OverflowError`.
    Overflow = 3,
}

/// Python's float `**`, without complex results.
pub fn float_pow(a: f64, b: f64) -> Result<f64, PowError> {
    if a == 0.0 && b < 0.0 {
        return Err(PowError::ZeroToNegative);
    }
    if a < 0.0 && b.is_finite() && b.fract() != 0.0 {
        return Err(PowError::Fractional);
    }
    let r = a.powf(b);
    if r.is_infinite() && a.is_finite() && b.is_finite() {
        return Err(PowError::Overflow);
    }
    Ok(r)
}

/// Why an int `**` failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IntPowError {
    Overflow = 1,
    /// Python gives a float; Pythrust's `int ** int` is an int.
    NegativeExponent = 2,
}

/// Checked `int ** int`.
pub fn int_pow(a: i64, b: i64) -> Result<i64, IntPowError> {
    if b < 0 {
        return Err(IntPowError::NegativeExponent);
    }
    let r = match u32::try_from(b) {
        Ok(e) => a.checked_pow(e),
        // Only 0, 1 and -1 stay in range for such exponents.
        Err(_) => match a {
            0 | 1 => Some(a),
            -1 => Some(if b % 2 == 0 { 1 } else { -1 }),
            _ => None,
        },
    };
    r.ok_or(IntPowError::Overflow)
}

/// The fix-it in a negative-exponent error: `2.0 ** -1`, `(-2.0) ** -1`.
pub fn pow_hint(a: i64, b: i64) -> String {
    match a < 0 {
        true => format!("({a}.0) ** {b}"),
        false => format!("{a}.0 ** {b}"),
    }
}

/// Removes `_` separators, each of which must sit between two digits, as
/// in Python's numeric strings. `None` if one does not.
fn strip_underscores(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let valid = bytes.iter().enumerate().all(|(i, b)| {
        *b != b'_'
            || (i > 0
                && bytes[i - 1].is_ascii_digit()
                && bytes.get(i + 1).is_some_and(u8::is_ascii_digit))
    });
    valid.then(|| s.replace('_', ""))
}

/// `float(s)`; `None` if `s` is not a float literal.
pub fn parse_float(s: &str) -> Option<f64> {
    strip_underscores(s.trim()).and_then(|t| t.parse().ok())
}

/// Why `int(s)` failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ParseIntError {
    Invalid = 1,
    Overflow = 2,
}

/// `int(s)` in base 10.
pub fn parse_int(s: &str) -> Result<i64, ParseIntError> {
    let text = strip_underscores(s.trim()).ok_or(ParseIntError::Invalid)?;
    let digits = text.strip_prefix(['+', '-']).unwrap_or(&text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ParseIntError::Invalid);
    }
    // Only digits and a sign remain, so the one failure left is range.
    text.parse().map_err(|_| ParseIntError::Overflow)
}

/// Writes `status` for the caller.
///
/// # Safety
///
/// `status` points to a writable byte.
unsafe fn report(status: *mut u8, code: u8) {
    // SAFETY: guaranteed by the caller.
    unsafe { *status = code };
}

/// Writes `f` as Python's `repr` does.
#[unsafe(no_mangle)]
pub extern "C" fn pt_write_float(f: f64) {
    crate::write(repr(f).as_bytes());
}

/// `str(f)`.
#[unsafe(no_mangle)]
pub extern "C" fn pt_str_from_float(f: f64) -> *mut PtStr {
    into_ptr(PtStr::new(repr(f).into()))
}

/// `a // b` on floats; `b` is nonzero (generated code checks).
#[unsafe(no_mangle)]
pub extern "C" fn pt_float_floordiv(a: f64, b: f64) -> f64 {
    divmod(a, b).0
}

/// `a % b` on floats; `b` is nonzero (generated code checks).
#[unsafe(no_mangle)]
pub extern "C" fn pt_float_mod(a: f64, b: f64) -> f64 {
    divmod(a, b).1
}

/// `a ** b` on floats; a [`PowError`] code in `*status` on failure.
///
/// # Safety
///
/// `status` points to a writable byte.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_float_pow(a: f64, b: f64, status: *mut u8) -> f64 {
    let (r, code) = match float_pow(a, b) {
        Ok(r) => (r, 0),
        Err(e) => (0.0, e as u8),
    };
    // SAFETY: guaranteed by the caller.
    unsafe { report(status, code) };
    r
}

/// `a ** b` on ints; an [`IntPowError`] code in `*status` on failure.
///
/// # Safety
///
/// `status` points to a writable byte.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_int_pow(a: i64, b: i64, status: *mut u8) -> i64 {
    let (r, code) = match int_pow(a, b) {
        Ok(r) => (r, 0),
        Err(e) => (0, e as u8),
    };
    // SAFETY: guaranteed by the caller.
    unsafe { report(status, code) };
    r
}

/// [`pow_hint`] as a new string.
#[unsafe(no_mangle)]
pub extern "C" fn pt_pow_hint(a: i64, b: i64) -> *mut PtStr {
    into_ptr(PtStr::new(pow_hint(a, b).into()))
}

/// `float(s)`; `*status` is 1 if `s` is not a float.
///
/// # Safety
///
/// `s` is a live string and `status` points to a writable byte.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_float_parse(s: *const PtStr, status: *mut u8) -> f64 {
    // SAFETY: guaranteed by the caller.
    let parsed = parse_float(unsafe { crate::str::borrow(s) }.text());
    // SAFETY: guaranteed by the caller.
    unsafe { report(status, u8::from(parsed.is_none())) };
    parsed.unwrap_or(0.0)
}

/// `int(s)`; a [`ParseIntError`] code in `*status` on failure.
///
/// # Safety
///
/// `s` is a live string and `status` points to a writable byte.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_int_parse(s: *const PtStr, status: *mut u8) -> i64 {
    // SAFETY: guaranteed by the caller.
    let (r, code) = match parse_int(unsafe { crate::str::borrow(s) }.text()) {
        Ok(r) => (r, 0),
        Err(e) => (0, e as u8),
    };
    // SAFETY: guaranteed by the caller.
    unsafe { report(status, code) };
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repr_matches_python() {
        let cases = [
            (1.0, "1.0"),
            (0.1, "0.1"),
            (-0.0, "-0.0"),
            (0.30000000000000004, "0.30000000000000004"),
            (123456789.0, "123456789.0"),
            (1e15, "1000000000000000.0"),
            (1e16, "1e+16"),
            (1.5e300, "1.5e+300"),
            (0.0001, "0.0001"),
            (0.00001, "1e-05"),
            (-1.25e-7, "-1.25e-07"),
            (f64::INFINITY, "inf"),
            (f64::NEG_INFINITY, "-inf"),
            (f64::NAN, "nan"),
        ];
        for (f, want) in cases {
            assert_eq!(repr(f), want, "repr of {f:?}");
        }
    }

    #[test]
    fn floor_division_and_modulo_floor() {
        assert_eq!(divmod(7.0, 2.0), (3.0, 1.0));
        assert_eq!(divmod(-7.0, 2.0), (-4.0, 1.0));
        assert_eq!(divmod(7.0, -2.0), (-4.0, -1.0));
        let (q, m) = divmod(-0.0, 5.0);
        assert!(q == 0.0 && q.is_sign_negative() && m == 0.0 && m.is_sign_positive());
    }

    #[test]
    fn float_pow_fails_as_python_does() {
        assert_eq!(float_pow(2.0, 0.5), Ok(2.0_f64.sqrt()));
        assert_eq!(float_pow(0.0, -1.0), Err(PowError::ZeroToNegative));
        assert_eq!(float_pow(-8.0, 0.5), Err(PowError::Fractional));
        assert_eq!(float_pow(10.0, 400.0), Err(PowError::Overflow));
        assert_eq!(float_pow(-2.0, 3.0), Ok(-8.0));
    }

    #[test]
    fn int_pow_is_checked() {
        assert_eq!(int_pow(2, 62), Ok(1 << 62));
        assert_eq!(int_pow(2, 63), Err(IntPowError::Overflow));
        assert_eq!(int_pow(-2, 63), Ok(i64::MIN));
        assert_eq!(int_pow(-1, i64::MAX), Ok(-1));
        assert_eq!(int_pow(2, -1), Err(IntPowError::NegativeExponent));
        assert_eq!(pow_hint(-2, -1), "(-2.0) ** -1");
        assert_eq!(pow_hint(2, -1), "2.0 ** -1");
    }

    #[test]
    fn parsing_matches_python() {
        assert_eq!(parse_float(" 2.25\n"), Some(2.25));
        assert_eq!(parse_float("1_000.5"), Some(1000.5));
        assert_eq!(parse_float("1__0"), None);
        assert_eq!(parse_float("x"), None);
        assert_eq!(parse_int(" -42 "), Ok(-42));
        assert_eq!(parse_int("1_000"), Ok(1000));
        assert_eq!(parse_int("4.5"), Err(ParseIntError::Invalid));
        assert_eq!(parse_int(""), Err(ParseIntError::Invalid));
        assert_eq!(
            parse_int("9223372036854775808"),
            Err(ParseIntError::Overflow)
        );
    }

    #[test]
    fn statuses_reach_the_caller() {
        let mut status = 9;
        // SAFETY: `status` is a live local.
        let r = unsafe { pt_int_pow(3, 4, &mut status) };
        assert_eq!((r, status), (81, 0));
        // SAFETY: as above.
        unsafe { pt_float_pow(0.0, -2.0, &mut status) };
        assert_eq!(status, PowError::ZeroToNegative as u8);
    }
}

//! A probe binding for `pythrust build --rust probe=...`.

/// `x` doubled.
pub fn double(x: i64) -> i64 {
    x * 2
}

/// `x` halved.
pub fn half(x: f64) -> f64 {
    x / 2.0
}

//! A probe binding for `pythrust build --rust probe=...`.

/// `x` doubled.
pub fn double(x: i64) -> i64 {
    x * 2
}

/// `x` halved.
pub fn half(x: f64) -> f64 {
    x / 2.0
}

/// Panics when `x` is positive; gives `x` otherwise.
pub fn boom(x: i64) -> i64 {
    if x > 0 {
        panic!("boom {x}");
    }
    x
}

/// Prints a line from Rust, then gives `x`.
pub fn say(x: i64) -> i64 {
    println!("rust says {x}");
    x
}

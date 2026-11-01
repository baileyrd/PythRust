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

/// Not `b`.
pub fn negate(b: bool) -> bool {
    !b
}

/// A greeting for `name`.
pub fn greet(name: &str) -> String {
    format!("hello, {name}")
}

/// `text`'s UTF-8 bytes.
pub fn encode(text: &str) -> Vec<u8> {
    text.as_bytes().to_vec()
}

/// `data` reversed.
pub fn reversed(data: &[u8]) -> Vec<u8> {
    data.iter().rev().copied().collect()
}

/// Prints a line from Rust; gives nothing.
pub fn note(x: i64) {
    println!("note {x}");
}

/// The size of the file at `path`.
pub fn size(path: &str) -> std::io::Result<i64> {
    let len = std::fs::metadata(path)?.len();
    Ok(i64::try_from(len).unwrap_or(i64::MAX))
}

/// Always fails, with `why` as the message and no errno.
pub fn refuse(why: &str) -> std::io::Result<String> {
    Err(std::io::Error::other(why.to_string()))
}

/// `text`'s length in bytes, read by a scoped thread from the borrowed
/// argument.
pub fn scoped_len(text: &str) -> i64 {
    let n = std::thread::scope(|s| s.spawn(|| text.len()).join());
    n.map_or(-1, |n| i64::try_from(n).unwrap_or(i64::MAX))
}

/// Whether a thread this function spawns panicked: its message still
/// prints, since the panic is not inside the call's own thread.
pub fn worker_panics() -> bool {
    std::thread::spawn(|| panic!("worker gave up")).join().is_err()
}

/// Takes a string it could keep past the call: no stub over it builds.
pub fn keep(text: &'static str) -> i64 {
    i64::try_from(text.len()).unwrap_or(i64::MAX)
}

/// Takes bytes it could keep past the call: no stub over it builds.
pub fn keep_bytes(data: &'static [u8]) -> i64 {
    i64::try_from(data.len()).unwrap_or(i64::MAX)
}

/// An `unsafe fn`: no stub over it builds, since the shim may not call
/// it without `unsafe`.
///
/// # Safety
///
/// None needed; it only gives `x`.
pub unsafe fn danger(x: i64) -> i64 {
    x
}

//! The `sys` module (M6a): `sys.argv`.

#![allow(unsafe_code)] // The C ABI below.

use crate::seq::{new_seq, PtSeq};
use crate::str::{into_ptr, PtStr};

/// `sys.argv`: a new `list[str]` of the program's arguments, the
/// program first. An argument that is not UTF-8 is decoded lossily (a
/// listed deviation: CPython keeps its bytes as surrogate escapes).
///
/// # Safety
///
/// `desc` points to `len` bytes that live for the whole program: the
/// compiler's descriptor of `list[str]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_argv(desc: *const u8, len: usize) -> *mut PtSeq {
    // SAFETY: guaranteed by the caller.
    let desc: &'static [u8] = unsafe { std::slice::from_raw_parts(desc, len) };
    let items = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned().into_boxed_str())
        .map(|a| into_ptr(PtStr::new(a)) as u64)
        .collect();
    new_seq(desc, items)
}

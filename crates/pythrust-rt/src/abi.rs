//! What the generated Rust binding shims call (M6b,
//! `docs/research/rust-interop.md#the-shim`): conversions in (`arg_*`,
//! [`site`]), results out ([`IntoValue`]), and the two calls that run a
//! binding's function, [`call`] and [`call_raising`]. Part of the
//! audited C ABI surface: `arg_*` and [`site`] read raw pointers.

use std::any::Any;
use std::cell::Cell;
use std::io;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Once;

pub use crate::os::Site;

/// A Pythrust `str`, as a shim receives and returns it.
pub type Str = crate::str::PtStr;

/// A Pythrust `bytes`, as a shim receives and returns it.
pub type Bytes = crate::obj::PtObj;

/// A Pythrust `list[str]`, as a shim receives and returns it.
pub type List = crate::seq::PtSeq;

/// The runtime descriptor of `list[str]`, as the compiler writes it
/// (`codegen::seq::descriptor`), for a list a binding returns.
const LIST_STR: &[u8] = b"ls";

/// The site a shim's call raises or faults at.
///
/// # Safety
///
/// `head` and `tail` point to `head_len` and `tail_len` bytes of the
/// compiler's read-only data, which live for the whole program.
#[allow(unsafe_code)]
pub unsafe fn site(head: *const u8, head_len: usize, tail: *const u8, tail_len: usize) -> Site {
    // SAFETY: guaranteed by the caller.
    unsafe { crate::os::site(head, head_len, tail, tail_len) }
}

/// A `str` argument's text, borrowed for as long as the shim's own
/// argument `p` lives (the elided lifetime ties the two), so a binding
/// cannot keep it past the call.
///
/// # Safety
///
/// `*p` is a live string.
#[allow(unsafe_code)]
pub unsafe fn arg_str(p: &*const Str) -> &str {
    // SAFETY: guaranteed by the caller; the borrow is tied to `p`.
    unsafe { crate::str::borrow(*p) }.text()
}

/// A `bytes` argument's data, borrowed as for [`arg_str`].
///
/// # Safety
///
/// `*p` is a live bytes object.
#[allow(unsafe_code)]
pub unsafe fn arg_bytes(p: &*const Bytes) -> &[u8] {
    // SAFETY: guaranteed by the caller; the borrow is tied to `p`.
    let obj = unsafe { crate::obj::borrow(*p) };
    obj.bytes()
        .unwrap_or_else(|| crate::internal("a Rust call was passed a file as bytes"))
}

/// A `list[str]` argument's strings, copied under the list's read
/// borrow, so the binding gets plain Rust data that nothing else can
/// change or free during the call; it borrows them as `&[String]`.
///
/// # Safety
///
/// `*p` is a live `list[str]`.
#[allow(unsafe_code)]
pub unsafe fn arg_str_list(p: &*const List) -> Vec<String> {
    // SAFETY: guaranteed by the caller.
    let list = unsafe { crate::seq::seq(*p as u64) };
    let items = list.items.borrow();
    items
        .iter()
        // SAFETY: a `list[str]` holds live strings, retained by the list
        // while its read borrow is held.
        .map(|v| {
            unsafe { crate::str::borrow(*v as *const Str) }
                .text()
                .to_string()
        })
        .collect()
}

/// A `bool` argument, which crosses the C ABI as a byte: any nonzero
/// byte is true, as for `pt_write_bool`.
pub fn arg_bool(b: u8) -> bool {
    b != 0
}

/// A Rust result a shim returns to generated code: its C form, and the
/// placeholder returned while an exception is pending.
pub trait IntoValue {
    type Out;
    fn into_value(self) -> Self::Out;
    fn placeholder() -> Self::Out;
}

impl IntoValue for i64 {
    type Out = i64;
    fn into_value(self) -> i64 {
        self
    }
    fn placeholder() -> i64 {
        0
    }
}

impl IntoValue for f64 {
    type Out = f64;
    fn into_value(self) -> f64 {
        self
    }
    fn placeholder() -> f64 {
        0.0
    }
}

impl IntoValue for bool {
    type Out = u8;
    fn into_value(self) -> u8 {
        u8::from(self)
    }
    fn placeholder() -> u8 {
        0
    }
}

/// `None`, a byte as generated code holds it.
impl IntoValue for () {
    type Out = u8;
    fn into_value(self) -> u8 {
        0
    }
    fn placeholder() -> u8 {
        0
    }
}

/// A new string the caller owns.
impl IntoValue for String {
    type Out = *mut Str;
    fn into_value(self) -> *mut Str {
        crate::str::into_ptr(Str::new(self.into_boxed_str()))
    }
    fn placeholder() -> *mut Str {
        std::ptr::null_mut()
    }
}

/// A new `list[str]` the caller owns, taking each new string.
impl IntoValue for Vec<String> {
    type Out = *mut List;
    fn into_value(self) -> *mut List {
        let items = self
            .into_iter()
            .map(|t| crate::str::into_ptr(Str::new(t.into_boxed_str())) as u64)
            .collect();
        crate::seq::new_seq(LIST_STR, items)
    }
    fn placeholder() -> *mut List {
        std::ptr::null_mut()
    }
}

/// A new bytes object the caller owns.
impl IntoValue for Vec<u8> {
    type Out = *mut Bytes;
    fn into_value(self) -> *mut Bytes {
        crate::obj::new_bytes(self)
    }
    fn placeholder() -> *mut Bytes {
        std::ptr::null_mut()
    }
}

/// Runs a binding's function `f` that cannot fail and converts its
/// result. See [`guarded`] for output and panics.
pub fn call<T: IntoValue>(at: Site, f: impl FnOnce() -> T) -> T::Out {
    guarded(at, f).into_value()
}

/// Runs a binding's function `f` whose stub `raises`: an `Err` becomes
/// the `OSError` M6a maps it to (`os::os_error`), pending at `at`, and
/// the placeholder is returned for generated code to test the flag.
pub fn call_raising<T: IntoValue>(at: Site, f: impl FnOnce() -> io::Result<T>) -> T::Out {
    match guarded(at, f) {
        Ok(value) => value.into_value(),
        Err(e) => {
            at.raise(&e, None);
            T::placeholder()
        }
    }
}

thread_local! {
    /// Whether this thread is inside [`guarded`], whose panics Pythrust
    /// reports itself.
    static QUIET: Cell<bool> = const { Cell::new(false) };
}

static HOOK: Once = Once::new();

/// Wraps the panic hook once, so a panic inside [`guarded`] on the
/// calling thread prints nothing of Rust's own; any other panic, as in a
/// thread a binding spawns, prints as the previous hook would.
fn quiet_hook() {
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !QUIET.with(Cell::get) {
                previous(info);
            }
        }));
    });
}

/// Runs `f`. Flushes stdout first, so the binding's own output follows
/// what the program printed before the call. A panic must not unwind
/// into generated code (it would abort and lose the buffered output): it
/// ends the program as a fault at `at` does, its message in the
/// diagnostic. A panic is never catchable, like overflow.
fn guarded<R>(at: Site, f: impl FnOnce() -> R) -> R {
    crate::flush();
    quiet_hook();
    QUIET.with(|q| q.set(true));
    // `f` cannot leave anything half-changed that is seen again: on a
    // panic the program ends.
    let result = catch_unwind(AssertUnwindSafe(f));
    QUIET.with(|q| q.set(false));
    result.unwrap_or_else(|payload| at.fail(&panic_line(payload.as_ref())))
}

/// The diagnostic's message for a panic with `payload`.
fn panic_line(payload: &(dyn Any + Send)) -> String {
    let text = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str));
    match text {
        Some(t) => format!("Rust panic: {t}"),
        None => "Rust panic".to_string(),
    }
}

#[cfg(test)]
#[allow(unsafe_code)] // Values made and freed as generated code does.
mod tests {
    use super::*;
    use crate::exc::{pt_catch, pt_pending_is};

    #[test]
    fn call_converts_each_result() {
        assert_eq!(call(Site::BARE, || 21_i64 * 2), 42);
        assert_eq!(call(Site::BARE, || 1.5_f64), 1.5);
        assert_eq!(call(Site::BARE, || true), 1);
        assert_eq!(call(Site::BARE, || ()), 0);
        let s = call(Site::BARE, || "héllo".to_string());
        // SAFETY: `s` is the new string `call` made.
        let text = unsafe { crate::str::borrow(s) }.text().to_string();
        assert_eq!(text, "héllo");
        // SAFETY: the test owns `s`'s one reference.
        unsafe { crate::str::pt_str_dec(s) };
        let b = call(Site::BARE, || vec![0_u8, 255]);
        // SAFETY: as above, for bytes.
        let data = unsafe { crate::obj::borrow(b) }.bytes().map(<[u8]>::to_vec);
        assert_eq!(data, Some(vec![0, 255]));
        // SAFETY: the test owns `b`'s one reference.
        unsafe { crate::obj::pt_obj_dec(b) };
    }

    #[test]
    fn a_str_list_round_trips() {
        let made = vec!["a".to_string(), "héllo".to_string(), String::new()];
        let list = call(Site::BARE, || made.clone());
        let p: *const List = list;
        // SAFETY: `list` is the live `list[str]` `call` made.
        assert_eq!(unsafe { arg_str_list(&p) }, made);
        // SAFETY: the test owns `list`'s one reference.
        unsafe { crate::seq::pt_seq_dec(list) };
        let empty = call(Site::BARE, Vec::<String>::new);
        let q: *const List = empty;
        // SAFETY: as above.
        assert!(unsafe { arg_str_list(&q) }.is_empty());
        // SAFETY: as above.
        unsafe { crate::seq::pt_seq_dec(empty) };
    }

    #[test]
    fn arguments_borrow_the_runtimes_values() {
        let s = crate::str::into_ptr(Str::new("abc".into()));
        let p: *const Str = s;
        // SAFETY: `s` is live until the dec below.
        assert_eq!(unsafe { arg_str(&p) }, "abc");
        // SAFETY: the test owns `s`'s one reference.
        unsafe { crate::str::pt_str_dec(s) };
        let b = crate::obj::new_bytes(b"xy".to_vec());
        let q: *const Bytes = b;
        // SAFETY: `b` is live until the dec below.
        assert_eq!(unsafe { arg_bytes(&q) }, b"xy");
        // SAFETY: the test owns `b`'s one reference.
        unsafe { crate::obj::pt_obj_dec(b) };
        assert!(arg_bool(2) && !arg_bool(0));
    }

    #[test]
    fn an_err_raises_the_mapped_os_error() {
        let missing = || -> io::Result<String> { Err(io::ErrorKind::NotFound.into()) };
        assert!(call_raising(Site::BARE, missing).is_null());
        assert_eq!(pt_pending_is(1 << crate::exc::FILE_NOT_FOUND), 1);
        let exc = pt_catch();
        // SAFETY: `pt_catch` gave the pending exception's reference.
        unsafe { crate::seq::pt_seq_dec(exc) };
        assert_eq!(call_raising(Site::BARE, || Ok(7_i64)), 7);
        assert_eq!(pt_pending_is(u32::MAX), 0);
    }

    #[test]
    fn a_panic_reads_as_its_message() {
        assert_eq!(panic_line(&"boom"), "Rust panic: boom");
        assert_eq!(panic_line(&"boom 2".to_string()), "Rust panic: boom 2");
        assert_eq!(panic_line(&42_i32), "Rust panic");
    }
}

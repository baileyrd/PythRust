//! Rust binding interop (M6b, `docs/research/rust-interop.md`): the pure
//! parts of the generated crate `build` hands to cargo. [`gen`] writes
//! its manifest and shims; [`lock`] reads `pythrust.lock` and [`runtime`]
//! holds the shipped runtime source (M6d). Running
//! cargo and linking stay in the driver.

pub mod gen;
pub mod lock;
pub mod runtime;

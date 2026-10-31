//! Pythrust compiler.
//!
//! Contract types shared by every stage: [`span`], [`diag`], [`token`],
//! [`ast`], [`exc`] (the builtin exception classes), [`stdlib`] (the
//! importable modules), and [`ir`] for the backend. Stages: [`lexer`], [`parser`],
//! [`infer`], [`interp`]; and for native code [`lower`], [`rc`] (refcount
//! elision), [`codegen`].
//! [`interop`] holds the pure parts of Rust binding builds (M6b).
//! [`own`] summarizes how functions use their parameters, on the IR.
//! See `docs/grammar.md` for the language these types describe.

#![forbid(unsafe_code)]

pub mod ast;
pub mod diag;
pub mod exc;
pub mod ir;
pub mod span;
pub mod stdlib;
pub mod token;

pub mod codegen;
pub mod infer;
pub mod interop;
pub mod interp;
pub mod lexer;
pub mod lower;
pub mod own;
pub mod parser;
pub mod rc;

pub use diag::{DiagKind, Diagnostic, Result, Warning};
pub use span::Span;
pub use token::{Token, TokenKind};

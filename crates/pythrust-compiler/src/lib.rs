//! Pythrust compiler.
//!
//! Contract types shared by every stage: [`span`], [`diag`], [`token`],
//! [`ast`], and [`ir`] for the backend. Stages: [`lexer`], [`parser`],
//! [`infer`], [`interp`]; and for native code [`lower`], [`rc`] (refcount
//! elision), [`codegen`].
//! [`own`] summarizes how functions use their parameters, on the IR.
//! See `docs/grammar.md` for the language these types describe.

#![forbid(unsafe_code)]

pub mod ast;
pub mod diag;
pub mod ir;
pub mod span;
pub mod token;

pub mod codegen;
pub mod infer;
pub mod interp;
pub mod lexer;
pub mod lower;
pub mod own;
pub mod parser;
pub mod rc;

pub use diag::{DiagKind, Diagnostic, Result};
pub use span::Span;
pub use token::{Token, TokenKind};

//! Pythrust compiler.
//!
//! Contract types shared by every stage: [`span`], [`diag`], [`token`],
//! [`ast`]. Stages: [`lexer`], [`parser`], [`infer`], [`interp`].
//! See `docs/grammar.md` for the language these types describe.

#![forbid(unsafe_code)]

pub mod ast;
pub mod diag;
pub mod span;
pub mod token;

pub mod infer;
pub mod interp;
pub mod lexer;
pub mod parser;

pub use diag::{DiagKind, Diagnostic, Result};
pub use span::Span;
pub use token::{Token, TokenKind};

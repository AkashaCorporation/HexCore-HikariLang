#![allow(clippy::module_inception)]

pub mod lexer;
pub mod tokens;

pub use lexer::tokenize;
pub use tokens::{Keyword, Token};

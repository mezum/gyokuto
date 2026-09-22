//! 構文解析。仕様は `docs/spec/syntax.md` を参照。

pub mod ast;
mod error;
mod literal;
mod parser;
mod types;

pub use error::{Error, ErrorKind, Expected, Found, LiteralError};
pub use parser::parse_expr;
pub use types::parse_type;

//! 構文解析。仕様は `docs/spec/syntax.md` を参照。

pub mod ast;
mod parser;

pub use parser::{Error, parse_expr};

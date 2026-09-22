//! 字句解析。仕様は `docs/spec/lexical.md` を参照。

mod error;
mod quoted;
mod token;

pub use error::LexError;
pub use token::Token;

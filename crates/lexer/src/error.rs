use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Error)]
pub enum LexError {
    #[default]
    #[error("unexpected character")]
    UnexpectedChar,

    #[error("invalid digit for the base of the number literal")]
    InvalidDigit,
    #[error("missing digits after the base prefix")]
    MissingDigits,
    #[error("missing digits in the exponent")]
    MissingExponentDigits,
    #[error("invalid suffix on the number literal")]
    InvalidNumberSuffix,

    #[error("invalid character literal")]
    InvalidCharLiteral,
    #[error("invalid string literal")]
    InvalidStringLiteral,
    #[error("invalid byte literal")]
    InvalidByteLiteral,
    #[error("invalid byte string literal")]
    InvalidByteStringLiteral,
    #[error("invalid unicode escape")]
    InvalidUnicodeEscape,

    #[error("unterminated raw string literal")]
    UnterminatedRawString,
    #[error("too many `#` in the raw string literal (at most 255)")]
    TooManyRawStringHashes,
    #[error("bare carriage return in the literal")]
    BareCarriageReturn,
    #[error("literal suffixes are reserved")]
    ReservedLiteralSuffix,

    #[error("unterminated block comment")]
    UnterminatedBlockComment,
}

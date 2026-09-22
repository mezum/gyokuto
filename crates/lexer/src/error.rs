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

    #[error("unterminated character literal")]
    UnterminatedCharLiteral,
    #[error("unterminated string literal")]
    UnterminatedStringLiteral,
    #[error("empty character literal")]
    EmptyCharLiteral,
    #[error("character literal may only contain one character")]
    TooManyCharsInCharLiteral,
    #[error("character must be escaped in the character literal")]
    UnescapedCharInCharLiteral,
    #[error("invalid escape sequence")]
    InvalidEscape,
    #[error("invalid unicode escape")]
    InvalidUnicodeEscape,
    #[error("unicode escape in the byte literal")]
    UnicodeEscapeInByteLiteral,
    #[error("non-ASCII character in the byte literal")]
    NonAsciiInByteLiteral,

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

use crate::error::LexError;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Quoted {
    Char,
    Byte,
    Str,
    ByteStr,
}

impl Quoted {
    fn is_single(self) -> bool {
        matches!(self, Self::Char | Self::Byte)
    }

    fn is_byte(self) -> bool {
        matches!(self, Self::Byte | Self::ByteStr)
    }

    fn quote(self) -> char {
        if self.is_single() { '\'' } else { '"' }
    }

    fn unterminated(self) -> LexError {
        if self.is_single() {
            LexError::UnterminatedCharLiteral
        } else {
            LexError::UnterminatedStringLiteral
        }
    }
}

/// 文字・文字列リテラルとして読めなかった並びから、最初に見つかったエラーの原因を判別する
pub(crate) fn invalid_quoted(text: &str, kind: Quoted) -> LexError {
    let prefix = if kind.is_byte() { 2 } else { 1 };
    let mut chars = text[prefix..].chars().peekable();
    let mut units = 0;
    loop {
        let Some(c) = chars.next() else {
            return kind.unterminated();
        };
        let error = match c {
            '\'' if kind.is_single() && units == 0 && chars.peek() == Some(&'\'') => {
                Some(LexError::UnescapedCharInCharLiteral)
            }
            c if c == kind.quote() => break,
            '\\' => escape_error(&mut chars, kind),
            '\t' | '\r' if kind.is_single() => Some(LexError::UnescapedCharInCharLiteral),
            '\r' if chars.peek() != Some(&'\n') => Some(LexError::BareCarriageReturn),
            c if kind == Quoted::Byte && !c.is_ascii() => Some(LexError::NonAsciiInByteLiteral),
            _ => None,
        };
        if let Some(error) = error {
            return error;
        }
        units += 1;
    }
    match units {
        0 if kind.is_single() => LexError::EmptyCharLiteral,
        2.. if kind.is_single() => LexError::TooManyCharsInCharLiteral,
        _ if chars.next().is_some() => LexError::ReservedLiteralSuffix,
        _ => LexError::UnexpectedChar,
    }
}

fn escape_error(chars: &mut Peekable<Chars>, kind: Quoted) -> Option<LexError> {
    match chars.next() {
        None => Some(kind.unterminated()),
        Some('n' | 'r' | 't' | '\\' | '0' | '\'' | '"') => None,
        Some('x') => {
            let hex: String = chars.by_ref().take(2).collect();
            let valid = hex.len() == 2
                && hex.chars().all(|c| c.is_ascii_hexdigit())
                && u8::from_str_radix(&hex, 16).is_ok_and(|v| kind.is_byte() || v <= 0x7F);
            (!valid).then_some(LexError::InvalidEscape)
        }
        Some('u') if kind == Quoted::Byte => Some(LexError::UnicodeEscapeInByteLiteral),
        Some('u') => {
            let digits: String = if chars.next() == Some('{') {
                chars.by_ref().take_while(|&c| c != '}').collect()
            } else {
                String::new()
            };
            let digits: String = digits.chars().filter(|&c| c != '_').collect();
            let valid = (1..=6).contains(&digits.len())
                && digits.chars().all(|c| c.is_ascii_hexdigit())
                && u32::from_str_radix(&digits, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .is_some();
            (!valid).then_some(LexError::InvalidUnicodeEscape)
        }
        Some('\n') if !kind.is_single() => None,
        Some('\r') if !kind.is_single() && chars.next_if_eq(&'\n').is_some() => None,
        _ => Some(LexError::InvalidEscape),
    }
}

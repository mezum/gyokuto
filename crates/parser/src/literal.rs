use crate::ast::{FloatSuffix, IntSuffix, Lit};
use crate::error::LiteralError;
use gyokuto_lexer::Token;

const INT_SUFFIXES: [(&str, IntSuffix); 10] = [
    ("i8", IntSuffix::I8),
    ("i16", IntSuffix::I16),
    ("i32", IntSuffix::I32),
    ("i64", IntSuffix::I64),
    ("isize", IntSuffix::Isize),
    ("u8", IntSuffix::U8),
    ("u16", IntSuffix::U16),
    ("u32", IntSuffix::U32),
    ("u64", IntSuffix::U64),
    ("usize", IntSuffix::Usize),
];

const FLOAT_SUFFIXES: [(&str, FloatSuffix); 2] =
    [("f32", FloatSuffix::F32), ("f64", FloatSuffix::F64)];

/// リテラルのトークンを値に変換する
pub(crate) fn decode(token: Token, text: &str) -> Result<Lit, LiteralError> {
    let quoted = |prefix: usize| {
        text.get(prefix + 1..text.len().saturating_sub(1))
            .ok_or(LiteralError::Malformed)
    };
    let utf8 = |bytes| String::from_utf8(bytes).map_err(|_| LiteralError::InvalidUtf8);
    Ok(match token {
        Token::Int => decode_int(text)?,
        Token::Float => {
            let (digits, suffix) = split_suffix(text, &FLOAT_SUFFIXES);
            Lit::Float {
                digits: digits.replace('_', ""),
                suffix,
            }
        }
        Token::Char => {
            let chars: Vec<char> = utf8(unescape(quoted(0)?)?)?.chars().collect();
            match chars[..] {
                [c] => Lit::Char(c),
                _ => return Err(LiteralError::Malformed),
            }
        }
        Token::Byte => match unescape(quoted(1)?)?[..] {
            [b] => Lit::Byte(b),
            _ => return Err(LiteralError::Malformed),
        },
        Token::Str => Lit::Str(utf8(unescape(quoted(0)?)?)?),
        Token::ByteStr => Lit::ByteStr(unescape(quoted(1)?)?),
        Token::RawStr => Lit::Str(raw_body(text.get(1..).ok_or(LiteralError::Malformed)?)?),
        Token::RawByteStr => {
            Lit::ByteStr(raw_body(text.get(2..).ok_or(LiteralError::Malformed)?)?.into_bytes())
        }
        _ => return Err(LiteralError::NotALiteral),
    })
}

fn decode_int(text: &str) -> Result<Lit, LiteralError> {
    let (digits, suffix) = split_suffix(text, &INT_SUFFIXES);
    let (radix, digits) = [("0x", 16), ("0o", 8), ("0b", 2)]
        .into_iter()
        .find_map(|(prefix, radix)| digits.strip_prefix(prefix).map(|d| (radix, d)))
        .unwrap_or((10, digits));
    let value = u64::from_str_radix(&digits.replace('_', ""), radix)
        .map_err(|_| LiteralError::IntegerTooLarge)?;
    Ok(Lit::Int { value, suffix })
}

fn split_suffix<'a, S: Copy>(text: &'a str, suffixes: &[(&str, S)]) -> (&'a str, Option<S>) {
    suffixes
        .iter()
        .find_map(|&(name, suffix)| text.strip_suffix(name).map(|rest| (rest, Some(suffix))))
        .unwrap_or((text, None))
}

/// `r` / `br` の後の `#...#"` と `"#...#` を除き、CRLF を正規化した中身を得る
fn raw_body(text: &str) -> Result<String, LiteralError> {
    let hashes = text.len() - text.trim_start_matches('#').len();
    let body = text
        .get(hashes + 1..text.len().saturating_sub(hashes + 1))
        .ok_or(LiteralError::Malformed)?;
    Ok(body.replace("\r\n", "\n"))
}

/// エスケープ・行継続・CRLF を処理したバイト列を得る
fn unescape(body: &str) -> Result<Vec<u8>, LiteralError> {
    let body = body.replace("\r\n", "\n");
    let mut bytes = Vec::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            push_char(&mut bytes, c);
            continue;
        }
        match chars.next() {
            Some('n') => bytes.push(b'\n'),
            Some('r') => bytes.push(b'\r'),
            Some('t') => bytes.push(b'\t'),
            Some('0') => bytes.push(0),
            Some(c @ ('\\' | '\'' | '"')) => bytes.push(c as u8),
            Some('x') => {
                let hex: String = chars.by_ref().take(2).collect();
                bytes.push(u8::from_str_radix(&hex, 16).map_err(|_| LiteralError::InvalidEscape)?);
            }
            Some('u') => {
                let hex: String = chars
                    .by_ref()
                    .take_while(|&c| c != '}')
                    .filter(|&c| c != '{' && c != '_')
                    .collect();
                let c = u32::from_str_radix(&hex, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or(LiteralError::InvalidEscape)?;
                push_char(&mut bytes, c);
            }
            Some('\n') => {
                chars = chars
                    .as_str()
                    .trim_start_matches(is_pattern_white_space)
                    .chars()
            }
            _ => return Err(LiteralError::InvalidEscape),
        }
    }
    Ok(bytes)
}

fn push_char(bytes: &mut Vec<u8>, c: char) {
    bytes.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
}

fn is_pattern_white_space(c: char) -> bool {
    matches!(
        c,
        '\t'..='\r' | ' ' | '\u{85}' | '\u{200E}' | '\u{200F}' | '\u{2028}' | '\u{2029}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{FloatSuffix, IntSuffix};
    use logos::Logos;

    fn lit(src: &str) -> Result<Lit, LiteralError> {
        let (token, span) = Token::lexer(src).spanned().next().unwrap();
        decode(token.unwrap(), &src[span])
    }

    fn int(value: u64, suffix: Option<IntSuffix>) -> Result<Lit, LiteralError> {
        Ok(Lit::Int { value, suffix })
    }

    fn float(digits: &str, suffix: Option<FloatSuffix>) -> Result<Lit, LiteralError> {
        Ok(Lit::Float {
            digits: digits.to_string(),
            suffix,
        })
    }

    fn str(value: &str) -> Result<Lit, LiteralError> {
        Ok(Lit::Str(value.to_string()))
    }

    #[test]
    fn integers() {
        assert_eq!(lit("0"), int(0, None));
        assert_eq!(lit("1_000"), int(1000, None));
        assert_eq!(lit("0xff"), int(255, None));
        assert_eq!(lit("0o17"), int(15, None));
        assert_eq!(lit("0b1010"), int(10, None));
        assert_eq!(lit("255u8"), int(255, Some(IntSuffix::U8)));
        assert_eq!(lit("0xffusize"), int(255, Some(IntSuffix::Usize)));
        assert_eq!(lit("1i64"), int(1, Some(IntSuffix::I64)));
        assert_eq!(lit("18446744073709551615"), int(u64::MAX, None));
        assert_eq!(
            lit("18446744073709551616"),
            Err(LiteralError::IntegerTooLarge)
        );
    }

    #[test]
    fn floats() {
        assert_eq!(lit("1_000.5"), float("1000.5", None));
        assert_eq!(lit("2.5E-3f32"), float("2.5E-3", Some(FloatSuffix::F32)));
        assert_eq!(lit("1f64"), float("1", Some(FloatSuffix::F64)));
    }

    #[test]
    fn chars_and_bytes() {
        assert_eq!(lit("'a'"), Ok(Lit::Char('a')));
        assert_eq!(lit(r"'\n'"), Ok(Lit::Char('\n')));
        assert_eq!(lit(r"'\''"), Ok(Lit::Char('\'')));
        assert_eq!(lit(r"'\x41'"), Ok(Lit::Char('A')));
        assert_eq!(lit(r"'\u{1F600}'"), Ok(Lit::Char('😀')));
        assert_eq!(lit("b'a'"), Ok(Lit::Byte(b'a')));
        assert_eq!(lit(r"b'\xFF'"), Ok(Lit::Byte(0xFF)));
    }

    #[test]
    fn ligatures_and_rtl_chars() {
        assert_eq!(lit("'ﬁ'"), Ok(Lit::Char('ﬁ')));
        assert_eq!(lit("'ﷺ'"), Ok(Lit::Char('ﷺ')));
        assert_eq!(lit("'م'"), Ok(Lit::Char('م')));
        assert_eq!(lit("'א'"), Ok(Lit::Char('א')));
        assert_eq!(lit("'\u{200F}'"), Ok(Lit::Char('\u{200F}')));
        assert_eq!(lit(r"'\u{202E}'"), Ok(Lit::Char('\u{202E}')));
    }

    #[test]
    fn ligatures_and_rtl_strings() {
        for text in [
            "ﬁle ﬂow",
            "مرحبا بالعالم",
            "שלום עולם",
            "abc مرحبا 123",
            "a\u{200F}b\u{200E}c",
            "\u{202E}abc\u{202C}",
            "e\u{301}",
            "👨\u{200D}👩\u{200D}👧",
            "🇯🇵",
        ] {
            assert_eq!(lit(&format!("\"{text}\"")), str(text), "{text:?}");
            assert_eq!(lit(&format!("r\"{text}\"")), str(text), "{text:?}");
            let expected = Ok(Lit::ByteStr(text.as_bytes().to_vec()));
            assert_eq!(lit(&format!("b\"{text}\"")), expected, "{text:?}");
        }
    }

    #[test]
    fn line_continuation_skips_directional_marks() {
        assert_eq!(lit("\"a\\\n\u{200F}\u{200E} b\""), str("ab"));
        assert_eq!(lit("\"a\\\n\u{202E}b\""), str("a\u{202E}b"));
    }

    #[test]
    fn strings() {
        assert_eq!(lit(r#""a\tb""#), str("a\tb"));
        assert_eq!(lit(r#""\u{3042}""#), str("あ"));
        assert_eq!(lit(r#""\\u{41}""#), str(r"\u{41}"));
        assert_eq!(lit("\"a\r\nb\""), str("a\nb"));
        assert_eq!(lit("\"a\\\n    b\""), str("ab"));
        assert_eq!(lit("\"a\\\r\n    b\""), str("ab"));
    }

    #[test]
    fn byte_strings() {
        let expected = [&[0xFF][..], "ああ".as_bytes()].concat();
        assert_eq!(lit(r#"b"\xFF\u{3042}あ""#), Ok(Lit::ByteStr(expected)));
    }

    #[test]
    fn malformed_input_is_error_instead_of_panic() {
        for (token, text, error) in [
            (Token::Str, r#""\x80""#, LiteralError::InvalidUtf8),
            (Token::Str, r#""\q""#, LiteralError::InvalidEscape),
            (Token::Str, r#""\xZZ""#, LiteralError::InvalidEscape),
            (Token::Str, r#""\u{D800}""#, LiteralError::InvalidEscape),
            (Token::Str, r#"""#, LiteralError::Malformed),
            (Token::Char, "''", LiteralError::Malformed),
            (Token::Char, "'ab'", LiteralError::Malformed),
            (Token::Byte, "b''", LiteralError::Malformed),
            (Token::RawStr, "r#", LiteralError::Malformed),
            (Token::Ident, "a", LiteralError::NotALiteral),
        ] {
            assert_eq!(decode(token, text), Err(error), "{token:?} {text}");
        }
    }

    #[test]
    fn raw_strings() {
        assert_eq!(lit(r###"r#"a\n"b"#"###), str(r#"a\n"b"#));
        assert_eq!(lit("r\"a\r\nb\""), str("a\nb"));
        assert_eq!(lit(r#"br"\x""#), Ok(Lit::ByteStr(br"\x".to_vec())));
    }
}

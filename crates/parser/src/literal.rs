use crate::ast::{FloatSuffix, IntSuffix, Lit};
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

/// 字句解析で検証済みのリテラルのトークンを値に変換する
pub(crate) fn decode(token: Token, text: &str) -> Result<Lit, &'static str> {
    let quoted = |prefix: usize| &text[prefix + 1..text.len() - 1];
    let utf8 = |bytes| String::from_utf8(bytes).expect("文字・文字列リテラルは UTF-8 になる");
    Ok(match token {
        Token::Int => decode_int(text)?,
        Token::Float => {
            let (digits, suffix) = split_suffix(text, &FLOAT_SUFFIXES);
            Lit::Float {
                digits: digits.replace('_', ""),
                suffix,
            }
        }
        Token::Char => Lit::Char(utf8(unescape(quoted(0))).chars().next().unwrap()),
        Token::Byte => Lit::Byte(unescape(quoted(1))[0]),
        Token::Str => Lit::Str(utf8(unescape(quoted(0)))),
        Token::ByteStr => Lit::ByteStr(unescape(quoted(1))),
        Token::RawStr => Lit::Str(raw_body(&text[1..])),
        Token::RawByteStr => Lit::ByteStr(raw_body(&text[2..]).into_bytes()),
        _ => unreachable!("リテラルではないトークン: {token:?}"),
    })
}

fn decode_int(text: &str) -> Result<Lit, &'static str> {
    let (digits, suffix) = split_suffix(text, &INT_SUFFIXES);
    let (radix, digits) = [("0x", 16), ("0o", 8), ("0b", 2)]
        .into_iter()
        .find_map(|(prefix, radix)| digits.strip_prefix(prefix).map(|d| (radix, d)))
        .unwrap_or((10, digits));
    let value = u64::from_str_radix(&digits.replace('_', ""), radix)
        .map_err(|_| "integer literal is too large")?;
    Ok(Lit::Int { value, suffix })
}

fn split_suffix<'a, S: Copy>(text: &'a str, suffixes: &[(&str, S)]) -> (&'a str, Option<S>) {
    suffixes
        .iter()
        .find_map(|&(name, suffix)| text.strip_suffix(name).map(|rest| (rest, Some(suffix))))
        .unwrap_or((text, None))
}

/// `r` / `br` の後の `#...#"` と `"#...#` を除き、CRLF を正規化した中身を得る
fn raw_body(text: &str) -> String {
    let hashes = text.len() - text.trim_start_matches('#').len();
    text[hashes + 1..text.len() - hashes - 1].replace("\r\n", "\n")
}

/// エスケープ・行継続・CRLF を処理したバイト列を得る
fn unescape(body: &str) -> Vec<u8> {
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
                bytes.push(u8::from_str_radix(&hex, 16).unwrap());
            }
            Some('u') => {
                let hex: String = chars
                    .by_ref()
                    .take_while(|&c| c != '}')
                    .filter(|&c| c != '{' && c != '_')
                    .collect();
                let value = u32::from_str_radix(&hex, 16).unwrap();
                push_char(&mut bytes, char::from_u32(value).unwrap());
            }
            Some('\n') => {
                chars = chars
                    .as_str()
                    .trim_start_matches(is_pattern_white_space)
                    .chars()
            }
            other => unreachable!("字句解析で検証済みのエスケープ: {other:?}"),
        }
    }
    bytes
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

    fn lit(src: &str) -> Result<Lit, &'static str> {
        let (token, span) = Token::lexer(src).spanned().next().unwrap();
        decode(token.unwrap(), &src[span])
    }

    fn int(value: u64, suffix: Option<IntSuffix>) -> Result<Lit, &'static str> {
        Ok(Lit::Int { value, suffix })
    }

    fn float(digits: &str, suffix: Option<FloatSuffix>) -> Result<Lit, &'static str> {
        Ok(Lit::Float {
            digits: digits.to_string(),
            suffix,
        })
    }

    fn str(value: &str) -> Result<Lit, &'static str> {
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
        assert!(lit("18446744073709551616").is_err());
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
    fn raw_strings() {
        assert_eq!(lit(r###"r#"a\n"b"#"###), str(r#"a\n"b"#));
        assert_eq!(lit("r\"a\r\nb\""), str("a\nb"));
        assert_eq!(lit(r#"br"\x""#), Ok(Lit::ByteStr(br"\x".to_vec())));
    }
}

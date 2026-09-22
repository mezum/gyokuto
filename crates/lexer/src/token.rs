use crate::error::LexError;
use crate::quoted::{Quoted, invalid_quoted};
use logos::{Lexer, Logos};

/// エラー用パターンの callback の戻り値の型
type Fail = Result<(), LexError>;

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(error = LexError)]
#[logos(skip r"\p{Pattern_White_Space}+")]
#[logos(skip(r"//[^\n]*", allow_greedy = true))]
#[logos(skip(r"/\*", callback = block_comment))]
#[logos(subpattern dec = r"[0-9][0-9_]*")]
#[logos(subpattern hex = r"0x[0-9a-fA-F_]*[0-9a-fA-F][0-9a-fA-F_]*")]
#[logos(subpattern oct = r"0o[0-7_]*[0-7][0-7_]*")]
#[logos(subpattern bin = r"0b[01_]*[01][01_]*")]
#[logos(subpattern exp = r"[eE][+-]?[0-9_]*[0-9][0-9_]*")]
#[logos(subpattern float_suffix = r"f32|f64")]
#[logos(subpattern esc = r#"\\([nrt\\0'"]|x[0-7][0-9a-fA-F]|u\{_*([0-9a-fA-F]_*){1,6}\})"#)]
#[logos(subpattern byte_esc = r#"\\([nrt\\0'"]|x[0-9a-fA-F]{2})"#)]
pub enum Token {
    /// 字句解析のエラー。構文解析の入力でエラーの位置を表すために使う
    Error,

    #[regex(r"[\p{XID_Start}_]\p{XID_Continue}*")]
    Ident,
    #[token("_", priority = 3)]
    Underscore,

    #[regex(r"((?&dec)|(?&hex)|(?&oct)|(?&bin))(i8|i16|i32|i64|isize|u8|u16|u32|u64|usize)?")]
    /// 数値リテラルの直後に無効なサフィックスが続く場合はエラーとする
    #[regex(
        r"((?&dec)(\.(?&dec))?(?&exp)?|(?&hex)|(?&oct)|(?&bin))\p{XID_Continue}+",
        invalid_number,
        priority = 0
    )]
    Int,
    #[regex(r"(?&dec)(\.(?&dec)(?&exp)?|(?&exp))(?&float_suffix)?")]
    #[regex(r"(?&dec)(?&float_suffix)")]
    Float,
    #[regex(r"'([^'\\\n\r\t]|(?&esc))'", unicode_escapes_are_valid)]
    /// 閉じていない・不正な内容・直後に識別子の文字が続く文字リテラルはエラーとする
    #[regex(
        r"'([^\\\n]|\\.)?([^'\\\n]|\\.)*'?\p{XID_Continue}*",
        |lex| Fail::Err(invalid_quoted(lex.slice(), Quoted::Char)),
        priority = 0
    )]
    Char,
    #[regex(r#""([^"\\\r]|\r\n|(?&esc)|\\\r?\n)*""#, unicode_escapes_are_valid)]
    /// 閉じていない・不正な内容・直後に識別子の文字が続く文字列リテラルはエラーとする
    #[regex(
        r#""([^"\\]|\\(.|\n))*("\p{XID_Continue}*)?"#,
        |lex| Fail::Err(invalid_quoted(lex.slice(), Quoted::Str)),
        priority = 0
    )]
    Str,
    #[regex(r"b'([\x00-\x7F&&[^'\\\n\r\t]]|(?&byte_esc))'")]
    /// 閉じていない・不正な内容・直後に識別子の文字が続くバイト文字リテラルはエラーとする
    #[regex(
        r"b'([^\\\n]|\\.)?([^'\\\n]|\\.)*'?\p{XID_Continue}*",
        |lex| Fail::Err(invalid_quoted(lex.slice(), Quoted::Byte)),
        priority = 0
    )]
    Byte,
    #[regex(
        r#"b"([^"\\\r]|\r\n|(?&byte_esc)|(?&esc)|\\\r?\n)*""#,
        unicode_escapes_are_valid
    )]
    /// 閉じていない・不正な内容・直後に識別子の文字が続くバイト文字列リテラルはエラーとする
    #[regex(
        r#"b"([^"\\]|\\(.|\n))*("\p{XID_Continue}*)?"#,
        |lex| Fail::Err(invalid_quoted(lex.slice(), Quoted::ByteStr)),
        priority = 0
    )]
    ByteStr,
    #[regex(r##"r#*""##, raw_string)]
    RawStr,
    #[regex(r##"br#*""##, raw_string)]
    RawByteStr,

    #[token("as")]
    As,
    #[token("break")]
    Break,
    #[token("comptime")]
    Comptime,
    #[token("const")]
    Const,
    #[token("continue")]
    Continue,
    #[token("crate")]
    Crate,
    #[token("dyn")]
    Dyn,
    #[token("else")]
    Else,
    #[token("enum")]
    Enum,
    #[token("false")]
    False,
    #[token("fn")]
    Fn,
    #[token("for")]
    For,
    #[token("if")]
    If,
    #[token("impl")]
    Impl,
    #[token("in")]
    In,
    #[token("let")]
    Let,
    #[token("loop")]
    Loop,
    #[token("match")]
    Match,
    #[token("move")]
    Move,
    #[token("mut")]
    Mut,
    #[token("pub")]
    Pub,
    #[token("return")]
    Return,
    #[token("self")]
    SelfValue,
    #[token("Self")]
    SelfType,
    #[token("struct")]
    Struct,
    #[token("super")]
    Super,
    #[token("trait")]
    Trait,
    #[token("true")]
    True,
    #[token("type")]
    Type,
    #[token("use")]
    Use,
    #[token("where")]
    Where,
    #[token("while")]
    While,

    /// 予約語。識別子としては使えない
    #[token("yield")]
    Yield,

    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("&")]
    Amp,
    #[token("|")]
    Pipe,
    #[token("^")]
    Caret,
    #[token("!")]
    Bang,
    #[token("<<")]
    Shl,
    #[token(">>")]
    Shr,
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("==")]
    EqEq,
    #[token("!=")]
    Ne,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,
    #[token("=")]
    Eq,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,
    #[token("%=")]
    PercentEq,
    #[token("&=")]
    AmpEq,
    #[token("|=")]
    PipeEq,
    #[token("^=")]
    CaretEq,
    #[token("<<=")]
    ShlEq,
    #[token(">>=")]
    ShrEq,
    #[token(".")]
    Dot,
    #[token("..")]
    DotDot,
    #[token("..=")]
    DotDotEq,
    #[token("::")]
    ColonColon,
    #[token(":")]
    Colon,
    #[token(";")]
    Semi,
    #[token(",")]
    Comma,
    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,
    #[token("?")]
    Question,
    #[token("#")]
    Pound,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
}

/// 数値リテラルとして読めなかった並びから、エラーの原因を判別する
fn invalid_number(lex: &mut Lexer<Token>) -> Result<(), LexError> {
    let text = lex.slice();
    let digits_of = |radix: u32| move |c: char| c == '_' || c.is_digit(radix);
    let prefixed = [("0x", 16), ("0o", 8), ("0b", 2)]
        .into_iter()
        .find_map(|(prefix, radix)| text.strip_prefix(prefix).map(|rest| (radix, rest)));
    Err(match prefixed {
        Some((radix, rest)) => {
            let after = rest.trim_start_matches(digits_of(radix));
            let has_digits = rest[..rest.len() - after.len()].contains(|c: char| c != '_');
            match (has_digits, after.starts_with(|c: char| c.is_ascii_digit())) {
                (_, true) => LexError::InvalidDigit,
                (false, false) => LexError::MissingDigits,
                (true, false) => LexError::InvalidNumberSuffix,
            }
        }
        None => {
            let after = text.trim_start_matches(digits_of(10));
            let after = after
                .strip_prefix('.')
                .map_or(after, |fraction| fraction.trim_start_matches(digits_of(10)));
            let exponent_has_digits = |exponent: &str| {
                exponent
                    .trim_start_matches(['+', '-'])
                    .trim_start_matches('_')
                    .starts_with(|c: char| c.is_ascii_digit())
            };
            match after.strip_prefix(['e', 'E']) {
                Some(exponent) if !exponent_has_digits(exponent) => LexError::MissingExponentDigits,
                _ => LexError::InvalidNumberSuffix,
            }
        }
    })
}

/// `\u{...}` の値が Unicode スカラー値であるか検査する
fn unicode_escapes_are_valid(lex: &mut Lexer<Token>) -> Result<(), LexError> {
    // 先に `\\` で分割し、エスケープされた `\` の後の `u{` を誤検出しないようにする
    lex.slice()
        .split(r"\\")
        .flat_map(|s| s.split(r"\u{").skip(1))
        .all(|s| {
            let hex: String = s
                .chars()
                .take_while(|&c| c != '}')
                .filter(|&c| c != '_')
                .collect();
            u32::from_str_radix(&hex, 16)
                .ok()
                .and_then(char::from_u32)
                .is_some()
        })
        .then_some(())
        .ok_or(LexError::InvalidUnicodeEscape)
}

/// raw 文字列を、開始と同じ数の `#` が `"` の後に続く終端まで読み進める
///
/// `#` の数をそろえる処理は正規表現で表現できないため、開始部分以降をここで扱う
fn raw_string(lex: &mut Lexer<Token>) -> Result<(), LexError> {
    let hashes = lex.slice().matches('#').count();
    let terminator = format!("\"{}", "#".repeat(hashes));
    let remainder = lex.remainder();
    let Some(end) = remainder.find(&terminator) else {
        lex.bump(remainder.len());
        return Err(LexError::UnterminatedRawString);
    };
    let has_bare_cr = remainder[..end].replace("\r\n", "").contains('\r');
    let suffix_len: usize = remainder[end + terminator.len()..]
        .chars()
        .take_while(|&c| unicode_ident::is_xid_continue(c))
        .map(char::len_utf8)
        .sum();
    lex.bump(end + terminator.len() + suffix_len);
    [
        (hashes > 255, LexError::TooManyRawStringHashes),
        (has_bare_cr, LexError::BareCarriageReturn),
        (suffix_len > 0, LexError::ReservedLiteralSuffix),
    ]
    .into_iter()
    .find_map(|(failed, error)| failed.then_some(error))
    .map_or(Ok(()), Err)
}

/// ブロックコメントを、ネストの深さを数えて対応する `*/` まで読み飛ばす
///
/// ネストの対応は正規表現で表現できないため、開始部分以降をここで扱う
fn block_comment(lex: &mut Lexer<Token>) -> Result<(), LexError> {
    let remainder = lex.remainder().as_bytes();
    let mut depth = 1;
    let mut i = 0;
    while depth > 0 {
        match remainder.get(i..i + 2) {
            Some(b"/*") => (depth, i) = (depth + 1, i + 2),
            Some(b"*/") => (depth, i) = (depth - 1, i + 2),
            Some([_, b'/' | b'*']) => i += 1,
            Some(_) => i += 2,
            None => {
                lex.bump(remainder.len());
                return Err(LexError::UnterminatedBlockComment);
            }
        }
    }
    lex.bump(i);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::ops::Range;

    fn lex(src: &str) -> Vec<(Result<Token, LexError>, Range<usize>)> {
        Token::lexer(src).spanned().collect()
    }

    #[test]
    fn skips_whitespace_and_tracks_spans() {
        assert_eq!(
            lex(" foo\t\r\nbar "),
            [(Ok(Token::Ident), 1..4), (Ok(Token::Ident), 7..10)]
        );
    }

    #[rstest]
    #[case("a")]
    #[case("a1")]
    #[case("_x")]
    #[case("__")]
    #[case("変数")]
    #[case("letter")]
    #[case("Selfie")]
    fn identifier(#[case] src: &str) {
        assert_eq!(lex(src), [(Ok(Token::Ident), 0..src.len())]);
    }

    #[test]
    fn underscore_alone_is_not_identifier() {
        assert_eq!(lex("_"), [(Ok(Token::Underscore), 0..1)]);
    }

    #[rstest]
    #[case("as", Token::As)]
    #[case("break", Token::Break)]
    #[case("comptime", Token::Comptime)]
    #[case("const", Token::Const)]
    #[case("continue", Token::Continue)]
    #[case("crate", Token::Crate)]
    #[case("dyn", Token::Dyn)]
    #[case("else", Token::Else)]
    #[case("enum", Token::Enum)]
    #[case("extern", Token::Extern)]
    #[case("false", Token::False)]
    #[case("fn", Token::Fn)]
    #[case("for", Token::For)]
    #[case("if", Token::If)]
    #[case("impl", Token::Impl)]
    #[case("in", Token::In)]
    #[case("let", Token::Let)]
    #[case("loop", Token::Loop)]
    #[case("match", Token::Match)]
    #[case("move", Token::Move)]
    #[case("mut", Token::Mut)]
    #[case("pub", Token::Pub)]
    #[case("return", Token::Return)]
    #[case("self", Token::SelfValue)]
    #[case("Self", Token::SelfType)]
    #[case("struct", Token::Struct)]
    #[case("super", Token::Super)]
    #[case("trait", Token::Trait)]
    #[case("true", Token::True)]
    #[case("type", Token::Type)]
    #[case("use", Token::Use)]
    #[case("where", Token::Where)]
    #[case("while", Token::While)]
    #[case("yield", Token::Yield)]
    fn keyword(#[case] src: &str, #[case] token: Token) {
        assert_eq!(lex(src), [(Ok(token), 0..src.len())]);
    }

    #[test]
    fn unknown_char_is_error_and_lexing_continues() {
        assert_eq!(
            lex("a $ b"),
            [
                (Ok(Token::Ident), 0..1),
                (Err(LexError::UnexpectedChar), 2..3),
                (Ok(Token::Ident), 4..5)
            ]
        );
    }

    #[test]
    fn raw_identifier_is_not_supported() {
        assert_eq!(
            lex("r#foo"),
            [
                (Ok(Token::Ident), 0..1),
                (Ok(Token::Pound), 1..2),
                (Ok(Token::Ident), 2..5)
            ]
        );
    }

    #[rstest]
    #[case("+", Token::Plus)]
    #[case("-", Token::Minus)]
    #[case("*", Token::Star)]
    #[case("/", Token::Slash)]
    #[case("%", Token::Percent)]
    #[case("&", Token::Amp)]
    #[case("|", Token::Pipe)]
    #[case("^", Token::Caret)]
    #[case("!", Token::Bang)]
    #[case("<<", Token::Shl)]
    #[case(">>", Token::Shr)]
    #[case("&&", Token::AndAnd)]
    #[case("||", Token::OrOr)]
    #[case("==", Token::EqEq)]
    #[case("!=", Token::Ne)]
    #[case("<", Token::Lt)]
    #[case(">", Token::Gt)]
    #[case("<=", Token::Le)]
    #[case(">=", Token::Ge)]
    #[case("=", Token::Eq)]
    #[case("+=", Token::PlusEq)]
    #[case("-=", Token::MinusEq)]
    #[case("*=", Token::StarEq)]
    #[case("/=", Token::SlashEq)]
    #[case("%=", Token::PercentEq)]
    #[case("&=", Token::AmpEq)]
    #[case("|=", Token::PipeEq)]
    #[case("^=", Token::CaretEq)]
    #[case("<<=", Token::ShlEq)]
    #[case(">>=", Token::ShrEq)]
    #[case(".", Token::Dot)]
    #[case("..", Token::DotDot)]
    #[case("..=", Token::DotDotEq)]
    #[case("::", Token::ColonColon)]
    #[case(":", Token::Colon)]
    #[case(";", Token::Semi)]
    #[case(",", Token::Comma)]
    #[case("->", Token::Arrow)]
    #[case("=>", Token::FatArrow)]
    #[case("?", Token::Question)]
    #[case("#", Token::Pound)]
    #[case("(", Token::LParen)]
    #[case(")", Token::RParen)]
    #[case("[", Token::LBracket)]
    #[case("]", Token::RBracket)]
    #[case("{", Token::LBrace)]
    #[case("}", Token::RBrace)]
    fn punctuation(#[case] src: &str, #[case] token: Token) {
        assert_eq!(lex(src), [(Ok(token), 0..src.len())]);
    }

    #[rstest]
    #[case::shl_eq("a<<=b", &[(Ok(Token::Ident), 0..1), (Ok(Token::ShlEq), 1..4), (Ok(Token::Ident), 4..5)])]
    #[case::shr("T>>", &[(Ok(Token::Ident), 0..1), (Ok(Token::Shr), 1..3)])]
    #[case::dot_dot("...", &[(Ok(Token::DotDot), 0..2), (Ok(Token::Dot), 2..3)])]
    #[case::pound("#!", &[(Ok(Token::Pound), 0..1), (Ok(Token::Bang), 1..2)])]
    fn longest_match(
        #[case] src: &str,
        #[case] expected: &[(Result<Token, LexError>, Range<usize>)],
    ) {
        assert_eq!(lex(src), expected);
    }

    #[rstest]
    #[case("@")]
    #[case("$")]
    #[case("~")]
    fn unsupported_symbol_is_error(#[case] src: &str) {
        assert_eq!(lex(src), [(Err(LexError::UnexpectedChar), 0..1)]);
    }

    #[rstest]
    #[case("0")]
    #[case("123")]
    #[case("1_000")]
    #[case("1_")]
    #[case("0xff")]
    #[case("0xFF_FF")]
    #[case("0x1f32")]
    #[case("0o17")]
    #[case("0b1010")]
    #[case("1u8")]
    #[case("1i64")]
    #[case("0xffusize")]
    #[case("0b1_u32")]
    fn integer_literal(#[case] src: &str) {
        assert_eq!(lex(src), [(Ok(Token::Int), 0..src.len())]);
    }

    #[rstest]
    #[case("1.0")]
    #[case("0.5")]
    #[case("1_000.5")]
    #[case("1e10")]
    #[case("2.5E-3")]
    #[case("1e+5")]
    #[case("1.0f32")]
    #[case("1f64")]
    #[case("1e10f32")]
    fn float_literal(#[case] src: &str) {
        assert_eq!(lex(src), [(Ok(Token::Float), 0..src.len())]);
    }

    #[rstest]
    #[case::dot("1.", &[(Ok(Token::Int), 0..1), (Ok(Token::Dot), 1..2)])]
    #[case::range("1..2", &[(Ok(Token::Int), 0..1), (Ok(Token::DotDot), 1..3), (Ok(Token::Int), 3..4)])]
    #[case::method("1.abs", &[(Ok(Token::Int), 0..1), (Ok(Token::Dot), 1..2), (Ok(Token::Ident), 2..5)])]
    #[case::tuple_field("t.0.1", &[(Ok(Token::Ident), 0..1), (Ok(Token::Dot), 1..2), (Ok(Token::Float), 2..5)])]
    fn number_followed_by_dot(
        #[case] src: &str,
        #[case] expected: &[(Result<Token, LexError>, Range<usize>)],
    ) {
        assert_eq!(lex(src), expected);
    }

    #[test]
    fn minus_is_not_part_of_literal() {
        assert_eq!(
            lex("-1"),
            [(Ok(Token::Minus), 0..1), (Ok(Token::Int), 1..2)]
        );
    }

    #[rstest]
    #[case("1u7", LexError::InvalidNumberSuffix)]
    #[case("1abc", LexError::InvalidNumberSuffix)]
    #[case("1.0x", LexError::InvalidNumberSuffix)]
    #[case("1f16", LexError::InvalidNumberSuffix)]
    #[case("1e+5abc", LexError::InvalidNumberSuffix)]
    #[case("2.5E-3u8", LexError::InvalidNumberSuffix)]
    #[case("0xffg", LexError::InvalidNumberSuffix)]
    #[case("0b12", LexError::InvalidDigit)]
    #[case("0o8", LexError::InvalidDigit)]
    #[case("0x", LexError::MissingDigits)]
    #[case("0b_", LexError::MissingDigits)]
    #[case("1e", LexError::MissingExponentDigits)]
    #[case("1.5E", LexError::MissingExponentDigits)]
    fn invalid_number_literal(#[case] src: &str, #[case] error: LexError) {
        assert_eq!(lex(src), [(Err(error), 0..src.len())]);
    }

    #[rstest]
    #[case("'a'")]
    #[case("'あ'")]
    #[case("'\"'")]
    #[case(r"'\n'")]
    #[case(r"'\''")]
    #[case(r"'\\'")]
    #[case(r"'\0'")]
    #[case(r"'\x7F'")]
    #[case(r"'\u{1F600}'")]
    #[case(r"'\u{10_FFFF}'")]
    fn char_literal(#[case] src: &str) {
        assert_eq!(lex(src), [(Ok(Token::Char), 0..src.len())]);
    }

    #[rstest]
    #[case("''", LexError::EmptyCharLiteral)]
    #[case("'ab'", LexError::TooManyCharsInCharLiteral)]
    #[case("'a", LexError::UnterminatedCharLiteral)]
    #[case("'''", LexError::UnescapedCharInCharLiteral)]
    #[case("'\t'", LexError::UnescapedCharInCharLiteral)]
    #[case(r"'\q'", LexError::InvalidEscape)]
    #[case(r"'\x80'", LexError::InvalidEscape)]
    #[case(r"'\u{110000}'", LexError::InvalidUnicodeEscape)]
    #[case(r"'\u{D800}'", LexError::InvalidUnicodeEscape)]
    #[case(r"'\u{}'", LexError::InvalidUnicodeEscape)]
    #[case(r"'\u{1234567}'", LexError::InvalidUnicodeEscape)]
    #[case("'a'x", LexError::ReservedLiteralSuffix)]
    fn invalid_char_literal(#[case] src: &str, #[case] error: LexError) {
        assert_eq!(lex(src), [(Err(error), 0..src.len())]);
    }

    #[rstest]
    #[case(r#""""#)]
    #[case(r#""abc""#)]
    #[case(r#""'""#)]
    #[case(r#""a\nb""#)]
    #[case(r#""\"\\""#)]
    #[case(r#""\\u{110000}""#)]
    #[case("\"line\nnext\"")]
    #[case("\"line\r\nnext\"")]
    #[case("\"a\\\n    b\"")]
    #[case("\"a\\\r\n    b\"")]
    fn string_literal(#[case] src: &str) {
        assert_eq!(lex(src), [(Ok(Token::Str), 0..src.len())]);
    }

    #[rstest]
    #[case(r#""abc"#, LexError::UnterminatedStringLiteral)]
    #[case(r#""a\qb""#, LexError::InvalidEscape)]
    #[case("\"a\rb\"", LexError::BareCarriageReturn)]
    #[case(r#""\u{D800}""#, LexError::InvalidUnicodeEscape)]
    #[case(r#""abc"x"#, LexError::ReservedLiteralSuffix)]
    fn invalid_string_literal(#[case] src: &str, #[case] error: LexError) {
        assert_eq!(lex(src), [(Err(error), 0..src.len())]);
    }

    #[test]
    fn lexing_continues_after_invalid_string() {
        assert_eq!(
            lex(r#""a\qb" x"#),
            [
                (Err(LexError::InvalidEscape), 0..6),
                (Ok(Token::Ident), 7..8)
            ]
        );
    }

    #[rstest]
    #[case("b'a'")]
    #[case("b'\"'")]
    #[case(r"b'\n'")]
    #[case(r"b'\''")]
    #[case(r"b'\\'")]
    #[case(r"b'\x7F'")]
    #[case(r"b'\xFF'")]
    fn byte_literal(#[case] src: &str) {
        assert_eq!(lex(src), [(Ok(Token::Byte), 0..src.len())]);
    }

    #[rstest]
    #[case("b'あ'", LexError::NonAsciiInByteLiteral)]
    #[case(r"b'\u{41}'", LexError::UnicodeEscapeInByteLiteral)]
    #[case("b''", LexError::EmptyCharLiteral)]
    #[case("b'ab'", LexError::TooManyCharsInCharLiteral)]
    #[case("b'a", LexError::UnterminatedCharLiteral)]
    #[case("b'a'x", LexError::ReservedLiteralSuffix)]
    #[case(r"b'\x1'", LexError::InvalidEscape)]
    fn invalid_byte_literal(#[case] src: &str, #[case] error: LexError) {
        assert_eq!(lex(src), [(Err(error), 0..src.len())]);
    }

    #[rstest]
    #[case(r#"b"""#)]
    #[case(r#"b"abc""#)]
    #[case(r#"b"あ""#)]
    #[case(r#"b"\xFF""#)]
    #[case(r#"b"\u{3042}""#)]
    #[case("b\"line\r\nnext\"")]
    #[case("b\"a\\\n    b\"")]
    fn byte_string_literal(#[case] src: &str) {
        assert_eq!(lex(src), [(Ok(Token::ByteStr), 0..src.len())]);
    }

    #[rstest]
    #[case(r#"b"abc"#, LexError::UnterminatedStringLiteral)]
    #[case(r#"b"\q""#, LexError::InvalidEscape)]
    #[case(r#"b"\u{D800}""#, LexError::InvalidUnicodeEscape)]
    #[case("b\"a\rb\"", LexError::BareCarriageReturn)]
    #[case(r#"b"a"x"#, LexError::ReservedLiteralSuffix)]
    fn invalid_byte_string_literal(#[case] src: &str, #[case] error: LexError) {
        assert_eq!(lex(src), [(Err(error), 0..src.len())]);
    }

    #[test]
    fn b_alone_is_identifier() {
        assert_eq!(lex("b"), [(Ok(Token::Ident), 0..1)]);
    }

    #[test]
    fn b_separated_from_char_is_identifier() {
        assert_eq!(
            lex("b 'a'"),
            [(Ok(Token::Ident), 0..1), (Ok(Token::Char), 2..5)]
        );
    }

    #[rstest]
    #[case(r#"r"""#)]
    #[case(r#"r"abc""#)]
    #[case(r#"r"\n""#)]
    #[case(r#"r"C:\path\""#)]
    #[case(r###"r#"a"b"#"###)]
    #[case(r###"r##"a"#b"##"###)]
    #[case("r\"a\r\nb\"")]
    fn raw_string_literal(#[case] src: &str) {
        assert_eq!(lex(src), [(Ok(Token::RawStr), 0..src.len())]);
    }

    #[rstest]
    #[case(r#"br"""#)]
    #[case(r#"br"あ\x""#)]
    #[case(r###"br#"a"b"#"###)]
    fn raw_byte_string_literal(#[case] src: &str) {
        assert_eq!(lex(src), [(Ok(Token::RawByteStr), 0..src.len())]);
    }

    #[rstest]
    #[case(r#"r"abc"#, LexError::UnterminatedRawString)]
    #[case(r###"r#"abc""###, LexError::UnterminatedRawString)]
    #[case("r\"a\rb\"", LexError::BareCarriageReturn)]
    #[case(r#"r"a"x"#, LexError::ReservedLiteralSuffix)]
    #[case(r#"br"a"x"#, LexError::ReservedLiteralSuffix)]
    fn invalid_raw_string_literal(#[case] src: &str, #[case] error: LexError) {
        assert_eq!(lex(src), [(Err(error), 0..src.len())]);
    }

    #[test]
    fn raw_string_with_max_hashes() {
        let src = format!("r{0}\"a\"{0}", "#".repeat(255));
        assert_eq!(lex(&src), [(Ok(Token::RawStr), 0..src.len())]);
    }

    #[test]
    fn raw_string_with_too_many_hashes() {
        let src = format!("r{0}\"a\"{0}", "#".repeat(256));
        assert_eq!(
            lex(&src),
            [(Err(LexError::TooManyRawStringHashes), 0..src.len())]
        );
    }

    #[test]
    fn lexing_continues_after_raw_string() {
        assert_eq!(
            lex(r###"r#"a"# x"###),
            [(Ok(Token::RawStr), 0..6), (Ok(Token::Ident), 7..8)]
        );
    }

    #[rstest]
    #[case("a // x\nb")]
    #[case("a // x\r\nb")]
    #[case("a /* x */ b")]
    #[case("a/*x*/b")]
    #[case("a /* x /* y */ z */ b")]
    #[case("a /**/ b")]
    #[case("a /*/ */ b")]
    #[case("a /*x/*y*/z*/ b")]
    #[case("a /*x**/ b")]
    #[case("a /*x//*y*/*/ b")]
    #[case("a /// x\nb")]
    #[case("a //! x\nb")]
    #[case("a /** x */ b")]
    #[case("a /*! x */ b")]
    fn comment_is_skipped(#[case] src: &str) {
        let b = src.len() - 1;
        assert_eq!(
            lex(src),
            [(Ok(Token::Ident), 0..1), (Ok(Token::Ident), b..b + 1)]
        );
    }

    #[test]
    fn line_comment_until_end_of_input() {
        assert_eq!(lex("a // x"), [(Ok(Token::Ident), 0..1)]);
    }

    #[test]
    fn unterminated_block_comment_is_error() {
        assert_eq!(
            lex("a /* x"),
            [
                (Ok(Token::Ident), 0..1),
                (Err(LexError::UnterminatedBlockComment), 2..6)
            ]
        );
    }

    #[test]
    fn unterminated_nested_block_comment_is_error() {
        assert_eq!(
            lex("/* /* */"),
            [(Err(LexError::UnterminatedBlockComment), 0..8)]
        );
    }

    #[test]
    fn comment_marker_in_string() {
        assert_eq!(lex(r#""// x""#), [(Ok(Token::Str), 0..6)]);
    }

    #[test]
    fn block_comment_end_without_start() {
        assert_eq!(
            lex("*/"),
            [(Ok(Token::Star), 0..1), (Ok(Token::Slash), 1..2)]
        );
    }
}

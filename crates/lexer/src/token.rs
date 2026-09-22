use logos::{Lexer, Logos};

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
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
        |_| false,
        priority = 0
    )]
    Int,
    #[regex(r"(?&dec)(\.(?&dec)(?&exp)?|(?&exp))(?&float_suffix)?")]
    #[regex(r"(?&dec)(?&float_suffix)")]
    Float,
    #[regex(r"'([^'\\\n\r\t]|(?&esc))'", unicode_escapes_are_valid)]
    /// 閉じていない・不正な内容・直後に識別子の文字が続く文字リテラルはエラーとする
    #[regex(r"'([^\\\n]|\\.)?([^'\\\n]|\\.)*'?\p{XID_Continue}*", |_| false, priority = 0)]
    Char,
    #[regex(r#""([^"\\\r]|\r\n|(?&esc)|\\\r?\n)*""#, unicode_escapes_are_valid)]
    /// 閉じていない・不正な内容・直後に識別子の文字が続く文字列リテラルはエラーとする
    #[regex(r#""([^"\\]|\\(.|\n))*("\p{XID_Continue}*)?"#, |_| false, priority = 0)]
    Str,
    #[regex(r"b'([\x00-\x7F&&[^'\\\n\r\t]]|(?&byte_esc))'")]
    /// 閉じていない・不正な内容・直後に識別子の文字が続くバイト文字リテラルはエラーとする
    #[regex(r"b'([^\\\n]|\\.)?([^'\\\n]|\\.)*'?\p{XID_Continue}*", |_| false, priority = 0)]
    Byte,
    #[regex(
        r#"b"([^"\\\r]|\r\n|(?&byte_esc)|(?&esc)|\\\r?\n)*""#,
        unicode_escapes_are_valid
    )]
    /// 閉じていない・不正な内容・直後に識別子の文字が続くバイト文字列リテラルはエラーとする
    #[regex(r#"b"([^"\\]|\\(.|\n))*("\p{XID_Continue}*)?"#, |_| false, priority = 0)]
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

/// `\u{...}` の値が Unicode スカラー値であるか検査する
fn unicode_escapes_are_valid(lex: &mut Lexer<Token>) -> bool {
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
}

/// raw 文字列を、開始と同じ数の `#` が `"` の後に続く終端まで読み進める
///
/// `#` の数をそろえる処理は正規表現で表現できないため、開始部分以降をここで扱う
fn raw_string(lex: &mut Lexer<Token>) -> bool {
    let hashes = lex.slice().matches('#').count();
    let terminator = format!("\"{}", "#".repeat(hashes));
    let remainder = lex.remainder();
    let Some(end) = remainder.find(&terminator) else {
        lex.bump(remainder.len());
        return false;
    };
    let has_bare_cr = remainder[..end].replace("\r\n", "").contains('\r');
    let suffix_len: usize = remainder[end + terminator.len()..]
        .chars()
        .take_while(|&c| unicode_ident::is_xid_continue(c))
        .map(char::len_utf8)
        .sum();
    lex.bump(end + terminator.len() + suffix_len);
    hashes <= 255 && !has_bare_cr && suffix_len == 0
}

/// ブロックコメントを、ネストの深さを数えて対応する `*/` まで読み飛ばす
///
/// ネストの対応は正規表現で表現できないため、開始部分以降をここで扱う
fn block_comment(lex: &mut Lexer<Token>) -> Result<(), ()> {
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
                return Err(());
            }
        }
    }
    lex.bump(i);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::LexError;
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

    #[test]
    fn identifiers() {
        for src in ["a", "a1", "_x", "__", "変数", "letter", "Selfie"] {
            assert_eq!(lex(src), [(Ok(Token::Ident), 0..src.len())], "{src}");
        }
    }

    #[test]
    fn underscore_alone_is_not_identifier() {
        assert_eq!(lex("_"), [(Ok(Token::Underscore), 0..1)]);
    }

    #[test]
    fn keywords() {
        let table = [
            ("as", Token::As),
            ("break", Token::Break),
            ("comptime", Token::Comptime),
            ("const", Token::Const),
            ("continue", Token::Continue),
            ("crate", Token::Crate),
            ("dyn", Token::Dyn),
            ("else", Token::Else),
            ("enum", Token::Enum),
            ("false", Token::False),
            ("fn", Token::Fn),
            ("for", Token::For),
            ("if", Token::If),
            ("impl", Token::Impl),
            ("in", Token::In),
            ("let", Token::Let),
            ("loop", Token::Loop),
            ("match", Token::Match),
            ("move", Token::Move),
            ("mut", Token::Mut),
            ("pub", Token::Pub),
            ("return", Token::Return),
            ("self", Token::SelfValue),
            ("Self", Token::SelfType),
            ("struct", Token::Struct),
            ("super", Token::Super),
            ("trait", Token::Trait),
            ("true", Token::True),
            ("type", Token::Type),
            ("use", Token::Use),
            ("where", Token::Where),
            ("while", Token::While),
            ("yield", Token::Yield),
        ];
        for (src, token) in table {
            assert_eq!(lex(src), [(Ok(token), 0..src.len())], "{src}");
        }
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

    #[test]
    fn punctuation() {
        let table = [
            ("+", Token::Plus),
            ("-", Token::Minus),
            ("*", Token::Star),
            ("/", Token::Slash),
            ("%", Token::Percent),
            ("&", Token::Amp),
            ("|", Token::Pipe),
            ("^", Token::Caret),
            ("!", Token::Bang),
            ("<<", Token::Shl),
            (">>", Token::Shr),
            ("&&", Token::AndAnd),
            ("||", Token::OrOr),
            ("==", Token::EqEq),
            ("!=", Token::Ne),
            ("<", Token::Lt),
            (">", Token::Gt),
            ("<=", Token::Le),
            (">=", Token::Ge),
            ("=", Token::Eq),
            ("+=", Token::PlusEq),
            ("-=", Token::MinusEq),
            ("*=", Token::StarEq),
            ("/=", Token::SlashEq),
            ("%=", Token::PercentEq),
            ("&=", Token::AmpEq),
            ("|=", Token::PipeEq),
            ("^=", Token::CaretEq),
            ("<<=", Token::ShlEq),
            (">>=", Token::ShrEq),
            (".", Token::Dot),
            ("..", Token::DotDot),
            ("..=", Token::DotDotEq),
            ("::", Token::ColonColon),
            (":", Token::Colon),
            (";", Token::Semi),
            (",", Token::Comma),
            ("->", Token::Arrow),
            ("=>", Token::FatArrow),
            ("?", Token::Question),
            ("#", Token::Pound),
            ("(", Token::LParen),
            (")", Token::RParen),
            ("[", Token::LBracket),
            ("]", Token::RBracket),
            ("{", Token::LBrace),
            ("}", Token::RBrace),
        ];
        for (src, token) in table {
            assert_eq!(lex(src), [(Ok(token), 0..src.len())], "{src}");
        }
    }

    #[test]
    fn longest_match() {
        assert_eq!(
            lex("a<<=b"),
            [
                (Ok(Token::Ident), 0..1),
                (Ok(Token::ShlEq), 1..4),
                (Ok(Token::Ident), 4..5)
            ]
        );
        assert_eq!(
            lex("T>>"),
            [(Ok(Token::Ident), 0..1), (Ok(Token::Shr), 1..3)]
        );
        assert_eq!(
            lex("..."),
            [(Ok(Token::DotDot), 0..2), (Ok(Token::Dot), 2..3)]
        );
        assert_eq!(
            lex("#!"),
            [(Ok(Token::Pound), 0..1), (Ok(Token::Bang), 1..2)]
        );
    }

    #[test]
    fn unsupported_symbols_are_errors() {
        for src in ["@", "$", "~"] {
            assert_eq!(lex(src), [(Err(LexError::UnexpectedChar), 0..1)], "{src}");
        }
    }

    #[test]
    fn integer_literals() {
        for src in [
            "0",
            "123",
            "1_000",
            "1_",
            "0xff",
            "0xFF_FF",
            "0x1f32",
            "0o17",
            "0b1010",
            "1u8",
            "1i64",
            "0xffusize",
            "0b1_u32",
        ] {
            assert_eq!(lex(src), [(Ok(Token::Int), 0..src.len())], "{src}");
        }
    }

    #[test]
    fn float_literals() {
        for src in [
            "1.0", "0.5", "1_000.5", "1e10", "2.5E-3", "1e+5", "1.0f32", "1f64", "1e10f32",
        ] {
            assert_eq!(lex(src), [(Ok(Token::Float), 0..src.len())], "{src}");
        }
    }

    #[test]
    fn number_followed_by_dot() {
        assert_eq!(lex("1."), [(Ok(Token::Int), 0..1), (Ok(Token::Dot), 1..2)]);
        assert_eq!(
            lex("1..2"),
            [
                (Ok(Token::Int), 0..1),
                (Ok(Token::DotDot), 1..3),
                (Ok(Token::Int), 3..4)
            ]
        );
        assert_eq!(
            lex("1.abs"),
            [
                (Ok(Token::Int), 0..1),
                (Ok(Token::Dot), 1..2),
                (Ok(Token::Ident), 2..5)
            ]
        );
        assert_eq!(
            lex("t.0.1"),
            [
                (Ok(Token::Ident), 0..1),
                (Ok(Token::Dot), 1..2),
                (Ok(Token::Float), 2..5)
            ]
        );
    }

    #[test]
    fn minus_is_not_part_of_literal() {
        assert_eq!(
            lex("-1"),
            [(Ok(Token::Minus), 0..1), (Ok(Token::Int), 1..2)]
        );
    }

    #[test]
    fn invalid_number_literals() {
        for (src, error) in [
            ("1u7", LexError::InvalidNumberSuffix),
            ("1abc", LexError::InvalidNumberSuffix),
            ("1.0x", LexError::InvalidNumberSuffix),
            ("1f16", LexError::InvalidNumberSuffix),
            ("1e+5abc", LexError::InvalidNumberSuffix),
            ("2.5E-3u8", LexError::InvalidNumberSuffix),
            ("0xffg", LexError::InvalidNumberSuffix),
            ("0b12", LexError::InvalidDigit),
            ("0o8", LexError::InvalidDigit),
            ("0x", LexError::MissingDigits),
            ("0b_", LexError::MissingDigits),
            ("1e", LexError::MissingExponentDigits),
            ("1.5E", LexError::MissingExponentDigits),
        ] {
            assert_eq!(lex(src), [(Err(error), 0..src.len())], "{src}");
        }
    }

    #[test]
    fn char_literals() {
        for src in [
            "'a'",
            "'あ'",
            "'\"'",
            r"'\n'",
            r"'\''",
            r"'\\'",
            r"'\0'",
            r"'\x7F'",
            r"'\u{1F600}'",
            r"'\u{10_FFFF}'",
        ] {
            assert_eq!(lex(src), [(Ok(Token::Char), 0..src.len())], "{src}");
        }
    }

    #[test]
    fn invalid_char_literals() {
        for (src, error) in [
            ("''", LexError::InvalidCharLiteral),
            ("'ab'", LexError::InvalidCharLiteral),
            ("'a", LexError::InvalidCharLiteral),
            ("'''", LexError::InvalidCharLiteral),
            ("'\t'", LexError::InvalidCharLiteral),
            (r"'\q'", LexError::InvalidCharLiteral),
            (r"'\x80'", LexError::InvalidCharLiteral),
            (r"'\u{110000}'", LexError::InvalidUnicodeEscape),
            (r"'\u{D800}'", LexError::InvalidUnicodeEscape),
            (r"'\u{}'", LexError::InvalidCharLiteral),
            (r"'\u{1234567}'", LexError::InvalidCharLiteral),
            ("'a'x", LexError::InvalidCharLiteral),
        ] {
            assert_eq!(lex(src), [(Err(error), 0..src.len())], "{src}");
        }
    }

    #[test]
    fn string_literals() {
        for src in [
            r#""""#,
            r#""abc""#,
            r#""'""#,
            r#""a\nb""#,
            r#""\"\\""#,
            r#""\\u{110000}""#,
            "\"line\nnext\"",
            "\"line\r\nnext\"",
            "\"a\\\n    b\"",
            "\"a\\\r\n    b\"",
        ] {
            assert_eq!(lex(src), [(Ok(Token::Str), 0..src.len())], "{src:?}");
        }
    }

    #[test]
    fn invalid_string_literals() {
        for (src, error) in [
            (r#""abc"#, LexError::InvalidStringLiteral),
            (r#""a\qb""#, LexError::InvalidStringLiteral),
            ("\"a\rb\"", LexError::InvalidStringLiteral),
            (r#""\u{D800}""#, LexError::InvalidUnicodeEscape),
            (r#""abc"x"#, LexError::InvalidStringLiteral),
        ] {
            assert_eq!(lex(src), [(Err(error), 0..src.len())], "{src:?}");
        }
    }

    #[test]
    fn lexing_continues_after_invalid_string() {
        assert_eq!(
            lex(r#""a\qb" x"#),
            [
                (Err(LexError::InvalidStringLiteral), 0..6),
                (Ok(Token::Ident), 7..8)
            ]
        );
    }

    #[test]
    fn byte_literals() {
        for src in [
            "b'a'", "b'\"'", r"b'\n'", r"b'\''", r"b'\\'", r"b'\x7F'", r"b'\xFF'",
        ] {
            assert_eq!(lex(src), [(Ok(Token::Byte), 0..src.len())], "{src}");
        }
    }

    #[test]
    fn invalid_byte_literals() {
        for src in [
            "b'あ'",
            r"b'\u{41}'",
            "b''",
            "b'ab'",
            "b'a",
            "b'a'x",
            r"b'\x1'",
        ] {
            let error = LexError::InvalidByteLiteral;
            assert_eq!(lex(src), [(Err(error), 0..src.len())], "{src}");
        }
    }

    #[test]
    fn byte_string_literals() {
        for src in [
            r#"b"""#,
            r#"b"abc""#,
            r#"b"あ""#,
            r#"b"\xFF""#,
            r#"b"\u{3042}""#,
            "b\"line\r\nnext\"",
            "b\"a\\\n    b\"",
        ] {
            assert_eq!(lex(src), [(Ok(Token::ByteStr), 0..src.len())], "{src:?}");
        }
    }

    #[test]
    fn invalid_byte_string_literals() {
        for (src, error) in [
            (r#"b"abc"#, LexError::InvalidByteStringLiteral),
            (r#"b"\q""#, LexError::InvalidByteStringLiteral),
            (r#"b"\u{D800}""#, LexError::InvalidUnicodeEscape),
            ("b\"a\rb\"", LexError::InvalidByteStringLiteral),
            (r#"b"a"x"#, LexError::InvalidByteStringLiteral),
        ] {
            assert_eq!(lex(src), [(Err(error), 0..src.len())], "{src:?}");
        }
    }

    #[test]
    fn b_alone_is_identifier() {
        assert_eq!(lex("b"), [(Ok(Token::Ident), 0..1)]);
        assert_eq!(
            lex("b 'a'"),
            [(Ok(Token::Ident), 0..1), (Ok(Token::Char), 2..5)]
        );
    }

    #[test]
    fn raw_string_literals() {
        let max_hashes = format!("r{0}\"a\"{0}", "#".repeat(255));
        for src in [
            r#"r"""#,
            r#"r"abc""#,
            r#"r"\n""#,
            r#"r"C:\path\""#,
            r###"r#"a"b"#"###,
            r###"r##"a"#b"##"###,
            "r\"a\r\nb\"",
            &max_hashes,
        ] {
            assert_eq!(lex(src), [(Ok(Token::RawStr), 0..src.len())], "{src:?}");
        }
    }

    #[test]
    fn raw_byte_string_literals() {
        for src in [r#"br"""#, r#"br"あ\x""#, r###"br#"a"b"#"###] {
            assert_eq!(lex(src), [(Ok(Token::RawByteStr), 0..src.len())], "{src:?}");
        }
    }

    #[test]
    fn invalid_raw_string_literals() {
        let too_many_hashes = format!("r{0}\"a\"{0}", "#".repeat(256));
        for (src, error) in [
            (r#"r"abc"#, LexError::UnterminatedRawString),
            (r###"r#"abc""###, LexError::UnterminatedRawString),
            ("r\"a\rb\"", LexError::BareCarriageReturn),
            (r#"r"a"x"#, LexError::ReservedLiteralSuffix),
            (r#"br"a"x"#, LexError::ReservedLiteralSuffix),
            (&too_many_hashes, LexError::TooManyRawStringHashes),
        ] {
            assert_eq!(lex(src), [(Err(error), 0..src.len())], "{src:?}");
        }
    }

    #[test]
    fn lexing_continues_after_raw_string() {
        assert_eq!(
            lex(r###"r#"a"# x"###),
            [(Ok(Token::RawStr), 0..6), (Ok(Token::Ident), 7..8)]
        );
    }

    #[test]
    fn comments_are_skipped() {
        for src in [
            "a // x\nb",
            "a // x\r\nb",
            "a /* x */ b",
            "a/*x*/b",
            "a /* x /* y */ z */ b",
            "a /**/ b",
            "a /*/ */ b",
            "a /*x/*y*/z*/ b",
            "a /*x**/ b",
            "a /*x//*y*/*/ b",
            "a /// x\nb",
            "a //! x\nb",
            "a /** x */ b",
            "a /*! x */ b",
        ] {
            let b = src.len() - 1;
            assert_eq!(
                lex(src),
                [(Ok(Token::Ident), 0..1), (Ok(Token::Ident), b..b + 1)],
                "{src:?}"
            );
        }
    }

    #[test]
    fn line_comment_until_end_of_input() {
        assert_eq!(lex("a // x"), [(Ok(Token::Ident), 0..1)]);
    }

    #[test]
    fn unterminated_block_comment_is_error() {
        let error = LexError::UnterminatedBlockComment;
        assert_eq!(
            lex("a /* x"),
            [(Ok(Token::Ident), 0..1), (Err(error), 2..6)]
        );
        assert_eq!(lex("/* /* */"), [(Err(error), 0..8)]);
    }

    #[test]
    fn comment_markers_in_other_contexts() {
        assert_eq!(lex(r#""// x""#), [(Ok(Token::Str), 0..6)]);
        assert_eq!(
            lex("*/"),
            [(Ok(Token::Star), 0..1), (Ok(Token::Slash), 1..2)]
        );
    }
}

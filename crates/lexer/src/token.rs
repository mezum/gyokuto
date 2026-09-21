use logos::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(skip r"\p{Pattern_White_Space}+")]
#[logos(subpattern dec = r"[0-9][0-9_]*")]
#[logos(subpattern hex = r"0x[0-9a-fA-F_]*[0-9a-fA-F][0-9a-fA-F_]*")]
#[logos(subpattern oct = r"0o[0-7_]*[0-7][0-7_]*")]
#[logos(subpattern bin = r"0b[01_]*[01][01_]*")]
#[logos(subpattern exp = r"[eE][+-]?[0-9_]*[0-9][0-9_]*")]
#[logos(subpattern float_suffix = r"f32|f64")]
pub enum Token {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Range;

    fn lex(src: &str) -> Vec<(Result<Token, ()>, Range<usize>)> {
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
                (Err(()), 2..3),
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
            assert_eq!(lex(src), [(Err(()), 0..1)], "{src}");
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
    fn invalid_suffix_is_error() {
        for src in [
            "1u7", "1abc", "0b12", "0x", "1e", "0o8", "1.0x", "1f16", "1e+5abc", "2.5E-3u8",
        ] {
            assert_eq!(lex(src), [(Err(()), 0..src.len())], "{src}");
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
            r"'\'",
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
        for src in [
            "''",
            "'ab'",
            "'a",
            "'''",
            "'\t'",
            r"'\q'",
            r"'\x80'",
            r"'\u{110000}'",
            r"'\u{D800}'",
            r"'\u{}'",
            r"'\u{1234567}'",
            "'a'x",
        ] {
            assert_eq!(lex(src), [(Err(()), 0..src.len())], "{src}");
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
            r#""\u{110000}""#,
            "\"line\nnext\"",
            "\"line\r\nnext\"",
            "\"a\\n    b\"",
            "\"a\\r\n    b\"",
        ] {
            assert_eq!(lex(src), [(Ok(Token::Str), 0..src.len())], "{src:?}");
        }
    }

    #[test]
    fn invalid_string_literals() {
        for src in [
            r#""abc"#,
            r#""a\qb""#,
            "\"a\rb\"",
            r#""\u{D800}""#,
            r#""abc"x"#,
        ] {
            assert_eq!(lex(src), [(Err(()), 0..src.len())], "{src:?}");
        }
    }

    #[test]
    fn lexing_continues_after_invalid_string() {
        assert_eq!(
            lex(r#""a\qb" x"#),
            [(Err(()), 0..6), (Ok(Token::Ident), 7..8)]
        );
    }
}

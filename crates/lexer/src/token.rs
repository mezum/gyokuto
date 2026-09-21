use logos::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(skip r"\p{Pattern_White_Space}+")]
pub enum Token {
    #[regex(r"[\p{XID_Start}_]\p{XID_Continue}*")]
    Ident,
    #[token("_", priority = 3)]
    Underscore,

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
                (Err(()), 1..2),
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
}

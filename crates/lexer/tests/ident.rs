use logos::Logos;
use std::ops::Range;
use typed_vm_lexer::Token;

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

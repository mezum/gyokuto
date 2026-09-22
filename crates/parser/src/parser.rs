use crate::ast::{Expr, ExprKind, Lit, Path, PathSegment, Span};
use crate::literal;
use chumsky::{input::ValueInput, prelude::*};
use gyokuto_lexer::Token;
use logos::Logos;
use std::iter::once;

pub type Error = Rich<'static, Token>;

type Extra<'tok> = extra::Err<Rich<'tok, Token>>;

/// 式を解析する
pub fn parse_expr(src: &str) -> (Option<Expr>, Vec<Error>) {
    let tokens: Vec<(Token, Span)> = Token::lexer(src)
        .spanned()
        .map(|(token, span)| (token.unwrap_or(Token::Error), span.into()))
        .collect();
    let lex_errors = tokens
        .iter()
        .filter(|(token, _)| *token == Token::Error)
        .map(|(_, span)| Rich::custom(*span, "invalid token"));
    let eoi = Span::from(src.len()..src.len());
    let (expr, parse_errors) = expr(src)
        .then_ignore(end())
        .parse(tokens.as_slice().map(eoi, |(token, span)| (token, span)))
        .into_output_errors();
    let errors = lex_errors
        .chain(parse_errors.into_iter().map(Rich::into_owned))
        .collect();
    (expr, errors)
}

fn expr<'tok, 'src: 'tok, I>(src: &'src str) -> impl Parser<'tok, I, Expr, Extra<'tok>> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    recursive(|expr| {
        let bool_lit = select! {
            Token::True => ExprKind::Lit(Lit::Bool(true)),
            Token::False => ExprKind::Lit(Lit::Bool(false)),
        };
        let lit = one_of([
            Token::Int,
            Token::Float,
            Token::Char,
            Token::Byte,
            Token::Str,
            Token::ByteStr,
            Token::RawStr,
            Token::RawByteStr,
        ])
        .validate(move |token, e, emitter| {
            let span: Span = e.span();
            literal::decode(token, &src[span.into_range()])
                .map(ExprKind::Lit)
                .unwrap_or_else(|message| {
                    emitter.emit(Rich::custom(span, message));
                    ExprKind::Error
                })
        });

        let rest_items = just(Token::Comma).ignore_then(
            expr.clone()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect::<Vec<_>>(),
        );

        let parens = expr
            .clone()
            .then(rest_items.clone().or_not())
            .or_not()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .map(|inner| match inner {
                None => ExprKind::Tuple(Vec::new()),
                Some((first, None)) => ExprKind::Paren(Box::new(first)),
                Some((first, Some(rest))) => ExprKind::Tuple(once(first).chain(rest).collect()),
            });

        enum ArrayTail {
            Repeat(Expr),
            List(Vec<Expr>),
        }
        let array = expr
            .clone()
            .then(choice((
                just(Token::Semi)
                    .ignore_then(expr.clone())
                    .map(ArrayTail::Repeat),
                rest_items.map(ArrayTail::List),
                empty().map(|()| ArrayTail::List(Vec::new())),
            )))
            .or_not()
            .delimited_by(just(Token::LBracket), just(Token::RBracket))
            .map(|inner| match inner {
                None => ExprKind::Array(Vec::new()),
                Some((elem, ArrayTail::Repeat(len))) => ExprKind::Repeat {
                    elem: Box::new(elem),
                    len: Box::new(len),
                },
                Some((first, ArrayTail::List(rest))) => {
                    ExprKind::Array(once(first).chain(rest).collect())
                }
            });

        let error = |span| Expr {
            kind: ExprKind::Error,
            span,
        };
        choice((
            bool_lit,
            lit,
            path(src).map(ExprKind::Path),
            parens,
            array,
            just(Token::Error).to(ExprKind::Error),
        ))
        .map_with(|kind, e| Expr {
            kind,
            span: e.span(),
        })
        .recover_with(via_parser(nested_delimiters(
            Token::LParen,
            Token::RParen,
            [
                (Token::LBracket, Token::RBracket),
                (Token::LBrace, Token::RBrace),
            ],
            error,
        )))
        .recover_with(via_parser(nested_delimiters(
            Token::LBracket,
            Token::RBracket,
            [
                (Token::LParen, Token::RParen),
                (Token::LBrace, Token::RBrace),
            ],
            error,
        )))
    })
}

fn path<'tok, 'src: 'tok, I>(src: &'src str) -> impl Parser<'tok, I, Path, Extra<'tok>> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    let ident = just(Token::Ident)
        .to_span()
        .map(move |span: Span| PathSegment::Ident(src[span.into_range()].to_string()));
    let head = choice((
        just(Token::Crate).to(vec![PathSegment::Crate]),
        just(Token::SelfValue).to(vec![PathSegment::SelfValue]),
        just(Token::SelfType).to(vec![PathSegment::SelfType]),
        just(Token::Super)
            .to(PathSegment::Super)
            .separated_by(just(Token::ColonColon))
            .at_least(1)
            .collect(),
        ident.map(|segment| vec![segment]),
    ));
    let rest = just(Token::ColonColon)
        .ignore_then(ident)
        .repeated()
        .collect::<Vec<_>>();
    head.then(rest).map(|(mut segments, rest)| {
        segments.extend(rest);
        Path { segments }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(expr: &Expr) -> String {
        let list = |name: &str, exprs: &[&Expr]| {
            let items: String = exprs.iter().map(|e| format!(" {}", show(e))).collect();
            format!("({name}{items})")
        };
        match &expr.kind {
            ExprKind::Lit(Lit::Bool(b)) => b.to_string(),
            ExprKind::Lit(lit) => format!("{lit:?}"),
            ExprKind::Path(path) => path
                .segments
                .iter()
                .map(|s| match s {
                    PathSegment::Ident(name) => name.as_str(),
                    PathSegment::Crate => "crate",
                    PathSegment::Super => "super",
                    PathSegment::SelfValue => "self",
                    PathSegment::SelfType => "Self",
                })
                .collect::<Vec<_>>()
                .join("::"),
            ExprKind::Paren(e) => list("paren", &[e]),
            ExprKind::Tuple(es) => list("tuple", &es.iter().collect::<Vec<_>>()),
            ExprKind::Array(es) => list("array", &es.iter().collect::<Vec<_>>()),
            ExprKind::Repeat { elem, len } => list("repeat", &[elem, len]),
            ExprKind::Error => "error".to_string(),
        }
    }

    fn parse_ok(src: &str) -> String {
        let (expr, errors) = parse_expr(src);
        assert!(errors.is_empty(), "{src}: {errors:?}");
        show(&expr.unwrap())
    }

    #[test]
    fn bool_literals() {
        assert_eq!(parse_ok("true"), "true");
        assert_eq!(parse_ok("false"), "false");
    }

    #[test]
    fn literals_in_expressions() {
        assert_eq!(
            parse_ok(r#"(1u8, 2.5, 'a', "s")"#),
            r#"(tuple Int { value: 1, suffix: Some(U8) } Float { digits: "2.5", suffix: None } Char('a') Str("s"))"#
        );
    }

    #[test]
    fn multi_scalar_graphemes_are_not_chars() {
        for src in ["'e\u{301}'", "'👨\u{200D}👩'", "'🇯🇵'"] {
            assert!(!parse_expr(src).1.is_empty(), "{src:?}");
        }
    }

    #[test]
    fn too_large_integer_is_error() {
        let (expr, errors) = parse_expr("[18446744073709551616, a]");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].span().into_range(), 1..21);
        assert_eq!(show(&expr.unwrap()), "(array error a)");
    }

    #[test]
    fn paths() {
        for src in [
            "a",
            "a::b::c",
            "crate::a",
            "self",
            "self::a",
            "Self::new",
            "super::a",
            "super::super::a",
        ] {
            assert_eq!(parse_ok(src), src);
        }
    }

    #[test]
    fn invalid_paths() {
        for src in [
            "a::crate",
            "a::self",
            "a::super",
            "super::a::super",
            "::a",
            "a::",
        ] {
            assert!(!parse_expr(src).1.is_empty(), "{src}");
        }
    }

    #[test]
    fn spans() {
        let (expr, _) = parse_expr(" a::b ");
        assert_eq!(expr.unwrap().span.into_range(), 1..5);
    }

    #[test]
    fn lexical_errors_are_reported() {
        let (expr, errors) = parse_expr("$");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].span().into_range(), 0..1);
        assert_eq!(show(&expr.unwrap()), "error");
    }

    #[test]
    fn parens_and_tuples() {
        assert_eq!(parse_ok("(a)"), "(paren a)");
        assert_eq!(parse_ok("()"), "(tuple)");
        assert_eq!(parse_ok("(a,)"), "(tuple a)");
        assert_eq!(parse_ok("(a, b)"), "(tuple a b)");
        assert_eq!(parse_ok("(a, b,)"), "(tuple a b)");
        assert_eq!(parse_ok("((a))"), "(paren (paren a))");
    }

    #[test]
    fn arrays() {
        assert_eq!(parse_ok("[]"), "(array)");
        assert_eq!(parse_ok("[a, b,]"), "(array a b)");
        assert_eq!(parse_ok("[a; n]"), "(repeat a n)");
        assert_eq!(parse_ok("[(a, b), [c]]"), "(array (tuple a b) (array c))");
    }

    #[test]
    fn invalid_separators() {
        for src in ["(a,,)", "(,)", "[a,,]", "[,]", "[a; n,]", "[a, b; n]"] {
            assert!(!parse_expr(src).1.is_empty(), "{src}");
        }
    }

    #[test]
    fn deeply_nested_delimiters() {
        let depth = 64;
        let parens = format!("{}a{}", "(".repeat(depth), ")".repeat(depth));
        let arrays = format!("{}a{}", "[".repeat(depth), "]".repeat(depth));
        assert!(parse_expr(&parens).1.is_empty());
        assert!(parse_expr(&arrays).1.is_empty());
    }

    #[test]
    fn nested_spans() {
        let (expr, _) = parse_expr("( a )");
        let expr = expr.unwrap();
        assert_eq!(expr.span.into_range(), 0..5);
        let ExprKind::Paren(inner) = expr.kind else {
            panic!("{expr:?}");
        };
        assert_eq!(inner.span.into_range(), 2..3);
    }

    #[test]
    fn recovers_inside_delimiters() {
        let (expr, errors) = parse_expr("[(a b), c]");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(show(&expr.unwrap()), "(array error c)");
    }

    #[test]
    fn lexical_error_inside_delimiters() {
        let (expr, errors) = parse_expr("[a, $]");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(show(&expr.unwrap()), "(array a error)");
    }

    #[test]
    fn trailing_tokens_are_error() {
        assert!(!parse_expr("a b").1.is_empty());
    }
}

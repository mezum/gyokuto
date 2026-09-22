use crate::ast::{Expr, ExprKind, Lit, Path, PathSegment, Span};
use chumsky::{input::ValueInput, prelude::*};
use logos::Logos;
use typed_vm_lexer::Token;

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
    let lit = select! {
        Token::True => Lit::Bool(true),
        Token::False => Lit::Bool(false),
    };
    choice((
        lit.map(ExprKind::Lit),
        path(src).map(ExprKind::Path),
        just(Token::Error).to(ExprKind::Error),
    ))
    .map_with(|kind, e| Expr {
        kind,
        span: e.span(),
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
        match &expr.kind {
            ExprKind::Lit(Lit::Bool(b)) => b.to_string(),
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
    fn trailing_tokens_are_error() {
        assert!(!parse_expr("a b").1.is_empty());
    }
}

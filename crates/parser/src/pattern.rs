use crate::ast::{Expr, ExprKind, Pat, PatKind, PathName, Span};
use crate::control::block_like;
use crate::error::Error;
use crate::parser::{Extra, expr, input, lex, path, signed_literal};
use crate::types::{angle_args, ty};
use chumsky::{input::ValueInput, prelude::*};
use gyokuto_lexer::Token;
use std::iter::once;

/// パターンを解析する
pub fn parse_pattern(src: &str) -> (Option<Pat>, Vec<Error>) {
    let (tokens, mut errors) = lex(src);
    let expr = expr(src);
    let (pat, parse_errors) = pattern(src, expr.clone(), block_like(src, expr))
        .then_ignore(end())
        .parse(input(&tokens, src))
        .into_output_errors();
    errors.extend(parse_errors);
    (pat, errors)
}

/// パターンを解析する。型引数の式は `expr`、ブロック式の型引数は `block_like` で解析する
pub(crate) fn pattern<'tok, 'src: 'tok, I>(
    src: &'src str,
    expr: impl Parser<'tok, I, Expr, Extra> + Clone + 'tok,
    block_like: impl Parser<'tok, I, Expr, Extra> + Clone + 'tok,
) -> impl Parser<'tok, I, Pat, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    recursive(move |pattern| {
        let no_top_in = no_top_in(src, expr, block_like, pattern);
        let no_top = recursive(|no_top| {
            just(Token::Mut)
                .or_not()
                .map(|m| m.is_some())
                .then(ident(src))
                .then_ignore(just(Token::In))
                .then(no_top)
                .map_with(|((mutable, name), sub), e| Pat {
                    kind: PatKind::Ident {
                        mutable,
                        name,
                        sub: Some(Box::new(sub)),
                    },
                    span: e.span(),
                })
                .or(no_top_in)
        });
        no_top
            .clone()
            .then(
                just(Token::Pipe)
                    .ignore_then(no_top)
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map_with(|(first, rest), e| match rest.is_empty() {
                true => first,
                false => Pat {
                    kind: PatKind::Or(once(first).chain(rest).collect()),
                    span: e.span(),
                },
            })
    })
}

/// 最上位に `in` と `|` を含まないパターン (PatternNoTopIn) を解析する。内側のパターンは `pattern` で解析する
pub(crate) fn no_top_in<'tok, 'src: 'tok, I>(
    src: &'src str,
    expr: impl Parser<'tok, I, Expr, Extra> + Clone + 'tok,
    block_like: impl Parser<'tok, I, Expr, Extra> + Clone + 'tok,
    pattern: impl Parser<'tok, I, Pat, Extra> + Clone + 'tok,
) -> impl Parser<'tok, I, Pat, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    let turbofish = just(Token::ColonColon).ignore_then(angle_args(
        src,
        ty(src, expr, block_like.clone()),
        block_like,
    ));
    let path = path(src, turbofish.or_not());
    let bound = signed_literal(src).or(path.clone().map_with(|path, e| Expr {
        kind: ExprKind::Path(path),
        span: e.span(),
    }));
    let range = choice((
        bound
            .clone()
            .then(choice((
                just(Token::DotDotEq)
                    .ignore_then(bound.clone())
                    .map(|end| (true, Some(end))),
                just(Token::DotDot)
                    .ignore_then(bound.clone().or_not())
                    .map(|end| (false, end)),
            )))
            .map(|(start, (inclusive, end))| (Some(start), end, inclusive)),
        just(Token::DotDotEq)
            .ignore_then(bound)
            .map(|end| (None, Some(end), true)),
    ))
    .map(
        |(start, end, inclusive): (Option<Expr>, Option<Expr>, bool)| PatKind::Range {
            start: start.map(Box::new),
            end: end.map(Box::new),
            inclusive,
        },
    );

    let items = pattern
        .clone()
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>();
    let parens = pattern
        .then(just(Token::Comma).ignore_then(items).or_not())
        .or_not()
        .delimited_by(just(Token::LParen), just(Token::RParen))
        .map(|inner| match inner {
            None => PatKind::Tuple(Vec::new()),
            Some((first, None)) => PatKind::Paren(Box::new(first)),
            Some((first, Some(rest))) => PatKind::Tuple(once(first).chain(rest).collect()),
        });
    let binding = path.map(|path| match &path.segments[..] {
        [segment] if segment.args.is_none() => match &segment.name {
            PathName::Ident(name) => PatKind::Ident {
                mutable: false,
                name: name.clone(),
                sub: None,
            },
            _ => PatKind::Path(path),
        },
        _ => PatKind::Path(path),
    });

    let simple = choice((
        just(Token::Underscore).to(PatKind::Wild),
        just(Token::DotDot).to(PatKind::Rest),
        signed_literal(src).map(PatKind::Lit),
        parens,
        just(Token::Mut)
            .ignore_then(ident(src))
            .map(|name| PatKind::Ident {
                mutable: true,
                name,
                sub: None,
            }),
        binding,
    ))
    .map_with(|kind, e| Pat {
        kind,
        span: e.span(),
    });
    range
        .map_with(|kind, e| Pat {
            kind,
            span: e.span(),
        })
        .or(simple)
}

fn ident<'tok, 'src: 'tok, I>(src: &'src str) -> impl Parser<'tok, I, String, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    just(Token::Ident)
        .to_span()
        .map(move |span: Span| src[span.into_range()].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sexpr::AsSexpr;
    use rstest::rstest;

    fn parse_ok(src: &str) -> String {
        let (pat, errors) = parse_pattern(src);
        assert!(errors.is_empty(), "{src}: {errors:?}");
        pat.unwrap().as_sexpr()
    }

    #[rstest]
    #[case("_", "_")]
    #[case("x", "x")]
    #[case("mut x", "(mut x)")]
    #[case("true", "true")]
    #[case("'a'", "Char('a')")]
    #[case("-1", "(Neg Int { value: 1, suffix: None })")]
    fn simple_pattern(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("x in _", "(in x _)")]
    #[case("mut x in (a, b)", "(in mut x (tuple a b))")]
    #[case("x in y in _", "(in x (in y _))")]
    #[case("x in a | b", "(| (in x a) b)")]
    fn binding_pattern(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("'a'..='z'", "(..= Char('a') Char('z'))")]
    #[case("'a'..'z'", "(.. Char('a') Char('z'))")]
    #[case("'a'..", "(.. Char('a') _)")]
    #[case("..='z'", "(..= _ Char('z'))")]
    #[case("MIN..MAX", "(.. MIN MAX)")]
    #[case(
        "-1..=1",
        "(..= (Neg Int { value: 1, suffix: None }) Int { value: 1, suffix: None })"
    )]
    fn range_pattern(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("()", "(tuple)")]
    #[case("(x)", "(paren x)")]
    #[case("(x,)", "(tuple x)")]
    #[case("(x, .., y)", "(tuple x .. y)")]
    #[case("a | b | c", "(| a b c)")]
    #[case("(a | b, c)", "(tuple (| a b) c)")]
    fn compound_pattern(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("..=")]
    #[case("'a'..=")]
    #[case("mut")]
    #[case("mut 1")]
    #[case("x in")]
    #[case("1 in x")]
    #[case("a |")]
    #[case("| a")]
    #[case("(x y)")]
    fn invalid_pattern(#[case] src: &str) {
        assert!(!parse_pattern(src).1.is_empty());
    }

    #[test]
    fn pattern_spans() {
        let pat = parse_pattern("(a, x in _)").0.unwrap();
        let PatKind::Tuple(pats) = pat.kind else {
            panic!("{pat:?}");
        };
        let spans: Vec<_> = pats.iter().map(|p| p.span.into_range()).collect();
        assert_eq!(spans, [1..2, 4..10]);
    }
}

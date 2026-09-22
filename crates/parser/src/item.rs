use crate::ast::{Block, Expr, FnSig, Item, ItemKind, Param, Pat, SelfParam, Span};
use crate::control::block_like;
use crate::error::Error;
use crate::parser::{Extra, expr, input, lex};
use crate::pattern::{ident, no_top, pattern};
use crate::stmt::block;
use crate::types::ty;
use chumsky::{input::ValueInput, prelude::*};
use gyokuto_lexer::Token;

/// モジュール (項目の列) を解析する
pub fn parse_module(src: &str) -> (Option<Vec<Item>>, Vec<Error>) {
    let (tokens, mut errors) = lex(src);
    let expr = expr(src);
    let block_like = block_like(src, expr.clone());
    let pattern = pattern(src, expr.clone(), block_like.clone());
    let block = block(src, expr.clone(), block_like.clone(), pattern.clone());
    let (items, parse_errors) = item(src, expr, block_like, block, pattern)
        .repeated()
        .collect()
        .then_ignore(end())
        .parse(input(&tokens, src))
        .into_output_errors();
    errors.extend(parse_errors);
    (items, errors)
}

/// 項目を解析する。関数の本体は `block`、引数の左辺は `pattern` を内側に持つパターンで解析する
pub(crate) fn item<'tok, 'src: 'tok, I>(
    src: &'src str,
    expr: impl Parser<'tok, I, Expr, Extra> + Clone + 'tok,
    block_like: impl Parser<'tok, I, Expr, Extra> + Clone + 'tok,
    block: impl Parser<'tok, I, Block, Extra> + Clone + 'tok,
    pattern: impl Parser<'tok, I, Pat, Extra> + Clone + 'tok,
) -> impl Parser<'tok, I, Item, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    let ty = ty(src, expr.clone(), block_like.clone());
    let mutable = just(Token::Mut).or_not().map(|m| m.is_some());
    let self_param = choice((
        just(Token::Amp)
            .ignore_then(mutable)
            .map(|mutable| SelfParam::Ref { mutable }),
        mutable.map(|mutable| SelfParam::Value { mutable }),
    ))
    .then_ignore(just(Token::SelfValue));
    let param = no_top(src, expr, block_like, pattern)
        .then_ignore(just(Token::Colon))
        .then(ty.clone())
        .map(|(pat, ty)| Param { pat, ty });
    let params = param
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>();
    let self_and_params = choice((
        self_param
            .map(Some)
            .then(
                just(Token::Comma)
                    .ignore_then(params.clone())
                    .or_not()
                    .map(Option::unwrap_or_default),
            )
            .then_ignore(just(Token::RParen).rewind()),
        params.map(|params| (None, params)),
    ))
    .delimited_by(just(Token::LParen), just(Token::RParen));
    let sig = just(Token::Fn)
        .ignore_then(ident(src))
        .then(self_and_params)
        .then(just(Token::Arrow).ignore_then(ty).or_not())
        .map(|((name, (self_param, params)), ret)| FnSig {
            name,
            self_param,
            params,
            ret,
        });
    sig.then(block).map_with(|(sig, body), e| Item {
        kind: ItemKind::Fn { sig, body },
        span: e.span(),
    })
}

#[cfg(test)]
mod tests {
    use crate::parse_module;
    use crate::sexpr::AsSexpr;
    use rstest::rstest;

    fn parse_ok(src: &str) -> String {
        let (items, errors) = parse_module(src);
        assert!(errors.is_empty(), "{src}: {errors:?}");
        let items: Vec<_> = items.unwrap().iter().map(AsSexpr::as_sexpr).collect();
        items.join(" ")
    }

    #[rstest]
    #[case("", "")]
    #[case("fn f() {}", "(fn f (params) (block))")]
    #[case("fn f() { a }", "(fn f (params) (block a))")]
    #[case(
        "fn f() {} fn g() {}",
        "(fn f (params) (block)) (fn g (params) (block))"
    )]
    fn fn_item(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("fn f(x: i32) {}", "(fn f (params (: x i32)) (block))")]
    #[case("fn f(x: A, y: B,) {}", "(fn f (params (: x A) (: y B)) (block))")]
    #[case("fn f(mut x: T) {}", "(fn f (params (: (mut x) T)) (block))")]
    #[case(
        "fn f((a, b): (A, B)) {}",
        "(fn f (params (: (tuple a b) (tuple A B))) (block))"
    )]
    #[case(
        "fn f(x in Some(_): T) {}",
        "(fn f (params (: (in x (Some _)) T)) (block))"
    )]
    fn fn_params(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("fn f(self) {}", "(fn f (params self) (block))")]
    #[case("fn f(mut self) {}", "(fn f (params (mut self)) (block))")]
    #[case("fn f(&self) {}", "(fn f (params (& self)) (block))")]
    #[case(
        "fn f(&mut self, x: T) {}",
        "(fn f (params (&mut self) (: x T)) (block))"
    )]
    #[case("fn f(self,) {}", "(fn f (params self) (block))")]
    #[case("fn f(self::A(x): T) {}", "(fn f (params (: (self::A x) T)) (block))")]
    fn fn_self_param(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("fn f() -> T { a }", "(fn f (params) (-> T) (block a))")]
    #[case("fn f() -> impl A + B {}", "(fn f (params) (-> (impl A B)) (block))")]
    fn fn_ret(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("fn f()")]
    #[case("fn f();")]
    #[case("fn () {}")]
    #[case("f() {}")]
    #[case("fn f(x) {}")]
    #[case("fn f(a | b: T) {}")]
    #[case("fn f(x: T, self) {}")]
    #[case("fn f(& &self) {}")]
    #[case("fn f() -> {}")]
    #[case("a")]
    fn invalid_fn(#[case] src: &str) {
        assert!(!parse_module(src).1.is_empty());
    }

    #[test]
    fn item_spans() {
        let items = parse_module("fn f() {}  fn g() {}").0.unwrap();
        let spans: Vec<_> = items.iter().map(|i| i.span.into_range()).collect();
        assert_eq!(spans, [0..9, 11..20]);
    }
}

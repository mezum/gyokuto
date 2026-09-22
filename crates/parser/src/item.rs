use crate::ast::{
    Block, Expr, FnSig, GenericParam, Item, ItemKind, Param, Pat, SelfParam, Span, WherePred,
};
use crate::control::block_like;
use crate::error::Error;
use crate::parser::{Extra, expr, input, lex};
use crate::pattern::{ident, no_top, pattern};
use crate::stmt::block;
use crate::types::{bounds, ty};
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
    let param = no_top(src, expr.clone(), block_like.clone(), pattern)
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
    let bounds = bounds(src, expr, block_like);
    let generic = choice((
        just(Token::Const)
            .ignore_then(ident(src))
            .then_ignore(just(Token::Colon))
            .then(ty.clone())
            .map(|(name, ty)| GenericParam::Const { name, ty }),
        ident(src)
            .then(
                just(Token::Colon)
                    .ignore_then(bounds.clone())
                    .or_not()
                    .map(Option::unwrap_or_default),
            )
            .map(|(name, bounds)| GenericParam::Type { name, bounds }),
    ));
    let generics = generic
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect()
        .delimited_by(just(Token::Lt), just(Token::Gt))
        .or_not()
        .map(Option::unwrap_or_default);
    let where_pred = ty
        .clone()
        .then_ignore(just(Token::Colon))
        .then(bounds)
        .map(|(ty, bounds)| WherePred { ty, bounds });
    let where_preds = just(Token::Where)
        .ignore_then(
            where_pred
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect(),
        )
        .or_not()
        .map(Option::unwrap_or_default);
    let sig = just(Token::Fn)
        .ignore_then(ident(src))
        .then(generics)
        .then(self_and_params)
        .then(just(Token::Arrow).ignore_then(ty).or_not())
        .then(where_preds)
        .map(
            |((((name, generics), (self_param, params)), ret), where_preds)| FnSig {
                name,
                generics,
                self_param,
                params,
                ret,
                where_preds,
            },
        );
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
    #[case("fn f<>() {}", "(fn f (params) (block))")]
    #[case("fn f<T>() {}", "(fn f (generics T) (params) (block))")]
    #[case("fn f<T: A>() {}", "(fn f (generics (: T A)) (params) (block))")]
    #[case(
        "fn f<T: A + B<C>, U,>() {}",
        "(fn f (generics (: T A B<C>) U) (params) (block))"
    )]
    #[case(
        "fn f<const N: usize>() {}",
        "(fn f (generics (const N usize)) (params) (block))"
    )]
    #[case(
        "fn f<const S: &str, T>() {}",
        "(fn f (generics (const S (& str)) T) (params) (block))"
    )]
    fn fn_generics(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("fn f() where {}", "(fn f (params) (block))")]
    #[case("fn f() where T: A {}", "(fn f (params) (where (: T A)) (block))")]
    #[case(
        "fn f() -> T where T: A + B, Vec<T>: C, {}",
        "(fn f (params) (-> T) (where (: T A B) (: Vec<T> C)) (block))"
    )]
    #[case(
        "fn f() where F: Fn(A) -> B {}",
        "(fn f (params) (where (: F Fn(A) -> B)) (block))"
    )]
    fn fn_where(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("fn f<T:>() {}")]
    #[case("fn f<const N>() {}")]
    #[case("fn f<1>() {}")]
    #[case("fn f<T() {}")]
    #[case("fn f() where T {}")]
    #[case("fn f() where T: {}")]
    fn invalid_generics(#[case] src: &str) {
        assert!(!parse_module(src).1.is_empty());
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

    #[rstest]
    #[case("extern fn f();", "(extern fn f (params))")]
    #[case("extern fn log(msg: &str);", "(extern fn log (params (: msg (& str))))")]
    #[case(
        "extern fn f<T>(x: T) -> T where T: A;",
        "(extern fn f (generics T) (params (: x T)) (-> T) (where (: T A)))"
    )]
    #[case(
        "extern fn f(); fn g() {}",
        "(extern fn f (params)) (fn g (params) (block))"
    )]
    fn extern_fn(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("extern fn f() {}")]
    #[case("extern fn f()")]
    #[case("extern f();")]
    #[case("extern;")]
    fn invalid_extern_fn(#[case] src: &str) {
        assert!(!parse_module(src).1.is_empty());
    }

    #[test]
    fn item_spans() {
        let items = parse_module("fn f() {}  fn g() {}").0.unwrap();
        let spans: Vec<_> = items.iter().map(|i| i.span.into_range()).collect();
        assert_eq!(spans, [0..9, 11..20]);
    }
}

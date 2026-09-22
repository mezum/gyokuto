use crate::ast::{Expr, ExprKind, Span};
use crate::parser::Extra;
use crate::stmt::block;
use chumsky::{input::ValueInput, prelude::*};
use gyokuto_lexer::Token;

/// ブロック様の式を解析する
pub(crate) fn block_like<'tok, 'src: 'tok, I>(
    src: &'src str,
    expr: impl Parser<'tok, I, Expr, Extra> + Clone + 'tok,
) -> impl Parser<'tok, I, Expr, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    recursive(move |block_like| {
        let block = block(src, expr, block_like);
        block.map_with(|block, e| Expr {
            kind: ExprKind::Block(block),
            span: e.span(),
        })
    })
}

#[cfg(test)]
mod tests {
    use crate::parse_expr;
    use crate::sexpr::AsSexpr;
    use rstest::rstest;

    fn parse_ok(src: &str) -> String {
        let (expr, errors) = parse_expr(src);
        assert!(errors.is_empty(), "{src}: {errors:?}");
        expr.unwrap().as_sexpr()
    }

    #[rstest]
    #[case("break", "(break)")]
    #[case("break a", "(break a)")]
    #[case("continue", "(continue)")]
    #[case("return", "(return)")]
    #[case("return a + b", "(return (Add a b))")]
    #[case("return a = b", "(return (= a b))")]
    #[case("break return a", "(break (return a))")]
    #[case("{ break; }", "(block (semi (break)))")]
    #[case("{ return }", "(block (return))")]
    #[case("f(return, a)", "(call f (return) a)")]
    #[case("(break) + a", "(Add (paren (break)) a)")]
    #[case("a = return", "(= a (return))")]
    fn jump(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("a + return")]
    #[case("-break")]
    #[case("return.a")]
    #[case("continue a")]
    fn jump_as_operand_is_error(#[case] src: &str) {
        assert!(!parse_expr(src).1.is_empty());
    }

    #[rstest]
    #[case("if a { b }", "(if a (block b))")]
    #[case("if a { b } else { c }", "(if a (block b) (block c))")]
    #[case(
        "if a { b } else if c { d } else { e }",
        "(if a (block b) (if c (block d) (block e)))"
    )]
    #[case("if a == b { c }", "(if (Eq a b) (block c))")]
    #[case("if a.. { b }", "(if (.. a _) (block b))")]
    #[case("if return { a }", "(if (return) (block a))")]
    #[case("if ({ a }) { b }", "(if (paren (block a)) (block b))")]
    #[case("if a[{ b }] { c }", "(if (index a (block b)) (block c))")]
    #[case("if f({ a }) { b }", "(if (call f (block a)) (block b))")]
    #[case("if (if a { b }) { c }", "(if (paren (if a (block b))) (block c))")]
    fn if_expr(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("if a { b }.c", "(field (if a (block b)) c)")]
    #[case("a + if b { c } else { d }", "(Add a (if b (block c) (block d)))")]
    #[case("{ if a { b } -c }", "(block (expr (if a (block b))) (Neg c))")]
    #[case("{ if a { b } }", "(block (if a (block b)))")]
    #[case(
        "{ if a { b } else { c } d }",
        "(block (expr (if a (block b) (block c))) d)"
    )]
    fn if_in_expr_and_stmt(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("if { a } { b }")]
    #[case("if a + { b } { c }")]
    #[case("if a = { b } { c }")]
    #[case("if if a { b } { c } { d }")]
    #[case("if a")]
    #[case("if a b")]
    #[case("if a { b } else")]
    #[case("if a { b } else c")]
    #[case("{ if a { b }; }")]
    fn invalid_if(#[case] src: &str) {
        assert!(!parse_expr(src).1.is_empty());
    }
}

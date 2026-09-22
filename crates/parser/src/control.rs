use crate::ast::{Arm, Expr, ExprKind, Span};
use crate::parser::{Extra, expr_with};
use crate::pattern::{no_top_in, pattern};
use crate::stmt::block;
use chumsky::{input::ValueInput, prelude::*};
use gyokuto_lexer::Token;

/// ブロック様の式を解析する。内側の式は `expr` で解析する
pub(crate) fn block_like<'tok, 'src: 'tok, I>(
    src: &'src str,
    expr: impl Parser<'tok, I, Expr, Extra> + Clone + 'tok,
) -> impl Parser<'tok, I, Expr, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    recursive(move |block_like| {
        let cond = expr_with(src, expr.clone(), block_like.clone(), false);
        let pattern = pattern(src, expr.clone(), block_like.clone());
        let arm = pattern
            .clone()
            .then(just(Token::If).ignore_then(expr.clone()).or_not())
            .then_ignore(just(Token::FatArrow))
            .then(expr.clone())
            .map_with(|((pat, guard), body), e| Arm {
                pat,
                guard,
                body,
                span: e.span(),
            });
        let match_expr = just(Token::Match)
            .ignore_then(cond.clone())
            .then(
                arm.separated_by(just(Token::Comma))
                    .allow_trailing()
                    .collect()
                    .delimited_by(just(Token::LBrace), just(Token::RBrace)),
            )
            .map_with(|(scrutinee, arms), e| Expr {
                kind: ExprKind::Match {
                    scrutinee: Box::new(scrutinee),
                    arms,
                },
                span: e.span(),
            });
        let for_pattern = no_top_in(src, expr.clone(), block_like.clone(), pattern.clone());
        let block = block(src, expr, block_like, pattern);
        let block_expr = block.clone().map_with(|block, e| Expr {
            kind: ExprKind::Block(block),
            span: e.span(),
        });
        let loop_expr = choice((
            just(Token::Loop)
                .ignore_then(block.clone())
                .map(ExprKind::Loop),
            just(Token::While)
                .ignore_then(cond.clone())
                .then(block.clone())
                .map(|(cond, body)| ExprKind::While {
                    cond: Box::new(cond),
                    body,
                }),
            just(Token::For)
                .ignore_then(for_pattern)
                .then_ignore(just(Token::In))
                .then(cond.clone())
                .then(block.clone())
                .map(|((pat, iter), body)| ExprKind::For {
                    pat: Box::new(pat),
                    iter: Box::new(iter),
                    body,
                }),
        ))
        .map_with(|kind, e| Expr {
            kind,
            span: e.span(),
        });
        let if_expr = recursive({
            let block_expr = block_expr.clone();
            move |if_expr| {
                just(Token::If)
                    .ignore_then(cond)
                    .then(block)
                    .then(
                        just(Token::Else)
                            .ignore_then(block_expr.or(if_expr))
                            .or_not(),
                    )
                    .map_with(|((cond, then), otherwise), e| Expr {
                        kind: ExprKind::If {
                            cond: Box::new(cond),
                            then,
                            otherwise: otherwise.map(Box::new),
                        },
                        span: e.span(),
                    })
            }
        });
        choice((block_expr, if_expr, loop_expr, match_expr))
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
    #[case("if a as B<{ N }> { c }", "(if (as a B<(block N)>) (block c))")]
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

    #[rstest]
    #[case("loop {}", "(loop (block))")]
    #[case("loop { break a }", "(loop (block (break a)))")]
    #[case(
        "while a < b { a += c; }",
        "(while (Lt a b) (block (semi (Add= a c))))"
    )]
    #[case("while ({ a }) {}", "(while (paren (block a)) (block))")]
    #[case("for i in xs { f(i); }", "(for i xs (block (semi (call f i))))")]
    #[case("for i in a.. { b }", "(for i (.. a _) (block b))")]
    #[case("for i in a..b {}", "(for i (.. a b) (block))")]
    #[case("for i in f({ a }) {}", "(for i (call f (block a)) (block))")]
    #[case("for (a, b) in xs {}", "(for (tuple a b) xs (block))")]
    #[case("for Some(x) in xs {}", "(for (Some x) xs (block))")]
    #[case(
        "for (x in Some(_)) in xs {}",
        "(for (paren (in x (Some _))) xs (block))"
    )]
    #[case("for (A | B) in xs {}", "(for (paren (| A B)) xs (block))")]
    fn loop_expr(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("{ loop {} a }", "(block (expr (loop (block))) a)")]
    #[case("{ while a {} -b }", "(block (expr (while a (block))) (Neg b))")]
    #[case("{ for i in a {} }", "(block (for i a (block)))")]
    #[case("a + loop { break b }", "(Add a (loop (block (break b))))")]
    fn loop_in_expr_and_stmt(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("loop")]
    #[case("loop a")]
    #[case("while { a } { b }")]
    #[case("while a")]
    #[case("for { a }")]
    #[case("for i { a }")]
    #[case("for i in { a } { b }")]
    #[case("for x in Some(_) in xs {}")]
    #[case("for A | B in xs {}")]
    #[case("for i in a")]
    #[case("{ loop {}; }")]
    fn invalid_loop(#[case] src: &str) {
        assert!(!parse_expr(src).1.is_empty());
    }

    #[rstest]
    #[case("match x {}", "(match x)")]
    #[case("match x { _ => a }", "(match x (=> _ a))")]
    #[case(
        "match x { Some(y) => y, None => z, }",
        "(match x (=> (Some y) y) (=> None z))"
    )]
    #[case(
        "match x { y if y > a => b, _ => c }",
        "(match x (=> y (if (Gt y a)) b) (=> _ c))"
    )]
    #[case(
        "match x { A | B => {}, _ => { a } }",
        "(match x (=> (| A B) (block)) (=> _ (block a)))"
    )]
    #[case(
        "match x { _ => if a { b } else { c }, }",
        "(match x (=> _ (if a (block b) (block c))))"
    )]
    #[case("match a + b { _ => c }", "(match (Add a b) (=> _ c))")]
    #[case("match ({ a }) { _ => b }", "(match (paren (block a)) (=> _ b))")]
    fn match_expr(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("{ match x { _ => a } b }", "(block (expr (match x (=> _ a))) b)")]
    #[case("{ match x { _ => a } }", "(block (match x (=> _ a)))")]
    #[case("a + match x { _ => b }", "(Add a (match x (=> _ b)))")]
    fn match_in_expr_and_stmt(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("match x")]
    #[case("match { a } { _ => b }")]
    #[case("match x { _ }")]
    #[case("match x { _ => a b }")]
    #[case("match x { _ => {} _ => b }")]
    #[case("match x { _ => a,, }")]
    #[case("match x { if a => b }")]
    #[case("{ match x { _ => a }; }")]
    fn invalid_match(#[case] src: &str) {
        assert!(!parse_expr(src).1.is_empty());
    }
}

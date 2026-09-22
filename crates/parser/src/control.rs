use crate::ast::{Arm, BinaryOp, Expr, ExprKind, Span};
use crate::error::{Error, ErrorKind};
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
        let check_lets = |allowed: bool| {
            cond.clone().validate(move |cond: Expr, _, emitter| {
                for span in misplaced_lets(&cond, allowed) {
                    emitter.emit(Error {
                        span,
                        kind: ErrorKind::MisplacedLet,
                    });
                }
                cond
            })
        };
        let let_cond = check_lets(true);
        let cond = check_lets(false);
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
                .ignore_then(let_cond.clone())
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
                    .ignore_then(let_cond)
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

/// 括弧の外にある直下の式
fn bare_operands(expr: &Expr) -> Vec<&Expr> {
    match &expr.kind {
        ExprKind::Call { callee: expr, .. }
        | ExprKind::MethodCall { receiver: expr, .. }
        | ExprKind::Field { expr, .. }
        | ExprKind::Index { expr, .. }
        | ExprKind::Try(expr)
        | ExprKind::Cast { expr, .. }
        | ExprKind::Unary { expr, .. }
        | ExprKind::Let { expr, .. } => vec![expr],
        ExprKind::Binary { lhs, rhs, .. }
        | ExprKind::Assign {
            place: lhs,
            value: rhs,
            ..
        } => vec![lhs, rhs],
        ExprKind::Range { start, end, .. } => start.iter().chain(end).map(|e| &**e).collect(),
        ExprKind::Break(value) | ExprKind::Return(value) => value.iter().map(|e| &**e).collect(),
        ExprKind::Lit(_)
        | ExprKind::Path(_)
        | ExprKind::Paren(_)
        | ExprKind::Tuple(_)
        | ExprKind::Array(_)
        | ExprKind::Repeat { .. }
        | ExprKind::Block(_)
        | ExprKind::If { .. }
        | ExprKind::Loop(_)
        | ExprKind::While { .. }
        | ExprKind::For { .. }
        | ExprKind::Match { .. }
        | ExprKind::Continue
        | ExprKind::Error => Vec::new(),
    }
}

/// 括弧の外にブロック様の式を含むか
pub(crate) fn has_bare_block_like(expr: &Expr) -> bool {
    matches!(
        expr.kind,
        ExprKind::Block(_)
            | ExprKind::If { .. }
            | ExprKind::Loop(_)
            | ExprKind::While { .. }
            | ExprKind::For { .. }
            | ExprKind::Match { .. }
    ) || bare_operands(expr).into_iter().any(has_bare_block_like)
}

/// `&&` で連ねた最上位のオペランド以外にある `let` の位置。`allowed` が偽の場合は最上位でも使えない
fn misplaced_lets(expr: &Expr, allowed: bool) -> Vec<Span> {
    match &expr.kind {
        ExprKind::Let {
            expr: scrutinee, ..
        } if allowed => misplaced_lets(scrutinee, false),
        ExprKind::Let { .. } => vec![expr.span],
        ExprKind::Binary {
            op: BinaryOp::And,
            lhs,
            rhs,
        } => [lhs, rhs]
            .into_iter()
            .flat_map(|operand| misplaced_lets(operand, allowed))
            .collect(),
        _ => bare_operands(expr)
            .into_iter()
            .flat_map(|operand| misplaced_lets(operand, false))
            .collect(),
    }
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

    #[rstest]
    #[case("if let Some(x) = a { b }", "(if (let (Some x) a) (block b))")]
    #[case(
        "if let Some(x) = a && x > b { c }",
        "(if (And (let (Some x) a) (Gt x b)) (block c))"
    )]
    #[case(
        "if a && let Some(x) = b { c }",
        "(if (And a (let (Some x) b)) (block c))"
    )]
    #[case(
        "if let A = a && let B = b && c { d }",
        "(if (And (And (let A a) (let B b)) c) (block d))"
    )]
    #[case("if let A = a == b { c }", "(if (let A (Eq a b)) (block c))")]
    #[case("if let A = (a || b) { c }", "(if (let A (paren (Or a b))) (block c))")]
    #[case(
        "if let A = a { b } else if let B = c { d }",
        "(if (let A a) (block b) (if (let B c) (block d)))"
    )]
    #[case(
        "while let Some(x) = it.next() { f(x); }",
        "(while (let (Some x) (method it next)) (block (semi (call f x))))"
    )]
    fn let_condition(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("if let A = a || b {}")]
    #[case("if a || let A = b {}")]
    #[case("if !let A = a {}")]
    #[case("if (let A = a) {}")]
    #[case("if let A = { a } {}")]
    #[case("if let A = a..b {}")]
    #[case("if x = let A = a {}")]
    #[case("if return let A = a {}")]
    #[case("let A = a")]
    #[case("a && let A = b")]
    #[case("match let A = a { _ => b }")]
    #[case("for x in let A = a {}")]
    #[case("{ let x = let A = a; }")]
    fn invalid_let_condition(#[case] src: &str) {
        assert!(!parse_expr(src).1.is_empty());
    }
}

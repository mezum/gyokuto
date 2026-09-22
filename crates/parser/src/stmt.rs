use crate::ast::{Block, Expr, Span, Stmt, StmtKind};
use crate::parser::Extra;
use crate::types::ty;
use chumsky::{input::ValueInput, prelude::*};
use gyokuto_lexer::Token;

/// ブロック `{ ... }` を解析する。文の先頭のブロック様の式には `block_like` を使う
pub(crate) fn block<'tok, 'src: 'tok, I>(
    src: &'src str,
    expr: impl Parser<'tok, I, Expr, Extra> + Clone + 'tok,
    block_like: impl Parser<'tok, I, Expr, Extra> + Clone + 'tok,
) -> impl Parser<'tok, I, Block, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    let name = just(Token::Ident)
        .to_span()
        .map(move |span: Span| src[span.into_range()].to_string());
    let let_stmt = just(Token::Let)
        .ignore_then(just(Token::Mut).or_not().map(|m| m.is_some()))
        .then(name)
        .then(
            just(Token::Colon)
                .ignore_then(ty(src, expr.clone(), block_like.clone()))
                .or_not(),
        )
        .then(just(Token::Eq).ignore_then(expr.clone()).or_not())
        .then_ignore(just(Token::Semi))
        .map(|(((mutable, name), ty), init)| StmtKind::Let {
            mutable,
            name,
            ty,
            init,
        });
    let stmt = choice((
        let_stmt,
        block_like.map(StmtKind::Expr),
        expr.then(choice((
            just(Token::Semi).to(StmtKind::Semi as fn(Expr) -> StmtKind),
            just(Token::RBrace)
                .rewind()
                .to(StmtKind::Expr as fn(Expr) -> StmtKind),
        )))
        .map(|(expr, kind)| kind(expr)),
    ))
    .map_with(|kind, e| Stmt {
        kind,
        span: e.span(),
    });
    stmt.repeated()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::LBrace), just(Token::RBrace))
        .map(|mut stmts| {
            let expr = match stmts.pop() {
                Some(Stmt {
                    kind: StmtKind::Expr(expr),
                    ..
                }) => Some(expr),
                last => {
                    stmts.extend(last);
                    None
                }
            };
            Block {
                stmts,
                expr: expr.map(Box::new),
            }
        })
}

#[cfg(test)]
mod tests {
    use crate::ast::ExprKind;
    use crate::parse_expr;
    use crate::sexpr::AsSexpr;
    use rstest::rstest;

    fn parse_ok(src: &str) -> String {
        let (expr, errors) = parse_expr(src);
        assert!(errors.is_empty(), "{src}: {errors:?}");
        expr.unwrap().as_sexpr()
    }

    #[rstest]
    #[case("{}", "(block)")]
    #[case("{ a }", "(block a)")]
    #[case("{ a; }", "(block (semi a))")]
    #[case("{ a; b }", "(block (semi a) b)")]
    #[case("{ a = b; c += d; }", "(block (semi (= a b)) (semi (Add= c d)))")]
    fn block(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("{ let x; }", "(block (let x))")]
    #[case("{ let mut x; }", "(block (let (mut x)))")]
    #[case("{ let x: T; }", "(block (let x (: T)))")]
    #[case("{ let x = a; }", "(block (let x (= a)))")]
    #[case(
        "{ let mut x: Vec<T> = a + b; }",
        "(block (let (mut x) (: Vec<T>) (= (Add a b))))"
    )]
    #[case("{ let x = a; x }", "(block (let x (= a)) x)")]
    fn let_stmt(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("{ let (a, b) = c; }", "(block (let (tuple a b) (= c)))")]
    #[case("{ let A(x) | B(x) = a; }", "(block (let (| (A x) (B x)) (= a)))")]
    #[case("{ let x in Some(_) = a; }", "(block (let (in x (Some _)) (= a)))")]
    #[case("{ let x = a + { b }; }", "(block (let x (= (Add a (block b)))))")]
    fn let_pattern(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case(
        "{ let Some(x) = a else { return }; }",
        "(block (let (Some x) (= a) (else (block (return)))))"
    )]
    #[case(
        "{ let x: T = a.b() else { return }; }",
        "(block (let x (: T) (= (method a b)) (else (block (return)))))"
    )]
    #[case(
        "{ let Some(x) = (if a { b } else { c }) else { return }; }",
        "(block (let (Some x) (= (paren (if a (block b) (block c)))) (else (block (return)))))"
    )]
    #[case(
        "{ let Some(x) = f({ a }) else { return }; }",
        "(block (let (Some x) (= (call f (block a))) (else (block (return)))))"
    )]
    fn let_else(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("{ let Some(x) = if a { b } else { c } else { return }; }")]
    #[case("{ let x = { a } else { return }; }")]
    #[case("{ let x = a + { b } else { return }; }")]
    #[case("{ let x = -loop {} else { return }; }")]
    #[case("{ let x = { a }.b else { return }; }")]
    #[case("{ let x else { return }; }")]
    #[case("{ let x = a else b; }")]
    #[case("{ let x = a else { return } }")]
    fn invalid_let_else(#[case] src: &str) {
        assert!(!parse_expr(src).1.is_empty());
    }

    #[rstest]
    #[case("{ {} a }", "(block (expr (block)) a)")]
    #[case("{ { a } -b }", "(block (expr (block a)) (Neg b))")]
    #[case("{ { a } { b } }", "(block (expr (block a)) (block b))")]
    #[case("{ { a } }", "(block (block a))")]
    #[case("{ { a } b; }", "(block (expr (block a)) (semi b))")]
    fn block_like_stmt(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("a + { b }", "(Add a (block b))")]
    #[case("({ a } - b)", "(paren (Sub (block a) b))")]
    #[case("{{ a }}", "(block (block a))")]
    fn block_in_expr(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("{ ; }")]
    #[case("{ {}; }")]
    #[case("{ a; ; }")]
    #[case("{ a b }")]
    #[case("{ let x }")]
    #[case("{ let x = a }")]
    #[case("{ let x: = a; }")]
    #[case("{ let mut; }")]
    #[case("{ a")]
    fn invalid_block(#[case] src: &str) {
        assert!(!parse_expr(src).1.is_empty());
    }

    #[test]
    fn recovers_inside_block() {
        let (expr, errors) = parse_expr("[{ a b }, c]");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(expr.unwrap().as_sexpr(), "(array error c)");
    }

    #[test]
    fn deeply_nested_tail_blocks() {
        let src = format!("{}a{}", "{ (".repeat(32), ") }".repeat(32));
        assert!(parse_expr(&src).1.is_empty());
    }

    #[test]
    fn stmt_spans() {
        let expr = parse_expr("{ let x = a; b; {} c }").0.unwrap();
        let ExprKind::Block(block) = expr.kind else {
            panic!("{expr:?}");
        };
        let spans: Vec<_> = block.stmts.iter().map(|s| s.span.into_range()).collect();
        assert_eq!(spans, [2..12, 13..15, 16..18]);
    }
}

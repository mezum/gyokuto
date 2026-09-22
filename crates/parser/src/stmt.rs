use crate::ast::{Block, Expr, ExprKind, Span, Stmt, StmtKind};
use crate::parser::Extra;
use crate::types::ty;
use chumsky::{input::ValueInput, prelude::*};
use gyokuto_lexer::Token;

/// ブロック `{ ... }` を解析する
pub(crate) fn block<'tok, 'src: 'tok, I>(
    src: &'src str,
    expr: impl Parser<'tok, I, Expr, Extra> + Clone + 'tok,
) -> impl Parser<'tok, I, Block, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    recursive(move |block| {
        let block_like = block.map_with(|block, e| Expr {
            kind: ExprKind::Block(block),
            span: e.span(),
        });
        let name = just(Token::Ident)
            .to_span()
            .map(move |span: Span| src[span.into_range()].to_string());
        let let_stmt = just(Token::Let)
            .ignore_then(just(Token::Mut).or_not().map(|m| m.is_some()))
            .then(name)
            .then(
                just(Token::Colon)
                    .ignore_then(ty(src, expr.clone()))
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
            expr.clone()
                .then_ignore(just(Token::Semi))
                .map(StmtKind::Semi),
        ))
        .map_with(|kind, e| Stmt {
            kind,
            span: e.span(),
        });
        stmt.repeated()
            .collect::<Vec<_>>()
            .then(expr.or_not())
            .delimited_by(just(Token::LBrace), just(Token::RBrace))
            .map(|(mut stmts, expr)| {
                let expr = expr.or_else(|| match stmts.pop() {
                    Some(Stmt {
                        kind: StmtKind::Expr(expr),
                        ..
                    }) => Some(expr),
                    last => {
                        stmts.extend(last);
                        None
                    }
                });
                Block {
                    stmts,
                    expr: expr.map(Box::new),
                }
            })
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
    #[case("{ let mut x; }", "(block (let mut x))")]
    #[case("{ let x: T; }", "(block (let x (: T)))")]
    #[case("{ let x = a; }", "(block (let x (= a)))")]
    #[case(
        "{ let mut x: Vec<T> = a + b; }",
        "(block (let mut x (: Vec<T>) (= (Add a b))))"
    )]
    #[case("{ let x = a; x }", "(block (let x (= a)) x)")]
    fn let_stmt(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
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
    #[case("{ let 1; }")]
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
    fn stmt_spans() {
        let expr = parse_expr("{ let x = a; b; {} c }").0.unwrap();
        let ExprKind::Block(block) = expr.kind else {
            panic!("{expr:?}");
        };
        let spans: Vec<_> = block.stmts.iter().map(|s| s.span.into_range()).collect();
        assert_eq!(spans, [2..12, 13..15, 16..18]);
    }
}

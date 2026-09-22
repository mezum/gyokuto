use crate::ast::{
    BinaryOp, Expr, ExprKind, Field, GenericArgs, Lit, Path, PathName, PathSegment, Span, UnaryOp,
};
use crate::control::block_like;
use crate::error::{Error, ErrorKind};
use crate::literal;
use crate::types::{angle_args, ty, ty_no_bounds};
use chumsky::pratt::{Associativity, Operator, infix, left, none, postfix, prefix};
use chumsky::{input::ValueInput, prelude::*};
use gyokuto_lexer::Token;
use logos::Logos;
use std::iter::once;

pub(crate) type Extra = extra::Err<Error>;

/// 式を解析する
pub fn parse_expr(src: &str) -> (Option<Expr>, Vec<Error>) {
    let (tokens, mut errors) = lex(src);
    let (expr, parse_errors) = expr(src)
        .then_ignore(end())
        .parse(input(&tokens, src))
        .into_output_errors();
    errors.extend(parse_errors);
    (expr, errors)
}

/// 字句解析し、エラーのトークンを `Token::Error` に置き換えたトークン列と字句解析のエラーを得る
///
/// 型引数を閉じる `>` を取り出せるように、`>` で始まるトークンは 1 文字ずつに分割する。
/// 演算子としては、隙間なく並んだ分割後のトークンを [`glued`] で 1 つにまとめて解析する。
pub(crate) fn lex(src: &str) -> (Vec<(Token, Span)>, Vec<Error>) {
    let lexed: Vec<_> = Token::lexer(src)
        .spanned()
        .map(|(token, span)| (token, Span::from(span)))
        .collect();
    let tokens: Vec<(Token, Span)> = lexed
        .iter()
        .flat_map(|&(token, span)| {
            let parts = match token {
                Ok(Token::Shr) => vec![Token::Gt, Token::Gt],
                Ok(Token::Ge) => vec![Token::Gt, Token::Eq],
                Ok(Token::ShrEq) => vec![Token::Gt, Token::Gt, Token::Eq],
                _ => return vec![(token.unwrap_or(Token::Error), span)],
            };
            parts
                .into_iter()
                .enumerate()
                .map(move |(i, part)| (part, Span::from(span.start + i..span.start + i + 1)))
                .collect()
        })
        .collect();
    let errors = lexed
        .iter()
        .filter_map(|&(token, span)| {
            token.err().map(|error| Error {
                span,
                kind: error.into(),
            })
        })
        .collect();
    (tokens, errors)
}

/// トークン列を chumsky の入力にする
pub(crate) fn input<'tok>(
    tokens: &'tok [(Token, Span)],
    src: &str,
) -> impl ValueInput<'tok, Token = Token, Span = Span> {
    let eoi = Span::from(src.len()..src.len());
    tokens.map(eoi, |(token, span)| (token, span))
}

pub(crate) fn expr<'tok, 'src: 'tok, I>(src: &'src str) -> impl Parser<'tok, I, Expr, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    recursive(|expr| {
        let block_like = block_like(src, expr.clone()).boxed();
        expr_with(src, expr, Some(block_like))
    })
}

/// 式を解析する。括弧の内側の式には `expr` を使う
///
/// `block_like` が無い場合は、括弧の外にブロック様の式を含まない条件式を解析する
fn expr_with<'tok, 'src: 'tok, I>(
    src: &'src str,
    expr: impl Parser<'tok, I, Expr, Extra> + Clone + 'tok,
    block_like: Option<Boxed<'tok, 'tok, I, Expr, Extra>>,
) -> impl Parser<'tok, I, Expr, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    recursive(move |this| {
        let turbofish = just(Token::ColonColon).ignore_then(angle_args(src, ty(src, expr.clone())));
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
        let atom = choice((
            literal(src),
            path(src, turbofish.clone().or_not()).map(ExprKind::Path),
            parens,
            array,
            just(Token::Error).to(ExprKind::Error),
        ))
        .map_with(|kind, e| Expr {
            kind,
            span: e.span(),
        });
        let atom = match block_like {
            Some(block_like) => choice((atom, block_like))
                .recover_with(via_parser(nested_delimiters(
                    Token::LBrace,
                    Token::RBrace,
                    [
                        (Token::LParen, Token::RParen),
                        (Token::LBracket, Token::RBracket),
                    ],
                    error,
                )))
                .boxed(),
            None => atom.boxed(),
        }
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
        )));

        #[derive(Clone)]
        enum PostfixOp {
            Call(Vec<Expr>),
            Method(String, Option<GenericArgs>, Vec<Expr>),
            Field(Field),
            /// `t.0.1` のようにまとめて字句解析されたタプルのフィールドと、各フィールドの終端の位置
            TupleFields(Vec<(usize, usize)>),
            Index(Expr),
            Try,
        }
        let args = expr
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LParen), just(Token::RParen));
        let ident = just(Token::Ident)
            .to_span()
            .map(move |span: Span| src[span.into_range()].to_string());
        let postfix_op = postfix(
            13,
            choice((
                args.clone().map(PostfixOp::Call),
                just(Token::Dot)
                    .ignore_then(ident)
                    .then(turbofish.clone().or_not())
                    .then(args)
                    .map(|((method, generics), args)| PostfixOp::Method(method, generics, args)),
                just(Token::Dot)
                    .ignore_then(ident)
                    .map(|name| PostfixOp::Field(Field::Named(name))),
                just(Token::Dot)
                    .ignore_then(one_of([Token::Int, Token::Float]).to_span())
                    .validate(move |span: Span, _, emitter| {
                        tuple_indices(&src[span.into_range()], span.start).unwrap_or_else(|| {
                            emitter.emit(Error {
                                span,
                                kind: ErrorKind::InvalidTupleIndex,
                            });
                            Vec::new()
                        })
                    })
                    .map(PostfixOp::TupleFields),
                expr.clone()
                    .delimited_by(just(Token::LBracket), just(Token::RBracket))
                    .map(PostfixOp::Index),
                just(Token::Question).to(PostfixOp::Try),
            )),
            |lhs: Expr, op, e| {
                let start = lhs.span.start;
                let span = e.span();
                let wrap = |kind| Expr { kind, span };
                let lhs = Box::new(lhs);
                match op {
                    PostfixOp::Call(args) => wrap(ExprKind::Call { callee: lhs, args }),
                    PostfixOp::Method(method, generics, args) => wrap(ExprKind::MethodCall {
                        receiver: lhs,
                        method,
                        generics,
                        args,
                    }),
                    PostfixOp::Field(field) => wrap(ExprKind::Field { expr: lhs, field }),
                    PostfixOp::TupleFields(fields) => {
                        fields.into_iter().fold(*lhs, |expr, (index, end)| Expr {
                            kind: ExprKind::Field {
                                expr: Box::new(expr),
                                field: Field::Index(index),
                            },
                            span: Span::from(start..end),
                        })
                    }
                    PostfixOp::Index(index) => wrap(ExprKind::Index {
                        expr: lhs,
                        index: Box::new(index),
                    }),
                    PostfixOp::Try => wrap(ExprKind::Try(lhs)),
                }
            },
        );
        let cast = postfix(
            11,
            just(Token::As).ignore_then(ty_no_bounds(src, expr.clone())),
            |expr, ty, e| Expr {
                kind: ExprKind::Cast {
                    expr: Box::new(expr),
                    ty: Box::new(ty),
                },
                span: e.span(),
            },
        );
        let unary = prefix(
            12,
            choice((
                just(Token::Amp).then(just(Token::Mut)).to(UnaryOp::RefMut),
                select! {
                    Token::Minus => UnaryOp::Neg,
                    Token::Bang => UnaryOp::Not,
                    Token::Star => UnaryOp::Deref,
                    Token::Amp => UnaryOp::Ref,
                },
            )),
            |op, expr, e| Expr {
                kind: ExprKind::Unary {
                    op,
                    expr: Box::new(expr),
                },
                span: e.span(),
            },
        );
        let ops = atom.pratt((
            postfix_op,
            unary,
            cast,
            binary(
                left(10),
                select! {
                    Token::Star => BinaryOp::Mul,
                    Token::Slash => BinaryOp::Div,
                    Token::Percent => BinaryOp::Rem,
                },
            ),
            binary(
                left(9),
                select! {
                    Token::Plus => BinaryOp::Add,
                    Token::Minus => BinaryOp::Sub,
                },
            ),
            binary(
                left(8),
                select! {
                    Token::Shl => BinaryOp::Shl,
                }
                .or(glued(&[Token::Gt, Token::Gt]).to(BinaryOp::Shr)),
            ),
            binary(left(7), just(Token::Amp).to(BinaryOp::BitAnd)),
            binary(left(6), just(Token::Caret).to(BinaryOp::BitXor)),
            binary(left(5), just(Token::Pipe).to(BinaryOp::BitOr)),
            binary(
                none(4),
                select! {
                    Token::EqEq => BinaryOp::Eq,
                    Token::Ne => BinaryOp::Ne,
                    Token::Lt => BinaryOp::Lt,
                    Token::Le => BinaryOp::Le,
                }
                .or(glued(&[Token::Gt]).to(BinaryOp::Gt))
                .or(glued(&[Token::Gt, Token::Eq]).to(BinaryOp::Ge)),
            ),
            binary(left(3), just(Token::AndAnd).to(BinaryOp::And)),
            binary(left(2), just(Token::OrOr).to(BinaryOp::Or)),
        ));

        let range_tail = choice((
            just(Token::DotDotEq)
                .ignore_then(ops.clone())
                .map(|end| (true, Some(end))),
            just(Token::DotDot)
                .ignore_then(ops.clone().or_not())
                .map(|end| (false, end)),
        ));
        let range = |start: Option<Expr>, (inclusive, end): (bool, Option<Expr>)| ExprKind::Range {
            start: start.map(Box::new),
            end: end.map(Box::new),
            inclusive,
        };
        let range = choice((
            ops.then(range_tail.clone().or_not())
                .map_with(move |(start, tail), e| match tail {
                    None => start,
                    Some(tail) => Expr {
                        kind: range(Some(start), tail),
                        span: e.span(),
                    },
                }),
            range_tail.map_with(move |tail, e| Expr {
                kind: range(None, tail),
                span: e.span(),
            }),
        ));

        let assign_op = select! {
            Token::Eq => None,
            Token::PlusEq => Some(BinaryOp::Add),
            Token::MinusEq => Some(BinaryOp::Sub),
            Token::StarEq => Some(BinaryOp::Mul),
            Token::SlashEq => Some(BinaryOp::Div),
            Token::PercentEq => Some(BinaryOp::Rem),
            Token::AmpEq => Some(BinaryOp::BitAnd),
            Token::PipeEq => Some(BinaryOp::BitOr),
            Token::CaretEq => Some(BinaryOp::BitXor),
            Token::ShlEq => Some(BinaryOp::Shl),
        }
        .or(glued(&[Token::Gt, Token::Gt, Token::Eq]).to(Some(BinaryOp::Shr)));
        range
            .then(assign_op.then(this).or_not())
            .map_with(|(place, assign), e| match assign {
                None => place,
                Some((op, value)) => Expr {
                    kind: ExprKind::Assign {
                        op,
                        place: Box::new(place),
                        value: Box::new(value),
                    },
                    span: e.span(),
                },
            })
    })
}

/// タプルのフィールドの並び `0` / `0.1` を、各フィールドの値と終端の位置に分ける
///
/// フィールドは `_`・サフィックス・先頭の `0` などを含まない 10 進数に限る
fn tuple_indices(text: &str, offset: usize) -> Option<Vec<(usize, usize)>> {
    text.split('.')
        .scan(offset, |start, digits| {
            let end = *start + digits.len();
            *start = end + 1;
            Some((digits, end))
        })
        .map(|(digits, end)| {
            let decimal = digits.bytes().all(|b| b.is_ascii_digit());
            let canonical = digits == "0" || !digits.starts_with('0');
            let index = digits.parse().ok().filter(|_| decimal && canonical)?;
            Some((index, end))
        })
        .collect()
}

/// 分割された `>` で始まるトークンが、隙間なく `tokens` の通りに並んでいるものを 1 つの演算子として解析する
fn glued<'tok, I>(tokens: &'static [Token]) -> impl Parser<'tok, I, (), Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    let spanned = any().map_with(|token, e| (token, e.span()));
    spanned
        .repeated()
        .exactly(tokens.len())
        .collect::<Vec<(Token, Span)>>()
        .then(spanned.rewind().or_not())
        .filter(move |(parts, next)| {
            let joined = |a: &Span, b: &Span| a.end == b.start;
            parts.iter().map(|(token, _)| token).eq(tokens)
                && parts.windows(2).all(|w| joined(&w[0].1, &w[1].1))
                && !matches!(
                    (parts.last(), next),
                    (Some((_, last)), Some((Token::Gt | Token::Eq, span))) if joined(last, span)
                )
        })
        .ignored()
}

fn binary<'tok, I>(
    associativity: Associativity,
    op: impl Parser<'tok, I, BinaryOp, Extra> + Clone,
) -> impl Operator<'tok, I, Expr, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    infix(associativity, op, |lhs, op, rhs, e| Expr {
        kind: ExprKind::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        span: e.span(),
    })
}

/// 真偽値とリテラルのトークンを解析し、値に変換する
pub(crate) fn literal<'tok, 'src: 'tok, I>(
    src: &'src str,
) -> impl Parser<'tok, I, ExprKind, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
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
            .unwrap_or_else(|error| {
                emitter.emit(Error {
                    span,
                    kind: error.into(),
                });
                ExprKind::Error
            })
    });
    choice((bool_lit, lit))
}

/// パスを解析する。各セグメントの後には型引数として `args` を続けて解析する
pub(crate) fn path<'tok, 'src: 'tok, I>(
    src: &'src str,
    args: impl Parser<'tok, I, Option<GenericArgs>, Extra> + Clone,
) -> impl Parser<'tok, I, Path, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    let segment = |(name, args)| PathSegment { name, args };
    let ident = just(Token::Ident)
        .to_span()
        .map(move |span: Span| PathName::Ident(src[span.into_range()].to_string()));
    let head = choice((
        just(Token::Super)
            .to(PathName::Super)
            .then(args.clone())
            .map(segment)
            .separated_by(just(Token::ColonColon))
            .at_least(1)
            .collect(),
        choice((
            just(Token::Crate).to(PathName::Crate),
            just(Token::SelfValue).to(PathName::SelfValue),
            just(Token::SelfType).to(PathName::SelfType),
            ident,
        ))
        .then(args.clone())
        .map(move |name_args| vec![segment(name_args)]),
    ));
    let rest = just(Token::ColonColon)
        .ignore_then(ident.then(args).map(segment))
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
    use crate::error::{ErrorKind, Expected, Found, LiteralError};
    use crate::sexpr::AsSexpr;
    use gyokuto_lexer::LexError;
    use rstest::rstest;

    fn parse_ok(src: &str) -> String {
        let (expr, errors) = parse_expr(src);
        assert!(errors.is_empty(), "{src}: {errors:?}");
        expr.unwrap().as_sexpr()
    }

    #[rstest]
    #[case("true", "true")]
    #[case("false", "false")]
    fn bool_literal(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[test]
    fn literals_in_expressions() {
        assert_eq!(
            parse_ok(r#"(1u8, 2.5, 'a', "s")"#),
            r#"(tuple Int { value: 1, suffix: Some(U8) } Float { digits: "2.5", suffix: None } Char('a') Str("s"))"#
        );
    }

    #[rstest]
    #[case("'e\u{301}'")]
    #[case("'👨\u{200D}👩'")]
    #[case("'🇯🇵'")]
    fn multi_scalar_grapheme_is_not_char(#[case] src: &str) {
        assert!(!parse_expr(src).1.is_empty());
    }

    #[test]
    fn too_large_integer_is_error() {
        let (expr, errors) = parse_expr("[18446744073709551616, a]");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].span.into_range(), 1..21);
        assert_eq!(
            errors[0].kind,
            ErrorKind::Literal(LiteralError::IntegerTooLarge)
        );
        assert_eq!(expr.unwrap().as_sexpr(), "(array error a)");
    }

    #[rstest]
    #[case("a")]
    #[case("a::b::c")]
    #[case("crate::a")]
    #[case("self")]
    #[case("self::a")]
    #[case("Self::new")]
    #[case("super::a")]
    #[case("super::super::a")]
    fn path(#[case] src: &str) {
        assert_eq!(parse_ok(src), src);
    }

    #[rstest]
    #[case("a::crate")]
    #[case("a::self")]
    #[case("a::super")]
    #[case("super::a::super")]
    #[case("::a")]
    #[case("a::")]
    fn invalid_path(#[case] src: &str) {
        assert!(!parse_expr(src).1.is_empty());
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
        assert_eq!(errors[0].span.into_range(), 0..1);
        assert_eq!(errors[0].kind, ErrorKind::Lex(LexError::UnexpectedChar));
        assert_eq!(expr.unwrap().as_sexpr(), "error");
    }

    #[rstest]
    #[case("(a)", "(paren a)")]
    #[case("()", "(tuple)")]
    #[case("(a,)", "(tuple a)")]
    #[case("(a, b)", "(tuple a b)")]
    #[case("(a, b,)", "(tuple a b)")]
    #[case("((a))", "(paren (paren a))")]
    fn paren_or_tuple(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("[]", "(array)")]
    #[case("[a, b,]", "(array a b)")]
    #[case("[a; n]", "(repeat a n)")]
    #[case("[(a, b), [c]]", "(array (tuple a b) (array c))")]
    fn array(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("(a,,)")]
    #[case("(,)")]
    #[case("[a,,]")]
    #[case("[,]")]
    #[case("[a; n,]")]
    #[case("[a, b; n]")]
    fn invalid_separator(#[case] src: &str) {
        assert!(!parse_expr(src).1.is_empty());
    }

    #[test]
    fn deeply_nested_parens() {
        let src = format!("{}a{}", "(".repeat(64), ")".repeat(64));
        assert!(parse_expr(&src).1.is_empty());
    }

    #[test]
    fn deeply_nested_arrays() {
        let src = format!("{}a{}", "[".repeat(64), "]".repeat(64));
        assert!(parse_expr(&src).1.is_empty());
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
        assert_eq!(errors[0].span.into_range(), 4..5);
        assert!(
            matches!(
                &errors[0].kind,
                ErrorKind::Syntax { found: Found::Token(Token::Ident), expected }
                    if expected.contains(&Expected::Token(Token::RParen))
            ),
            "{errors:?}"
        );
        assert_eq!(expr.unwrap().as_sexpr(), "(array error c)");
    }

    #[test]
    fn lexical_error_inside_delimiters() {
        let (expr, errors) = parse_expr("[a, $]");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(expr.unwrap().as_sexpr(), "(array a error)");
    }

    #[rstest]
    #[case("-a", "(Neg a)")]
    #[case("!a", "(Not a)")]
    #[case("*a", "(Deref a)")]
    #[case("&a", "(Ref a)")]
    #[case("&mut a", "(RefMut a)")]
    #[case("-!*a", "(Neg (Not (Deref a)))")]
    fn unary_operator(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[test]
    fn double_ref_is_error() {
        assert!(!parse_expr("&&a").1.is_empty());
    }

    #[test]
    fn negative_literal_is_unary() {
        assert_eq!(
            parse_ok("-128i8"),
            "(Neg Int { value: 128, suffix: Some(I8) })"
        );
    }

    #[rstest]
    #[case("a * b + c", "(Add (Mul a b) c)")]
    #[case("a + b * c", "(Add a (Mul b c))")]
    #[case("a + b << c", "(Shl (Add a b) c)")]
    #[case("a << b & c", "(BitAnd (Shl a b) c)")]
    #[case("a & b ^ c", "(BitXor (BitAnd a b) c)")]
    #[case("a ^ b | c", "(BitOr (BitXor a b) c)")]
    #[case("a | b == c", "(Eq (BitOr a b) c)")]
    #[case("a == b && c", "(And (Eq a b) c)")]
    #[case("a && b || c", "(Or (And a b) c)")]
    #[case("-a * b", "(Mul (Neg a) b)")]
    #[case("(a + b) * c", "(Mul (paren (Add a b)) c)")]
    fn binary_precedence(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("a * b", "Mul")]
    #[case("a / b", "Div")]
    #[case("a % b", "Rem")]
    #[case("a + b", "Add")]
    #[case("a - b", "Sub")]
    #[case("a << b", "Shl")]
    #[case("a >> b", "Shr")]
    #[case("a & b", "BitAnd")]
    #[case("a ^ b", "BitXor")]
    #[case("a | b", "BitOr")]
    #[case("a == b", "Eq")]
    #[case("a != b", "Ne")]
    #[case("a < b", "Lt")]
    #[case("a > b", "Gt")]
    #[case("a <= b", "Le")]
    #[case("a >= b", "Ge")]
    #[case("a && b", "And")]
    #[case("a || b", "Or")]
    fn binary_operator(#[case] src: &str, #[case] op: &str) {
        assert_eq!(parse_ok(src), format!("({op} a b)"));
    }

    #[rstest]
    #[case("a - b - c", "(Sub (Sub a b) c)")]
    #[case("a || b || c", "(Or (Or a b) c)")]
    fn left_associative(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("a>b", "(Gt a b)")]
    #[case("a>=b", "(Ge a b)")]
    #[case("a>>b", "(Shr a b)")]
    #[case("a>>=b", "(Shr= a b)")]
    fn operator_starting_with_gt_without_spaces(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[test]
    fn glued_operator_span() {
        assert_eq!(parse_expr("a>>b").0.unwrap().span.into_range(), 0..4);
    }

    #[rstest]
    #[case("a > > b")]
    #[case("a > = b")]
    #[case("a >> = b")]
    #[case("a > >= b")]
    fn operator_starting_with_gt_with_spaces_is_error(#[case] src: &str) {
        assert!(!parse_expr(src).1.is_empty());
    }

    #[rstest]
    #[case("a < b < c")]
    #[case("a == b == c")]
    #[case("a < b == c")]
    #[case("(a < b > c)")]
    fn comparison_does_not_chain(#[case] src: &str) {
        assert!(!parse_expr(src).1.is_empty());
    }

    #[test]
    fn parenthesized_comparison_can_be_compared() {
        assert_eq!(parse_ok("(a < b) == c"), "(Eq (paren (Lt a b)) c)");
    }

    #[test]
    fn operator_spans() {
        let (expr, _) = parse_expr("a + -b");
        let expr = expr.unwrap();
        assert_eq!(expr.span.into_range(), 0..6);
        let ExprKind::Binary { rhs, .. } = expr.kind else {
            panic!("{expr:?}");
        };
        assert_eq!(rhs.span.into_range(), 4..6);
    }

    #[rstest]
    #[case("a..b", "(.. a b)")]
    #[case("a..", "(.. a _)")]
    #[case("..b", "(.. _ b)")]
    #[case("..", "(.. _ _)")]
    #[case("a..=b", "(..= a b)")]
    #[case("..=b", "(..= _ b)")]
    #[case("a || b..c && d", "(.. (Or a b) (And c d))")]
    #[case("(a..)", "(paren (.. a _))")]
    #[case("[.., a..]", "(array (.. _ _) (.. a _))")]
    fn range(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("a..=")]
    #[case("..=")]
    #[case("a..b..c")]
    #[case("..a..")]
    #[case("(a..=)")]
    fn invalid_range(#[case] src: &str) {
        assert!(!parse_expr(src).1.is_empty());
    }

    #[rstest]
    #[case("a = b", "")]
    #[case("a += b", "Add")]
    #[case("a -= b", "Sub")]
    #[case("a *= b", "Mul")]
    #[case("a /= b", "Div")]
    #[case("a %= b", "Rem")]
    #[case("a &= b", "BitAnd")]
    #[case("a |= b", "BitOr")]
    #[case("a ^= b", "BitXor")]
    #[case("a <<= b", "Shl")]
    #[case("a >>= b", "Shr")]
    fn assignment(#[case] src: &str, #[case] op: &str) {
        assert_eq!(parse_ok(src), format!("({op}= a b)"));
    }

    #[rstest]
    #[case("a = b = c", "(= a (= b c))")]
    #[case("a = b..c", "(= a (.. b c))")]
    #[case("a += b || c", "(Add= a (Or b c))")]
    #[case("*a = -b", "(= (Deref a) (Neg b))")]
    fn assignment_is_right_associative_and_weakest(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[test]
    fn call_without_args() {
        assert_eq!(parse_ok("f()"), "(call f)");
    }

    #[test]
    fn call_with_args() {
        assert_eq!(parse_ok("f(a, b,)"), "(call f a b)");
    }

    #[test]
    fn chained_calls() {
        assert_eq!(parse_ok("f(a)(b)"), "(call (call f a) b)");
    }

    #[test]
    fn method_call_without_args() {
        assert_eq!(parse_ok("a.f()"), "(method a f)");
    }

    #[test]
    fn method_call_with_args() {
        assert_eq!(parse_ok("a.f(b, c)"), "(method a f b c)");
    }

    #[test]
    fn named_field() {
        assert_eq!(parse_ok("a.b"), "(field a b)");
    }

    #[test]
    fn chained_fields() {
        assert_eq!(parse_ok("a.b.c"), "(field (field a b) c)");
    }

    #[test]
    fn index() {
        assert_eq!(parse_ok("a[i]"), "(index a i)");
    }

    #[test]
    fn chained_index() {
        assert_eq!(parse_ok("a[i][j]"), "(index (index a i) j)");
    }

    #[test]
    fn try_operator() {
        assert_eq!(parse_ok("a?"), "(? a)");
    }

    #[test]
    fn try_after_method_call() {
        assert_eq!(parse_ok("a.f()?"), "(? (method a f))");
    }

    #[test]
    fn calling_field_needs_parens() {
        assert_eq!(parse_ok("(a.f)(b)"), "(call (paren (field a f)) b)");
    }

    #[test]
    fn postfix_binds_tighter_than_neg() {
        assert_eq!(parse_ok("-a.b"), "(Neg (field a b))");
    }

    #[test]
    fn postfix_binds_tighter_than_deref() {
        assert_eq!(parse_ok("*a?"), "(Deref (? a))");
    }

    #[test]
    fn postfix_in_binary() {
        assert_eq!(parse_ok("a.b + c[d]"), "(Add (field a b) (index c d))");
    }

    #[test]
    fn call_args_need_commas() {
        assert!(!parse_expr("f(a b)").1.is_empty());
    }

    #[test]
    fn field_needs_name() {
        assert!(!parse_expr("a.").1.is_empty());
    }

    #[test]
    fn index_needs_expr() {
        assert!(!parse_expr("a[]").1.is_empty());
    }

    #[test]
    fn index_takes_one_expr() {
        assert!(!parse_expr("a[i, j]").1.is_empty());
    }

    #[test]
    fn postfix_spans() {
        let expr = parse_expr("a.b(c)").0.unwrap();
        let ExprKind::MethodCall { receiver, .. } = &expr.kind else {
            panic!("{expr:?}");
        };
        assert_eq!(expr.span.into_range(), 0..6);
        assert_eq!(receiver.span.into_range(), 0..1);
    }

    #[test]
    fn tuple_field() {
        assert_eq!(parse_ok("t.0"), "(field t 0)");
    }

    #[test]
    fn tuple_field_with_multiple_digits() {
        assert_eq!(parse_ok("t.12"), "(field t 12)");
    }

    #[test]
    fn nested_tuple_fields() {
        assert_eq!(parse_ok("t.0.1"), "(field (field t 0) 1)");
    }

    #[test]
    fn deeply_nested_tuple_fields() {
        assert_eq!(parse_ok("t.1.2.3"), "(field (field (field t 1) 2) 3)");
    }

    #[test]
    fn tuple_field_after_whitespace() {
        assert_eq!(parse_ok("t. /* c */ 0"), "(field t 0)");
    }

    #[test]
    fn method_call_on_tuple_field() {
        assert_eq!(parse_ok("t.0.f()"), "(method (field t 0) f)");
    }

    #[test]
    fn tuple_index_with_suffix() {
        assert_invalid_tuple_index("t.0u8");
    }

    #[test]
    fn tuple_index_with_leading_zero() {
        assert_invalid_tuple_index("t.01");
    }

    #[test]
    fn tuple_index_with_underscore() {
        assert_invalid_tuple_index("t.1_0");
    }

    #[test]
    fn tuple_index_in_hex() {
        assert_invalid_tuple_index("t.0x1");
    }

    #[test]
    fn nested_tuple_index_with_exponent() {
        assert_invalid_tuple_index("t.0.1e1");
    }

    #[test]
    fn nested_tuple_index_with_suffix() {
        assert_invalid_tuple_index("t.0.1f32");
    }

    #[test]
    fn nested_tuple_index_with_leading_zero() {
        assert_invalid_tuple_index("t.0.01");
    }

    fn assert_invalid_tuple_index(src: &str) {
        let kinds: Vec<_> = parse_expr(src).1.into_iter().map(|e| e.kind).collect();
        assert_eq!(kinds, [ErrorKind::InvalidTupleIndex], "{src}");
    }

    #[test]
    fn nested_tuple_field_spans() {
        let expr = parse_expr("t.0.1").0.unwrap();
        let ExprKind::Field { expr: inner, .. } = &expr.kind else {
            panic!("{expr:?}");
        };
        assert_eq!(expr.span.into_range(), 0..5);
        assert_eq!(inner.span.into_range(), 0..3);
    }

    #[test]
    fn path_with_turbofish() {
        assert_eq!(parse_ok("Vec::<i32>::new"), "Vec<i32>::new");
    }

    #[test]
    fn call_with_turbofish() {
        assert_eq!(parse_ok("parse::<i32>(s)"), "(call parse<i32> s)");
    }

    #[test]
    fn method_call_with_turbofish() {
        assert_eq!(parse_ok("a.f::<T, U>(b)"), "(method a f<T, U> b)");
    }

    #[test]
    fn turbofish_with_nested_generics() {
        assert_eq!(
            parse_ok("Vec::<Vec<i32>>::new()"),
            "(call Vec<Vec<i32>>::new)"
        );
    }

    #[test]
    fn generic_args_in_expr_need_colons() {
        assert!(!parse_expr("f<T>()").1.is_empty());
    }

    #[test]
    fn method_generic_args_need_colons() {
        assert!(!parse_expr("a.f<T>()").1.is_empty());
    }

    #[test]
    fn unterminated_turbofish() {
        assert!(!parse_expr("a::<T").1.is_empty());
    }

    #[test]
    fn cast() {
        assert_eq!(parse_ok("a as T"), "(as a T)");
    }

    #[test]
    fn chained_casts() {
        assert_eq!(parse_ok("a as T as U"), "(as (as a T) U)");
    }

    #[test]
    fn cast_binds_looser_than_neg() {
        assert_eq!(parse_ok("-a as T"), "(as (Neg a) T)");
    }

    #[test]
    fn cast_binds_tighter_than_mul() {
        assert_eq!(parse_ok("a * b as T"), "(Mul a (as b T))");
    }

    #[test]
    fn cast_after_postfix() {
        assert_eq!(parse_ok("a.b as T"), "(as (field a b) T)");
    }

    #[test]
    fn cast_to_generic_type() {
        assert_eq!(parse_ok("a as Vec<T>"), "(as a Vec<T>)");
    }

    #[test]
    fn cast_to_reference() {
        assert_eq!(parse_ok("a as &T"), "(as a (& T))");
    }

    #[test]
    fn cast_in_parens_then_comparison() {
        assert_eq!(parse_ok("(a as usize) < b"), "(Lt (paren (as a usize)) b)");
    }

    #[test]
    fn cast_type_does_not_take_bounds() {
        assert_eq!(parse_ok("a as dyn A + B"), "(Add (as a (dyn A)) B)");
    }

    #[test]
    fn lt_after_cast_starts_generic_args() {
        assert!(!parse_expr("a as usize < b").1.is_empty());
    }

    #[test]
    fn trailing_tokens_are_error() {
        assert!(!parse_expr("a b").1.is_empty());
    }
}

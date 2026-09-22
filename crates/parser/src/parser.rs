use crate::ast::{BinaryOp, Expr, ExprKind, Field, Lit, Path, PathSegment, Span, UnaryOp};
use crate::error::{Error, ErrorKind};
use crate::literal;
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
            path(src, empty()).map(|segments| {
                ExprKind::Path(Path {
                    segments: segments.into_iter().map(|(segment, ())| segment).collect(),
                })
            }),
            parens,
            array,
            just(Token::Error).to(ExprKind::Error),
        ))
        .map_with(|kind, e| Expr {
            kind,
            span: e.span(),
        })
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
            Method(String, Vec<Expr>),
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
            12,
            choice((
                args.clone().map(PostfixOp::Call),
                just(Token::Dot)
                    .ignore_then(ident)
                    .then(args)
                    .map(|(method, args)| PostfixOp::Method(method, args)),
                just(Token::Dot)
                    .ignore_then(ident)
                    .map(|name| PostfixOp::Field(Field::Named(name))),
                just(Token::Dot)
                    .ignore_then(one_of([Token::Int, Token::Float]))
                    .to_span()
                    .validate(move |span: Span, _, emitter| {
                        let index = span.start + 1;
                        tuple_indices(&src[index..span.end], index).unwrap_or_else(|| {
                            emitter.emit(Error {
                                span: Span::from(index..span.end),
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
                    PostfixOp::Method(method, args) => wrap(ExprKind::MethodCall {
                        receiver: lhs,
                        method,
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
        let unary = prefix(
            11,
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
            .then(assign_op.then(expr).or_not())
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

/// パスを解析する。各セグメントの後には `args` を続けて解析する
pub(crate) fn path<'tok, 'src: 'tok, I, O>(
    src: &'src str,
    args: impl Parser<'tok, I, O, Extra> + Clone,
) -> impl Parser<'tok, I, Vec<(PathSegment, O)>, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    let ident = just(Token::Ident)
        .to_span()
        .map(move |span: Span| PathSegment::Ident(src[span.into_range()].to_string()));
    let head = choice((
        just(Token::Super)
            .to(PathSegment::Super)
            .then(args.clone())
            .separated_by(just(Token::ColonColon))
            .at_least(1)
            .collect(),
        choice((
            just(Token::Crate).to(PathSegment::Crate),
            just(Token::SelfValue).to(PathSegment::SelfValue),
            just(Token::SelfType).to(PathSegment::SelfType),
            ident,
        ))
        .then(args.clone())
        .map(|segment| vec![segment]),
    ));
    let rest = just(Token::ColonColon)
        .ignore_then(ident.then(args))
        .repeated()
        .collect::<Vec<_>>();
    head.then(rest).map(|(mut segments, rest)| {
        segments.extend(rest);
        segments
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{ErrorKind, Expected, Found, LiteralError};
    use gyokuto_lexer::LexError;

    fn parse_ok(src: &str) -> String {
        let (expr, errors) = parse_expr(src);
        assert!(errors.is_empty(), "{src}: {errors:?}");
        expr.unwrap().to_string()
    }

    #[test]
    fn bool_literals() {
        assert_eq!(parse_ok("true"), "true");
        assert_eq!(parse_ok("false"), "false");
    }

    #[test]
    fn literals_in_expressions() {
        assert_eq!(
            parse_ok(r#"(1u8, 2.5, 'a', "s")"#),
            r#"(tuple Int { value: 1, suffix: Some(U8) } Float { digits: "2.5", suffix: None } Char('a') Str("s"))"#
        );
    }

    #[test]
    fn multi_scalar_graphemes_are_not_chars() {
        for src in ["'e\u{301}'", "'👨\u{200D}👩'", "'🇯🇵'"] {
            assert!(!parse_expr(src).1.is_empty(), "{src:?}");
        }
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
        assert_eq!(expr.unwrap().to_string(), "(array error a)");
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
        assert_eq!(errors[0].span.into_range(), 0..1);
        assert_eq!(errors[0].kind, ErrorKind::Lex(LexError::UnexpectedChar));
        assert_eq!(expr.unwrap().to_string(), "error");
    }

    #[test]
    fn parens_and_tuples() {
        assert_eq!(parse_ok("(a)"), "(paren a)");
        assert_eq!(parse_ok("()"), "(tuple)");
        assert_eq!(parse_ok("(a,)"), "(tuple a)");
        assert_eq!(parse_ok("(a, b)"), "(tuple a b)");
        assert_eq!(parse_ok("(a, b,)"), "(tuple a b)");
        assert_eq!(parse_ok("((a))"), "(paren (paren a))");
    }

    #[test]
    fn arrays() {
        assert_eq!(parse_ok("[]"), "(array)");
        assert_eq!(parse_ok("[a, b,]"), "(array a b)");
        assert_eq!(parse_ok("[a; n]"), "(repeat a n)");
        assert_eq!(parse_ok("[(a, b), [c]]"), "(array (tuple a b) (array c))");
    }

    #[test]
    fn invalid_separators() {
        for src in ["(a,,)", "(,)", "[a,,]", "[,]", "[a; n,]", "[a, b; n]"] {
            assert!(!parse_expr(src).1.is_empty(), "{src}");
        }
    }

    #[test]
    fn deeply_nested_delimiters() {
        let depth = 64;
        let parens = format!("{}a{}", "(".repeat(depth), ")".repeat(depth));
        let arrays = format!("{}a{}", "[".repeat(depth), "]".repeat(depth));
        assert!(parse_expr(&parens).1.is_empty());
        assert!(parse_expr(&arrays).1.is_empty());
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
        assert_eq!(expr.unwrap().to_string(), "(array error c)");
    }

    #[test]
    fn lexical_error_inside_delimiters() {
        let (expr, errors) = parse_expr("[a, $]");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(expr.unwrap().to_string(), "(array a error)");
    }

    #[test]
    fn unary_operators() {
        assert_eq!(parse_ok("-a"), "(Neg a)");
        assert_eq!(parse_ok("!a"), "(Not a)");
        assert_eq!(parse_ok("*a"), "(Deref a)");
        assert_eq!(parse_ok("&a"), "(Ref a)");
        assert_eq!(parse_ok("&mut a"), "(RefMut a)");
        assert_eq!(parse_ok("-!*a"), "(Neg (Not (Deref a)))");
        assert!(!parse_expr("&&a").1.is_empty());
    }

    #[test]
    fn negative_literal_is_unary() {
        assert_eq!(
            parse_ok("-128i8"),
            "(Neg Int { value: 128, suffix: Some(I8) })"
        );
    }

    #[test]
    fn binary_precedence() {
        for (src, expected) in [
            ("a * b + c", "(Add (Mul a b) c)"),
            ("a + b * c", "(Add a (Mul b c))"),
            ("a + b << c", "(Shl (Add a b) c)"),
            ("a << b & c", "(BitAnd (Shl a b) c)"),
            ("a & b ^ c", "(BitXor (BitAnd a b) c)"),
            ("a ^ b | c", "(BitOr (BitXor a b) c)"),
            ("a | b == c", "(Eq (BitOr a b) c)"),
            ("a == b && c", "(And (Eq a b) c)"),
            ("a && b || c", "(Or (And a b) c)"),
            ("-a * b", "(Mul (Neg a) b)"),
            ("(a + b) * c", "(Mul (paren (Add a b)) c)"),
        ] {
            assert_eq!(parse_ok(src), expected, "{src}");
        }
    }

    #[test]
    fn binary_operators() {
        for (src, op) in [
            ("a * b", "Mul"),
            ("a / b", "Div"),
            ("a % b", "Rem"),
            ("a + b", "Add"),
            ("a - b", "Sub"),
            ("a << b", "Shl"),
            ("a >> b", "Shr"),
            ("a & b", "BitAnd"),
            ("a ^ b", "BitXor"),
            ("a | b", "BitOr"),
            ("a == b", "Eq"),
            ("a != b", "Ne"),
            ("a < b", "Lt"),
            ("a > b", "Gt"),
            ("a <= b", "Le"),
            ("a >= b", "Ge"),
            ("a && b", "And"),
            ("a || b", "Or"),
        ] {
            assert_eq!(parse_ok(src), format!("({op} a b)"), "{src}");
        }
    }

    #[test]
    fn left_associative() {
        assert_eq!(parse_ok("a - b - c"), "(Sub (Sub a b) c)");
        assert_eq!(parse_ok("a || b || c"), "(Or (Or a b) c)");
    }

    #[test]
    fn operators_starting_with_gt_without_spaces() {
        assert_eq!(parse_ok("a>b"), "(Gt a b)");
        assert_eq!(parse_ok("a>=b"), "(Ge a b)");
        assert_eq!(parse_ok("a>>b"), "(Shr a b)");
        assert_eq!(parse_ok("a>>=b"), "(Shr= a b)");
        assert_eq!(parse_expr("a>>b").0.unwrap().span.into_range(), 0..4);
        for src in ["a > > b", "a > = b", "a >> = b", "a > >= b"] {
            assert!(!parse_expr(src).1.is_empty(), "{src}");
        }
    }

    #[test]
    fn comparisons_do_not_chain() {
        for src in ["a < b < c", "a == b == c", "a < b == c", "(a < b > c)"] {
            assert!(!parse_expr(src).1.is_empty(), "{src}");
        }
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

    #[test]
    fn ranges() {
        for (src, expected) in [
            ("a..b", "(.. a b)"),
            ("a..", "(.. a _)"),
            ("..b", "(.. _ b)"),
            ("..", "(.. _ _)"),
            ("a..=b", "(..= a b)"),
            ("..=b", "(..= _ b)"),
            ("a || b..c && d", "(.. (Or a b) (And c d))"),
            ("(a..)", "(paren (.. a _))"),
            ("[.., a..]", "(array (.. _ _) (.. a _))"),
        ] {
            assert_eq!(parse_ok(src), expected, "{src}");
        }
    }

    #[test]
    fn invalid_ranges() {
        for src in ["a..=", "..=", "a..b..c", "..a..", "(a..=)"] {
            assert!(!parse_expr(src).1.is_empty(), "{src}");
        }
    }

    #[test]
    fn assignments() {
        for (src, op) in [
            ("a = b", ""),
            ("a += b", "Add"),
            ("a -= b", "Sub"),
            ("a *= b", "Mul"),
            ("a /= b", "Div"),
            ("a %= b", "Rem"),
            ("a &= b", "BitAnd"),
            ("a |= b", "BitOr"),
            ("a ^= b", "BitXor"),
            ("a <<= b", "Shl"),
            ("a >>= b", "Shr"),
        ] {
            assert_eq!(parse_ok(src), format!("({op}= a b)"), "{src}");
        }
    }

    #[test]
    fn assignment_is_right_associative_and_weakest() {
        assert_eq!(parse_ok("a = b = c"), "(= a (= b c))");
        assert_eq!(parse_ok("a = b..c"), "(= a (.. b c))");
        assert_eq!(parse_ok("a += b || c"), "(Add= a (Or b c))");
        assert_eq!(parse_ok("*a = -b"), "(= (Deref a) (Neg b))");
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
    fn trailing_tokens_are_error() {
        assert!(!parse_expr("a b").1.is_empty());
    }
}

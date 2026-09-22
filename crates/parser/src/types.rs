use crate::ast::{Expr, ExprKind, GenericArg, GenericArgs, Path, Span, Type, TypeKind, UnaryOp};
use crate::error::Error;
use crate::parser::{Extra, expr, input, lex, literal, path};
use chumsky::{input::ValueInput, prelude::*};
use gyokuto_lexer::Token;
use std::iter::once;

/// 型を解析する
pub fn parse_type(src: &str) -> (Option<Type>, Vec<Error>) {
    let (tokens, mut errors) = lex(src);
    let (ty, parse_errors) = ty(src, expr(src))
        .then_ignore(end())
        .parse(input(&tokens, src))
        .into_output_errors();
    errors.extend(parse_errors);
    (ty, errors)
}

/// 型を解析する。配列の長さなどの式は `expr` で解析する
pub(crate) fn ty<'tok, 'src: 'tok, I>(
    src: &'src str,
    expr: impl Parser<'tok, I, Expr, Extra> + Clone + 'tok,
) -> impl Parser<'tok, I, Type, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    recursive(|ty| {
        let no_bounds = no_bounds(src, expr, ty.clone());
        let bounds = type_path(src, ty, no_bounds.clone())
            .separated_by(just(Token::Plus))
            .at_least(1)
            .collect::<Vec<_>>();
        choice((
            just(Token::Dyn)
                .ignore_then(bounds.clone())
                .map(TypeKind::Dyn),
            just(Token::Impl).ignore_then(bounds).map(TypeKind::Impl),
        ))
        .map_with(|kind, e| Type {
            kind,
            span: e.span(),
        })
        .or(no_bounds)
    })
}

/// `+` を含まない型 (TypeNoBounds) を解析する
pub(crate) fn ty_no_bounds<'tok, 'src: 'tok, I>(
    src: &'src str,
    expr: impl Parser<'tok, I, Expr, Extra> + Clone + 'tok,
) -> impl Parser<'tok, I, Type, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    no_bounds(src, expr.clone(), ty(src, expr))
}

/// TypeNoBounds を解析する。内側の型は `ty` で解析する
fn no_bounds<'tok, 'src: 'tok, I>(
    src: &'src str,
    expr: impl Parser<'tok, I, Expr, Extra> + Clone + 'tok,
    ty: impl Parser<'tok, I, Type, Extra> + Clone + 'tok,
) -> impl Parser<'tok, I, Type, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    let rest_items = just(Token::Comma).ignore_then(
        ty.clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>(),
    );
    let parens = ty
        .clone()
        .then(rest_items.or_not())
        .or_not()
        .delimited_by(just(Token::LParen), just(Token::RParen))
        .map(|inner| match inner {
            None => TypeKind::Tuple(Vec::new()),
            Some((first, None)) => TypeKind::Paren(Box::new(first)),
            Some((first, Some(rest))) => TypeKind::Tuple(once(first).chain(rest).collect()),
        });

    let array = ty
        .clone()
        .then(just(Token::Semi).ignore_then(expr).or_not())
        .delimited_by(just(Token::LBracket), just(Token::RBracket))
        .map(|(elem, len)| match len {
            None => TypeKind::Slice(Box::new(elem)),
            Some(len) => TypeKind::Array {
                elem: Box::new(elem),
                len: Box::new(len),
            },
        });

    let error = |span| Type {
        kind: TypeKind::Error,
        span,
    };
    recursive(|no_bounds| {
        let type_path = type_path(src, ty.clone(), no_bounds.clone());
        let reference = just(Token::Amp)
            .ignore_then(just(Token::Mut).or_not())
            .then(no_bounds.clone())
            .map(|(mutable, ty)| TypeKind::Ref {
                mutable: mutable.is_some(),
                ty: Box::new(ty),
            });
        let fn_type = just(Token::Fn)
            .ignore_then(signature(ty, no_bounds))
            .map(|(params, ret)| TypeKind::Fn { params, ret });
        choice((
            type_path.clone().map(TypeKind::Path),
            reference,
            parens,
            array,
            fn_type,
            just(Token::Dyn)
                .ignore_then(type_path.clone())
                .map(|path| TypeKind::Dyn(vec![path])),
            just(Token::Impl)
                .ignore_then(type_path)
                .map(|path| TypeKind::Impl(vec![path])),
            just(Token::Bang).to(TypeKind::Never),
            just(Token::Underscore).to(TypeKind::Infer),
        ))
        .map_with(|kind, e| Type {
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
        )))
    })
}

/// 型のパスを解析する。型引数の型は `ty`、`Fn() -> R` の戻り値の型は `no_bounds` で解析する
fn type_path<'tok, 'src: 'tok, I>(
    src: &'src str,
    ty: impl Parser<'tok, I, Type, Extra> + Clone + 'tok,
    no_bounds: impl Parser<'tok, I, Type, Extra> + Clone + 'tok,
) -> impl Parser<'tok, I, Path, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    let args = angle_args(src, ty.clone())
        .or(signature(ty, no_bounds).map(|(inputs, output)| GenericArgs::Paren { inputs, output }));
    path(src, args.or_not())
}

/// `(A, B) -> R` を解析する
fn signature<'tok, I>(
    ty: impl Parser<'tok, I, Type, Extra> + Clone,
    no_bounds: impl Parser<'tok, I, Type, Extra> + Clone,
) -> impl Parser<'tok, I, (Vec<Type>, Option<Box<Type>>), Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    ty.separated_by(just(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(Token::LParen), just(Token::RParen))
        .then(
            just(Token::Arrow)
                .ignore_then(no_bounds)
                .or_not()
                .map(|ret| ret.map(Box::new)),
        )
}

/// `<A, B>` の型引数を解析する
///
/// `<` の後は常に型引数として扱い、閉じられなくても比較演算子などとして解析し直さない
pub(crate) fn angle_args<'tok, 'src: 'tok, I>(
    src: &'src str,
    ty: impl Parser<'tok, I, Type, Extra> + Clone,
) -> impl Parser<'tok, I, GenericArgs, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    let lit = literal(src).map_with(|kind, e| Expr {
        kind,
        span: e.span(),
    });
    let neg_lit = just(Token::Minus)
        .ignore_then(lit.clone())
        .map_with(|lit, e| Expr {
            kind: ExprKind::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(lit),
            },
            span: e.span(),
        });
    let binding = just(Token::Ident)
        .to_span()
        .then_ignore(just(Token::Eq))
        .then(ty.clone())
        .map(move |(name, ty): (Span, _)| GenericArg::Binding {
            name: src[name.into_range()].to_string(),
            ty,
        });
    let arg = choice((
        binding,
        lit.or(neg_lit).map(GenericArg::Const),
        ty.map(GenericArg::Type),
    ));
    just(Token::Lt)
        .ignore_then(
            arg.separated_by(just(Token::Comma))
                .allow_trailing()
                .collect()
                .then_ignore(just(Token::Gt))
                .recover_with(via_parser(empty().map(|()| Vec::new()))),
        )
        .map(GenericArgs::Angle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sexpr::AsSexpr;

    fn parse_ok(src: &str) -> String {
        let (ty, errors) = parse_type(src);
        assert!(errors.is_empty(), "{src}: {errors:?}");
        ty.unwrap().as_sexpr()
    }

    #[test]
    fn path_types() {
        for src in ["i32", "a::B", "crate::a::T", "Self", "super::T", "Vec<_>"] {
            assert_eq!(parse_ok(src), src);
        }
    }

    #[test]
    fn generic_args() {
        for (src, expected) in [
            ("Vec<i32>", "Vec<i32>"),
            ("HashMap<K, V,>", "HashMap<K, V>"),
            ("Vec<Vec<i32> >", "Vec<Vec<i32>>"),
            ("Vec<Vec<i32>>", "Vec<Vec<i32>>"),
            ("A<B<C<D>>>", "A<B<C<D>>>"),
            ("a::B<T>::C", "a::B<T>::C"),
            ("Foo<>", "Foo<>"),
            ("Iterator<Item = T>", "Iterator<Item = T>"),
            ("Buffer<16>", "Buffer<Int { value: 16, suffix: None }>"),
            ("Buffer<-1>", "Buffer<(Neg Int { value: 1, suffix: None })>"),
            ("Flag<true>", "Flag<true>"),
        ] {
            assert_eq!(parse_ok(src), expected, "{src}");
        }
    }

    #[test]
    fn reference_types() {
        assert_eq!(parse_ok("&T"), "(& T)");
        assert_eq!(parse_ok("&mut T"), "(&mut T)");
        assert_eq!(parse_ok("&mut [u8]"), "(&mut (slice u8))");
        assert!(!parse_type("&&T").1.is_empty());
    }

    #[test]
    fn tuple_and_paren_types() {
        assert_eq!(parse_ok("()"), "(tuple)");
        assert_eq!(parse_ok("(T)"), "(paren T)");
        assert_eq!(parse_ok("(T,)"), "(tuple T)");
        assert_eq!(parse_ok("(A, B)"), "(tuple A B)");
    }

    #[test]
    fn array_and_slice_types() {
        assert_eq!(parse_ok("[T; n]"), "(array T n)");
        assert_eq!(parse_ok("[T]"), "(slice T)");
        assert_eq!(
            parse_ok("[[u8; n]; m + 1]"),
            "(array (array u8 n) (Add m Int { value: 1, suffix: None }))"
        );
    }

    #[test]
    fn fn_types() {
        assert_eq!(parse_ok("fn()"), "(fn ())");
        assert_eq!(parse_ok("fn(A, B,) -> C"), "(fn (A B) C)");
        assert_eq!(parse_ok("fn(fn(A)) -> !"), "(fn ((fn (A))) !)");
    }

    #[test]
    fn parenthesized_generic_args() {
        assert_eq!(parse_ok("Fn(A) -> B"), "Fn(A) -> B");
        assert_eq!(parse_ok("FnMut()"), "FnMut()");
        assert_eq!(
            parse_ok("Box<dyn Fn(i32) -> i32>"),
            "Box<(dyn Fn(i32) -> i32)>"
        );
    }

    #[test]
    fn dyn_and_impl_types() {
        for (src, expected) in [
            ("dyn A", "(dyn A)"),
            ("dyn A + B", "(dyn A B)"),
            (
                "impl Iterator<Item = T> + Clone",
                "(impl Iterator<Item = T> Clone)",
            ),
            ("&dyn A", "(& (dyn A))"),
            ("&(dyn A + B)", "(& (paren (dyn A B)))"),
            ("dyn Fn() -> A + B", "(dyn Fn() -> A B)"),
            ("Vec<dyn A + B>", "Vec<(dyn A B)>"),
        ] {
            assert_eq!(parse_ok(src), expected, "{src}");
        }
    }

    #[test]
    fn bounds_need_parens_in_ambiguous_positions() {
        for src in [
            "&dyn A + B",
            "fn() -> dyn A + B",
            "dyn",
            "dyn A +",
            "impl &A",
        ] {
            assert!(!parse_type(src).1.is_empty(), "{src}");
        }
    }

    #[test]
    fn never_and_infer_types() {
        assert_eq!(parse_ok("!"), "!");
        assert_eq!(parse_ok("_"), "_");
    }

    #[test]
    fn invalid_types() {
        for src in [
            "Vec<",
            "a::",
            "1",
            "Vec<a + b>",
            "Vec<T",
            "[T; ]",
            "&",
            "(A B)",
        ] {
            assert!(!parse_type(src).1.is_empty(), "{src}");
        }
    }

    #[test]
    fn type_spans() {
        let (ty, _) = parse_type(" Vec<T> ");
        assert_eq!(ty.unwrap().span.into_range(), 1..7);
    }

    #[test]
    fn reference_type_spans() {
        let (ty, _) = parse_type(" &mut T ");
        assert_eq!(ty.unwrap().span.into_range(), 1..7);
    }

    #[test]
    fn recovers_inside_delimiters() {
        let (ty, errors) = parse_type("(A, [B C])");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(ty.unwrap().as_sexpr(), "(tuple A error)");
    }
}

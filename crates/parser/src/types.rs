use crate::ast::{
    Expr, ExprKind, GenericArg, GenericArgs, Span, Type, TypeKind, TypePath, TypePathSegment,
    UnaryOp,
};
use crate::error::Error;
use crate::parser::{Extra, input, lex, literal, path};
use chumsky::{input::ValueInput, prelude::*};
use gyokuto_lexer::Token;

/// 型を解析する
pub fn parse_type(src: &str) -> (Option<Type>, Vec<Error>) {
    let (tokens, mut errors) = lex(src);
    let (ty, parse_errors) = ty(src)
        .then_ignore(end())
        .parse(input(&tokens, src))
        .into_output_errors();
    errors.extend(parse_errors);
    (ty, errors)
}

pub(crate) fn ty<'tok, 'src: 'tok, I>(src: &'src str) -> impl Parser<'tok, I, Type, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    recursive(|ty| {
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
            ty.clone().map(GenericArg::Type),
        ));
        let args = arg
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect()
            .delimited_by(just(Token::Lt), just(Token::Gt))
            .map(GenericArgs::Angle);
        let type_path = path(src, args.or_not()).map(|segments| TypePath {
            segments: segments
                .into_iter()
                .map(|(segment, args)| TypePathSegment { segment, args })
                .collect(),
        });

        choice((
            type_path.map(TypeKind::Path),
            just(Token::Bang).to(TypeKind::Never),
            just(Token::Underscore).to(TypeKind::Infer),
        ))
        .map_with(|kind, e| Type {
            kind,
            span: e.span(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{GenericArg, GenericArgs};
    use crate::parser::tests::{show, show_segment};

    fn show_type(ty: &Type) -> String {
        let list = |name: &str, types: &[&Type]| {
            let items: String = types.iter().map(|t| format!(" {}", show_type(t))).collect();
            format!("({name}{items})")
        };
        match &ty.kind {
            TypeKind::Path(path) => path
                .segments
                .iter()
                .map(|s| {
                    let args = s.args.as_ref().map_or(String::new(), |args| match args {
                        GenericArgs::Angle(args) => {
                            let args: Vec<_> = args.iter().map(show_arg).collect();
                            format!("<{}>", args.join(", "))
                        }
                    });
                    format!("{}{args}", show_segment(&s.segment))
                })
                .collect::<Vec<_>>()
                .join("::"),
            TypeKind::Ref { mutable, ty } => list(if *mutable { "&mut" } else { "&" }, &[ty]),
            TypeKind::Paren(t) => list("paren", &[t]),
            TypeKind::Tuple(ts) => list("tuple", &ts.iter().collect::<Vec<_>>()),
            TypeKind::Array { elem, len } => format!("(array {} {})", show_type(elem), show(len)),
            TypeKind::Slice(t) => list("slice", &[t]),
            TypeKind::Never => "!".to_string(),
            TypeKind::Infer => "_".to_string(),
            TypeKind::Error => "error".to_string(),
        }
    }

    fn show_arg(arg: &GenericArg) -> String {
        match arg {
            GenericArg::Type(ty) => show_type(ty),
            GenericArg::Binding { name, ty } => format!("{name} = {}", show_type(ty)),
            GenericArg::Const(expr) => show(expr),
        }
    }

    fn parse_ok(src: &str) -> String {
        let (ty, errors) = parse_type(src);
        assert!(errors.is_empty(), "{src}: {errors:?}");
        show_type(&ty.unwrap())
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
    fn never_and_infer_types() {
        assert_eq!(parse_ok("!"), "!");
        assert_eq!(parse_ok("_"), "_");
    }

    #[test]
    fn invalid_types() {
        for src in ["Vec<", "a::", "[T; ]", "&", "(A B)", "1", "Vec<a + b>"] {
            assert!(!parse_type(src).1.is_empty(), "{src}");
        }
    }

    #[test]
    fn type_spans() {
        let (ty, _) = parse_type(" &mut T ");
        assert_eq!(ty.unwrap().span.into_range(), 1..7);
    }

    #[test]
    fn recovers_inside_delimiters() {
        let (ty, errors) = parse_type("(A, [B C])");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(show_type(&ty.unwrap()), "(tuple A error)");
    }
}

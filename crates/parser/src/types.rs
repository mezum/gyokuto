use crate::ast::{Span, Type, TypeKind};
use crate::error::Error;
use crate::parser::{Extra, input, lex};
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

pub(crate) fn ty<'tok, 'src: 'tok, I>(_src: &'src str) -> impl Parser<'tok, I, Type, Extra> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = Span>,
{
    any().map_with(|_, e| Type {
        kind: TypeKind::Error,
        span: e.span(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{GenericArg, GenericArgs};
    use crate::parser::tests::{show, show_segment};

    fn show_type(ty: &Type) -> String {
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
    fn never_and_infer_types() {
        assert_eq!(parse_ok("!"), "!");
        assert_eq!(parse_ok("_"), "_");
    }

    #[test]
    fn invalid_types() {
        for src in ["Vec<", "a::", "1", "Vec<a + b>", "Vec<T"] {
            assert!(!parse_type(src).1.is_empty(), "{src}");
        }
    }

    #[test]
    fn type_spans() {
        let (ty, _) = parse_type(" Vec<T> ");
        assert_eq!(ty.unwrap().span.into_range(), 1..7);
    }
}

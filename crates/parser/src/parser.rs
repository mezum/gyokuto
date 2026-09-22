#[cfg(test)]
mod tests {
    use super::*;

    fn show(expr: &Expr) -> String {
        match &expr.kind {
            ExprKind::Lit(Lit::Bool(b)) => b.to_string(),
            ExprKind::Path(path) => path
                .segments
                .iter()
                .map(|s| match s {
                    PathSegment::Ident(name) => name.as_str(),
                    PathSegment::Crate => "crate",
                    PathSegment::Super => "super",
                    PathSegment::SelfValue => "self",
                    PathSegment::SelfType => "Self",
                })
                .collect::<Vec<_>>()
                .join("::"),
            ExprKind::Error => "error".to_string(),
        }
    }

    fn parse_ok(src: &str) -> String {
        let (expr, errors) = parse_expr(src);
        assert!(errors.is_empty(), "{src}: {errors:?}");
        show(&expr.unwrap())
    }

    #[test]
    fn bool_literals() {
        assert_eq!(parse_ok("true"), "true");
        assert_eq!(parse_ok("false"), "false");
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
        assert_eq!(errors[0].span().into_range(), 0..1);
        assert_eq!(show(&expr.unwrap()), "error");
    }

    #[test]
    fn trailing_tokens_are_error() {
        assert!(!parse_expr("a b").1.is_empty());
    }
}

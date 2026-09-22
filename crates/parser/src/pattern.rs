#[cfg(test)]
mod tests {
    use super::*;
    use crate::sexpr::AsSexpr;
    use rstest::rstest;

    fn parse_ok(src: &str) -> String {
        let (pat, errors) = parse_pattern(src);
        assert!(errors.is_empty(), "{src}: {errors:?}");
        pat.unwrap().as_sexpr()
    }

    #[rstest]
    #[case("_", "_")]
    #[case("x", "x")]
    #[case("mut x", "(mut x)")]
    #[case("true", "true")]
    #[case("'a'", "Char('a')")]
    #[case("-1", "(Neg Int { value: 1, suffix: None })")]
    fn simple_pattern(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("x in _", "(in x _)")]
    #[case("mut x in (a, b)", "(in mut x (tuple a b))")]
    #[case("x in y in _", "(in x (in y _))")]
    #[case("x in a | b", "(| (in x a) b)")]
    fn binding_pattern(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("'a'..='z'", "(..= Char('a') Char('z'))")]
    #[case("'a'..'z'", "(.. Char('a') Char('z'))")]
    #[case("'a'..", "(.. Char('a') _)")]
    #[case("..='z'", "(..= _ Char('z'))")]
    #[case("MIN..MAX", "(.. MIN MAX)")]
    #[case(
        "-1..=1",
        "(..= (Neg Int { value: 1, suffix: None }) Int { value: 1, suffix: None })"
    )]
    fn range_pattern(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("()", "(tuple)")]
    #[case("(x)", "(paren x)")]
    #[case("(x,)", "(tuple x)")]
    #[case("(x, .., y)", "(tuple x .. y)")]
    #[case("a | b | c", "(| a b c)")]
    #[case("(a | b, c)", "(tuple (| a b) c)")]
    fn compound_pattern(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("..=")]
    #[case("'a'..=")]
    #[case("mut")]
    #[case("mut 1")]
    #[case("x in")]
    #[case("1 in x")]
    #[case("a |")]
    #[case("| a")]
    #[case("(x y)")]
    fn invalid_pattern(#[case] src: &str) {
        assert!(!parse_pattern(src).1.is_empty());
    }

    #[test]
    fn pattern_spans() {
        let pat = parse_pattern("(a, x in _)").0.unwrap();
        let PatKind::Tuple(pats) = pat.kind else {
            panic!("{pat:?}");
        };
        let spans: Vec<_> = pats.iter().map(|p| p.span.into_range()).collect();
        assert_eq!(spans, [1..2, 4..10]);
    }
}

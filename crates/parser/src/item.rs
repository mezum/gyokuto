#[cfg(test)]
mod tests {
    use crate::parse_module;
    use crate::sexpr::AsSexpr;
    use rstest::rstest;

    fn parse_ok(src: &str) -> String {
        let (items, errors) = parse_module(src);
        assert!(errors.is_empty(), "{src}: {errors:?}");
        let items: Vec<_> = items.unwrap().iter().map(AsSexpr::as_sexpr).collect();
        items.join(" ")
    }

    #[rstest]
    #[case("", "")]
    #[case("fn f() {}", "(fn f (params) (block))")]
    #[case("fn f() { a }", "(fn f (params) (block a))")]
    #[case("fn f() {} fn g() {}", "(fn f (params) (block)) (fn g (params) (block))")]
    fn fn_item(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("fn f(x: i32) {}", "(fn f (params (: x i32)) (block))")]
    #[case("fn f(x: A, y: B,) {}", "(fn f (params (: x A) (: y B)) (block))")]
    #[case("fn f(mut x: T) {}", "(fn f (params (: (mut x) T)) (block))")]
    #[case(
        "fn f((a, b): (A, B)) {}",
        "(fn f (params (: (tuple a b) (tuple A B))) (block))"
    )]
    #[case(
        "fn f(x in Some(_): T) {}",
        "(fn f (params (: (in x (Some _)) T)) (block))"
    )]
    fn fn_params(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("fn f(self) {}", "(fn f (params self) (block))")]
    #[case("fn f(mut self) {}", "(fn f (params (mut self)) (block))")]
    #[case("fn f(&self) {}", "(fn f (params (& self)) (block))")]
    #[case("fn f(&mut self, x: T) {}", "(fn f (params (&mut self) (: x T)) (block))")]
    #[case("fn f(self,) {}", "(fn f (params self) (block))")]
    #[case(
        "fn f(self::A(x): T) {}",
        "(fn f (params (: (self::A x) T)) (block))"
    )]
    fn fn_self_param(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("fn f() -> T { a }", "(fn f (params) (-> T) (block a))")]
    #[case("fn f() -> impl A + B {}", "(fn f (params) (-> (impl A B)) (block))")]
    fn fn_ret(#[case] src: &str, #[case] expected: &str) {
        assert_eq!(parse_ok(src), expected);
    }

    #[rstest]
    #[case("fn f()")]
    #[case("fn f();")]
    #[case("fn () {}")]
    #[case("f() {}")]
    #[case("fn f(x) {}")]
    #[case("fn f(a | b: T) {}")]
    #[case("fn f(x: T, self) {}")]
    #[case("fn f(self: T) {}")]
    #[case("fn f(& &self) {}")]
    #[case("fn f() -> {}")]
    #[case("a")]
    fn invalid_fn(#[case] src: &str) {
        assert!(!parse_module(src).1.is_empty());
    }

    #[test]
    fn item_spans() {
        let items = parse_module("fn f() {}  fn g() {}").0.unwrap();
        let spans: Vec<_> = items.iter().map(|i| i.span.into_range()).collect();
        assert_eq!(spans, [0..9, 11..20]);
    }
}

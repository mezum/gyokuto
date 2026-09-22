use chumsky::span::SimpleSpan;

pub type Span = SimpleSpan;

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Lit(Lit),
    Path(Path),
    /// 構文エラーから回復した箇所
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub segments: Vec<PathSegment>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathSegment {
    Ident(String),
    Crate,
    Super,
    SelfValue,
    SelfType,
}

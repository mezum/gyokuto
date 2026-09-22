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
    Paren(Box<Expr>),
    Tuple(Vec<Expr>),
    Array(Vec<Expr>),
    /// `[elem; len]`
    Repeat {
        elem: Box<Expr>,
        len: Box<Expr>,
    },
    /// 構文エラーから回復した箇所
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    Bool(bool),
    Int {
        value: u64,
        suffix: Option<IntSuffix>,
    },
    /// `f32` / `f64` のどちらになるかは型検査で決まるため、`_` を除いた 10 進表記で保持する
    Float {
        digits: String,
        suffix: Option<FloatSuffix>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntSuffix {
    I8,
    I16,
    I32,
    I64,
    Isize,
    U8,
    U16,
    U32,
    U64,
    Usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatSuffix {
    F32,
    F64,
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

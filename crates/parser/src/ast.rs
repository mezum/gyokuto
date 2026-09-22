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
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        /// `a.f::<T>()` の型引数
        generics: Option<GenericArgs>,
        args: Vec<Expr>,
    },
    Field {
        expr: Box<Expr>,
        field: Field,
    },
    Index {
        expr: Box<Expr>,
        index: Box<Expr>,
    },
    /// `expr?`
    Try(Box<Expr>),
    /// `expr as ty`
    Cast {
        expr: Box<Expr>,
        ty: Box<Type>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// `start..end` / `start..=end`。端点は省略できる
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
    },
    /// `place = value`。複合代入では `op` に演算を持つ
    Assign {
        op: Option<BinaryOp>,
        place: Box<Expr>,
        value: Box<Expr>,
    },
    Block(Block),
    /// 構文エラーから回復した箇所
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    /// 末尾の `;` を付けない式。ブロックの値となる
    pub expr: Option<Box<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Let {
        mutable: bool,
        name: String,
        ty: Option<Type>,
        init: Option<Expr>,
    },
    /// `expr;`
    Semi(Expr),
    /// `;` を付けないブロック様の式
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    Named(String),
    /// タプルのフィールド `t.0`
    Index(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub kind: TypeKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Path(Path),
    Ref {
        mutable: bool,
        ty: Box<Type>,
    },
    Paren(Box<Type>),
    Tuple(Vec<Type>),
    /// `[elem; len]`
    Array {
        elem: Box<Type>,
        len: Box<Expr>,
    },
    Slice(Box<Type>),
    /// `fn(params) -> ret`。`ret` が無い場合はユニット型
    Fn {
        params: Vec<Type>,
        ret: Option<Box<Type>>,
    },
    /// `dyn A + B`
    Dyn(Vec<Path>),
    /// `impl A + B`
    Impl(Vec<Path>),
    /// `!`
    Never,
    /// `_`
    Infer,
    /// 構文エラーから回復した箇所
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GenericArgs {
    /// `<A, B>`
    Angle(Vec<GenericArg>),
    /// `Fn(A, B) -> C`
    Paren {
        inputs: Vec<Type>,
        output: Option<Box<Type>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum GenericArg {
    Type(Type),
    /// 関連型の指定 `Item = T`
    Binding {
        name: String,
        ty: Type,
    },
    /// const generics の定数
    Const(Expr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    Deref,
    Ref,
    RefMut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Mul,
    Div,
    Rem,
    Add,
    Sub,
    Shl,
    Shr,
    BitAnd,
    BitXor,
    BitOr,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
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
    Char(char),
    Byte(u8),
    Str(String),
    ByteStr(Vec<u8>),
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
pub struct PathSegment {
    pub name: PathName,
    pub args: Option<GenericArgs>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathName {
    Ident(String),
    Crate,
    Super,
    SelfValue,
    SelfType,
}

impl PathName {
    /// ソース上の表記を得る
    pub fn as_str(&self) -> &str {
        match self {
            PathName::Ident(name) => name,
            PathName::Crate => "crate",
            PathName::Super => "super",
            PathName::SelfValue => "self",
            PathName::SelfType => "Self",
        }
    }
}

//! AST を S 式で表す

use crate::ast::{
    Arm, Block, Expr, ExprKind, Field, GenericArg, GenericArgs, GenericParam, Item, ItemKind, Lit,
    Pat, PatKind, Path, SelfParam, Stmt, StmtKind, Type, TypeKind,
};
use std::iter::once;

/// AST を S 式の文字列にする
pub trait AsSexpr {
    fn as_sexpr(&self) -> String;
}

fn list(name: &str, items: impl IntoIterator<Item = String>) -> String {
    once(format!("({name}"))
        .chain(items.into_iter().map(|item| format!(" {item}")))
        .chain(once(")".to_string()))
        .collect()
}

fn sexprs<'a, T: AsSexpr + 'a>(items: impl IntoIterator<Item = &'a T>) -> Vec<String> {
    items.into_iter().map(AsSexpr::as_sexpr).collect()
}

impl<T: AsSexpr> AsSexpr for Box<T> {
    fn as_sexpr(&self) -> String {
        T::as_sexpr(self)
    }
}

impl AsSexpr for Expr {
    fn as_sexpr(&self) -> String {
        match &self.kind {
            ExprKind::Lit(Lit::Bool(b)) => b.to_string(),
            ExprKind::Lit(lit) => format!("{lit:?}"),
            ExprKind::Path(path) => path.as_sexpr(),
            ExprKind::Paren(e) => list("paren", [e.as_sexpr()]),
            ExprKind::Tuple(es) => list("tuple", sexprs(es)),
            ExprKind::Array(es) => list("array", sexprs(es)),
            ExprKind::Repeat { elem, len } => list("repeat", sexprs([elem, len])),
            ExprKind::Call { callee, args } => {
                list("call", once(callee.as_sexpr()).chain(sexprs(args)))
            }
            ExprKind::MethodCall {
                receiver,
                method,
                generics,
                args,
            } => {
                let generics = generics.as_ref().map_or(String::new(), AsSexpr::as_sexpr);
                let head = [receiver.as_sexpr(), format!("{method}{generics}")];
                list("method", head.into_iter().chain(sexprs(args)))
            }
            ExprKind::Field { expr, field } => list("field", [expr.as_sexpr(), field.as_sexpr()]),
            ExprKind::Index { expr, index } => list("index", sexprs([expr, index])),
            ExprKind::Try(expr) => list("?", [expr.as_sexpr()]),
            ExprKind::Cast { expr, ty } => list("as", [expr.as_sexpr(), ty.as_sexpr()]),
            ExprKind::Unary { op, expr } => list(&format!("{op:?}"), [expr.as_sexpr()]),
            ExprKind::Binary { op, lhs, rhs } => list(&format!("{op:?}"), sexprs([lhs, rhs])),
            ExprKind::Range {
                start,
                end,
                inclusive,
            } => range(start, end, *inclusive),
            ExprKind::Assign { op, place, value } => {
                let op = op.map_or(String::new(), |op| format!("{op:?}"));
                list(&format!("{op}="), sexprs([place, value]))
            }
            ExprKind::Block(block) => block.as_sexpr(),
            ExprKind::If {
                cond,
                then,
                otherwise,
            } => list(
                "if",
                [cond.as_sexpr(), then.as_sexpr()]
                    .into_iter()
                    .chain(otherwise.as_ref().map(AsSexpr::as_sexpr)),
            ),
            ExprKind::Loop(body) => list("loop", [body.as_sexpr()]),
            ExprKind::While { cond, body } => list("while", [cond.as_sexpr(), body.as_sexpr()]),
            ExprKind::For { pat, iter, body } => {
                list("for", [pat.as_sexpr(), iter.as_sexpr(), body.as_sexpr()])
            }
            ExprKind::Let { pat, expr } => list("let", [pat.as_sexpr(), expr.as_sexpr()]),
            ExprKind::Match { scrutinee, arms } => {
                list("match", once(scrutinee.as_sexpr()).chain(sexprs(arms)))
            }
            ExprKind::Break(value) => list("break", value.as_ref().map(AsSexpr::as_sexpr)),
            ExprKind::Continue => "(continue)".to_string(),
            ExprKind::Return(value) => list("return", value.as_ref().map(AsSexpr::as_sexpr)),
            ExprKind::Error => "error".to_string(),
        }
    }
}

impl AsSexpr for Block {
    fn as_sexpr(&self) -> String {
        list(
            "block",
            sexprs(&self.stmts)
                .into_iter()
                .chain(self.expr.as_ref().map(AsSexpr::as_sexpr)),
        )
    }
}

impl AsSexpr for Item {
    fn as_sexpr(&self) -> String {
        match &self.kind {
            ItemKind::Fn { sig, body } => {
                let self_param = sig.self_param.map(|p| match p {
                    SelfParam::Value { mutable: false } => "self".to_string(),
                    SelfParam::Value { mutable: true } => "(mut self)".to_string(),
                    SelfParam::Ref { mutable: false } => "(& self)".to_string(),
                    SelfParam::Ref { mutable: true } => "(&mut self)".to_string(),
                });
                let params = self_param.into_iter().chain(
                    sig.params
                        .iter()
                        .map(|p| list(":", [p.pat.as_sexpr(), p.ty.as_sexpr()])),
                );
                let generics = sig.generics.iter().map(|g| match g {
                    GenericParam::Type { name, bounds } if bounds.is_empty() => name.clone(),
                    GenericParam::Type { name, bounds } => {
                        list(":", once(name.clone()).chain(sexprs(bounds)))
                    }
                    GenericParam::Const { name, ty } => {
                        list("const", [name.clone(), ty.as_sexpr()])
                    }
                });
                let where_preds = sig
                    .where_preds
                    .iter()
                    .map(|p| list(":", once(p.ty.as_sexpr()).chain(sexprs(&p.bounds))));
                let non_empty =
                    |name, items: Vec<String>| (!items.is_empty()).then(|| list(name, items));
                list(
                    "fn",
                    once(sig.name.clone())
                        .chain(non_empty("generics", generics.collect()))
                        .chain(once(list("params", params)))
                        .chain(sig.ret.as_ref().map(|ty| list("->", [ty.as_sexpr()])))
                        .chain(non_empty("where", where_preds.collect()))
                        .chain(once(body.as_sexpr())),
                )
            }
        }
    }
}

impl AsSexpr for Stmt {
    fn as_sexpr(&self) -> String {
        match &self.kind {
            StmtKind::Let {
                pat,
                ty,
                init,
                otherwise,
            } => list(
                "let",
                once(pat.as_sexpr())
                    .chain(ty.as_ref().map(|ty| list(":", [ty.as_sexpr()])))
                    .chain(init.as_ref().map(|init| list("=", [init.as_sexpr()])))
                    .chain(
                        otherwise
                            .as_ref()
                            .map(|block| list("else", [block.as_sexpr()])),
                    ),
            ),
            StmtKind::Semi(expr) => list("semi", [expr.as_sexpr()]),
            StmtKind::Expr(expr) => list("expr", [expr.as_sexpr()]),
        }
    }
}

fn range(start: &Option<Box<Expr>>, end: &Option<Box<Expr>>, inclusive: bool) -> String {
    let end_point = |e: &Option<Box<Expr>>| e.as_ref().map_or("_".into(), AsSexpr::as_sexpr);
    let op = if inclusive { "..=" } else { ".." };
    list(op, [end_point(start), end_point(end)])
}

impl AsSexpr for Arm {
    fn as_sexpr(&self) -> String {
        let guard = self
            .guard
            .as_ref()
            .map(|guard| list("if", [guard.as_sexpr()]));
        list(
            "=>",
            once(self.pat.as_sexpr())
                .chain(guard)
                .chain(once(self.body.as_sexpr())),
        )
    }
}

impl AsSexpr for Pat {
    fn as_sexpr(&self) -> String {
        match &self.kind {
            PatKind::Wild => "_".to_string(),
            PatKind::Rest => "..".to_string(),
            PatKind::Ident { mutable, name, sub } => {
                let binding = mutable
                    .then(|| "mut".to_string())
                    .into_iter()
                    .chain(once(name.clone()));
                match sub {
                    None if !mutable => name.clone(),
                    None => list("mut", once(name.clone())),
                    Some(sub) => list("in", binding.chain(once(sub.as_sexpr()))),
                }
            }
            PatKind::Lit(lit) => lit.as_sexpr(),
            PatKind::Range {
                start,
                end,
                inclusive,
            } => range(start, end, *inclusive),
            PatKind::Ref { mutable, pat } => {
                list(if *mutable { "&mut" } else { "&" }, [pat.as_sexpr()])
            }
            PatKind::Paren(pat) => list("paren", [pat.as_sexpr()]),
            PatKind::Tuple(pats) => list("tuple", sexprs(pats)),
            PatKind::Slice(pats) => list("slice", sexprs(pats)),
            PatKind::Path(path) => path.as_sexpr(),
            PatKind::TupleStruct { path, elems } => list(&path.as_sexpr(), sexprs(elems)),
            PatKind::Struct { path, fields, rest } => list(
                "struct",
                once(path.as_sexpr())
                    .chain(fields.iter().map(|f| list(&f.name, [f.pat.as_sexpr()])))
                    .chain(rest.then(|| "..".to_string())),
            ),
            PatKind::Or(pats) => list("|", sexprs(pats)),
            PatKind::Error => "error".to_string(),
        }
    }
}

impl AsSexpr for Field {
    fn as_sexpr(&self) -> String {
        match self {
            Field::Named(name) => name.clone(),
            Field::Index(index) => index.to_string(),
        }
    }
}

impl AsSexpr for Type {
    fn as_sexpr(&self) -> String {
        match &self.kind {
            TypeKind::Path(path) => path.as_sexpr(),
            TypeKind::Ref { mutable, ty } => {
                list(if *mutable { "&mut" } else { "&" }, [ty.as_sexpr()])
            }
            TypeKind::Paren(t) => list("paren", [t.as_sexpr()]),
            TypeKind::Tuple(ts) => list("tuple", sexprs(ts)),
            TypeKind::Array { elem, len } => list("array", [elem.as_sexpr(), len.as_sexpr()]),
            TypeKind::Slice(t) => list("slice", [t.as_sexpr()]),
            TypeKind::Fn { params, ret } => {
                let params = format!("({})", sexprs(params).join(" "));
                list(
                    "fn",
                    once(params).chain(ret.as_ref().map(AsSexpr::as_sexpr)),
                )
            }
            TypeKind::Dyn(bounds) => list("dyn", sexprs(bounds)),
            TypeKind::Impl(bounds) => list("impl", sexprs(bounds)),
            TypeKind::Never => "!".to_string(),
            TypeKind::Infer => "_".to_string(),
            TypeKind::Error => "error".to_string(),
        }
    }
}

impl AsSexpr for Path {
    fn as_sexpr(&self) -> String {
        let segments: Vec<_> = self
            .segments
            .iter()
            .map(|s| {
                let args = s.args.as_ref().map_or(String::new(), AsSexpr::as_sexpr);
                format!("{}{args}", s.name.as_str())
            })
            .collect();
        segments.join("::")
    }
}

impl AsSexpr for GenericArgs {
    fn as_sexpr(&self) -> String {
        match self {
            GenericArgs::Angle(args) => format!("<{}>", sexprs(args).join(", ")),
            GenericArgs::Paren { inputs, output } => {
                let output = output
                    .as_ref()
                    .map_or(String::new(), |t| format!(" -> {}", t.as_sexpr()));
                format!("({}){output}", sexprs(inputs).join(", "))
            }
        }
    }
}

impl AsSexpr for GenericArg {
    fn as_sexpr(&self) -> String {
        match self {
            GenericArg::Type(ty) => ty.as_sexpr(),
            GenericArg::Binding { name, ty } => format!("{name} = {}", ty.as_sexpr()),
            GenericArg::Const(expr) => expr.as_sexpr(),
            GenericArg::Dyn(expr) => list("dyn", [expr.as_sexpr()]),
        }
    }
}

//! AST を S 式で表す

use crate::ast::{Expr, ExprKind, Field, GenericArg, GenericArgs, Lit, Path, Type, TypeKind};
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
            } => {
                let end_point =
                    |e: &Option<Box<Expr>>| e.as_ref().map_or("_".into(), AsSexpr::as_sexpr);
                let op = if *inclusive { "..=" } else { ".." };
                list(op, [end_point(start), end_point(end)])
            }
            ExprKind::Assign { op, place, value } => {
                let op = op.map_or(String::new(), |op| format!("{op:?}"));
                list(&format!("{op}="), sexprs([place, value]))
            }
            ExprKind::Error => "error".to_string(),
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
        }
    }
}

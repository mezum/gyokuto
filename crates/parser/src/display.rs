//! AST を S 式で表示する

use crate::ast::{Expr, ExprKind, GenericArg, GenericArgs, Lit, Path, Type, TypeKind, TypePath};
use std::fmt::{self, Display, Formatter};

fn list(f: &mut Formatter, name: &str, items: &[&dyn Display]) -> fmt::Result {
    write!(f, "({name}")?;
    items.iter().try_for_each(|item| write!(f, " {item}"))?;
    write!(f, ")")
}

fn join<T: Display>(items: &[T], separator: &str) -> String {
    items
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(separator)
}

fn displays<T: Display>(items: &[T]) -> Vec<&dyn Display> {
    items.iter().map(|item| item as &dyn Display).collect()
}

impl Display for Path {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let segments: Vec<_> = self.segments.iter().map(|s| s.as_str()).collect();
        write!(f, "{}", segments.join("::"))
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self.kind {
            ExprKind::Lit(Lit::Bool(b)) => write!(f, "{b}"),
            ExprKind::Lit(lit) => write!(f, "{lit:?}"),
            ExprKind::Path(path) => write!(f, "{path}"),
            ExprKind::Paren(e) => list(f, "paren", &[e]),
            ExprKind::Tuple(es) => list(f, "tuple", &displays(es)),
            ExprKind::Array(es) => list(f, "array", &displays(es)),
            ExprKind::Repeat { elem, len } => list(f, "repeat", &[elem, len]),
            ExprKind::Unary { op, expr } => list(f, &format!("{op:?}"), &[expr]),
            ExprKind::Binary { op, lhs, rhs } => list(f, &format!("{op:?}"), &[lhs, rhs]),
            ExprKind::Range {
                start,
                end,
                inclusive,
            } => {
                let end_point =
                    |e: &Option<Box<Expr>>| e.as_ref().map_or("_".into(), |e| e.to_string());
                let op = if *inclusive { "..=" } else { ".." };
                list(f, op, &[&end_point(start), &end_point(end)])
            }
            ExprKind::Assign { op, place, value } => {
                let op = op.map_or(String::new(), |op| format!("{op:?}"));
                list(f, &format!("{op}="), &[place, value])
            }
            ExprKind::Error => write!(f, "error"),
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self.kind {
            TypeKind::Path(path) => write!(f, "{path}"),
            TypeKind::Ref { mutable, ty } => list(f, if *mutable { "&mut" } else { "&" }, &[ty]),
            TypeKind::Paren(t) => list(f, "paren", &[t]),
            TypeKind::Tuple(ts) => list(f, "tuple", &displays(ts)),
            TypeKind::Array { elem, len } => list(f, "array", &[elem, len]),
            TypeKind::Slice(t) => list(f, "slice", &[t]),
            TypeKind::Fn { params, ret } => {
                write!(f, "(fn ({})", join(params, " "))?;
                ret.iter().try_for_each(|ret| write!(f, " {ret}"))?;
                write!(f, ")")
            }
            TypeKind::Dyn(bounds) => list(f, "dyn", &displays(bounds)),
            TypeKind::Impl(bounds) => list(f, "impl", &displays(bounds)),
            TypeKind::Never => write!(f, "!"),
            TypeKind::Infer => write!(f, "_"),
            TypeKind::Error => write!(f, "error"),
        }
    }
}

impl Display for TypePath {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let segments: Vec<_> = self
            .segments
            .iter()
            .map(|s| {
                let args = s.args.as_ref().map_or(String::new(), ToString::to_string);
                format!("{}{args}", s.segment.as_str())
            })
            .collect();
        write!(f, "{}", segments.join("::"))
    }
}

impl Display for GenericArgs {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            GenericArgs::Angle(args) => write!(f, "<{}>", join(args, ", ")),
            GenericArgs::Paren { inputs, output } => {
                write!(f, "({})", join(inputs, ", "))?;
                output
                    .iter()
                    .try_for_each(|output| write!(f, " -> {output}"))
            }
        }
    }
}

impl Display for GenericArg {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            GenericArg::Type(ty) => write!(f, "{ty}"),
            GenericArg::Binding { name, ty } => write!(f, "{name} = {ty}"),
            GenericArg::Const(expr) => write!(f, "{expr}"),
        }
    }
}

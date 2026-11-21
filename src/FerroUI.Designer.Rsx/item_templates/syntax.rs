use crate::local_rust::{RustSourceScope, path_in_source};
use ferroui_designer_core::SourceSpan;
use syn::{Expr, Macro, Path, PathArguments, Stmt};

pub(super) fn template_body(
    expression: &Expr,
    scope: Option<RustSourceScope<'_>>,
) -> Option<SourceSpan> {
    let Expr::Call(factory) = expression else {
        return None;
    };
    if factory.args.len() != 1 || !factory.attrs.is_empty() {
        return None;
    }
    let Expr::Path(path) = factory.func.as_ref() else {
        return None;
    };
    let mut callee = path.clone();
    let last = callee.path.segments.last_mut()?;
    if matches!(last.arguments, PathArguments::AngleBracketed(_)) {
        last.arguments = PathArguments::None;
    }
    let name = path_in_source(&Expr::Path(callee), scope)?;
    if !matches!(
        name.as_str(),
        "ferroui_view::template" | "ferroui_view::prelude::template"
    ) {
        return None;
    }
    let Expr::Closure(closure) = &factory.args[0] else {
        return None;
    };
    if closure.inputs.len() != 1 || closure.asyncness.is_some() {
        return None;
    }
    let body = tail_macro(&closure.body)?;
    let ident = &body.path.segments.last()?.ident;
    let is_rsx = body.path.is_ident("rsx") || view_rsx(&body.path);
    is_rsx.then(|| {
        SourceSpan::new(
            ident.span().byte_range().start,
            body.delimiter.span().close().byte_range().end,
        )
    })
}

fn tail_macro(expression: &Expr) -> Option<&Macro> {
    match expression {
        Expr::Macro(body) => Some(&body.mac),
        Expr::Paren(body) => tail_macro(&body.expr),
        Expr::Block(body) => match body.block.stmts.last()? {
            Stmt::Expr(tail, None) => tail_macro(tail),
            Stmt::Macro(tail) if tail.semi_token.is_none() => Some(&tail.mac),
            _ => None,
        },
        _ => None,
    }
}

fn view_rsx(path: &Path) -> bool {
    let mut segments = path.segments.iter();
    let Some(root) = segments.next() else {
        return false;
    };
    let Some(next) = segments.next() else {
        return false;
    };
    if root.ident != "ferroui_view" || !root.arguments.is_empty() {
        return false;
    }
    let member = if next.ident == "prelude" && next.arguments.is_empty() {
        let Some(member) = segments.next() else {
            return false;
        };
        member
    } else {
        next
    };
    member.ident == "rsx" && member.arguments.is_empty() && segments.next().is_none()
}

pub(super) fn type_path(row_type: Option<&str>) -> Result<&str, String> {
    let row_type = row_type.unwrap_or("").trim();
    if row_type.len() > 512
        || row_type.contains(['\r', '\n'])
        || syn::parse_str::<syn::TypePath>(row_type).is_err()
    {
        return Err("Enter the Rust item type, such as crate::Customer or String".into());
    }
    Ok(row_type)
}

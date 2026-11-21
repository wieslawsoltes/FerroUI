use ferroui_designer_core::SourceSpan;
use syn::{Expr, ExprPath, Macro, Path, Stmt};

pub(super) fn template_body(expression: &Expr) -> Option<SourceSpan> {
    let Expr::Call(factory) = expression else {
        return None;
    };
    let Expr::Path(ExprPath {
        qself: None, path, ..
    }) = factory.func.as_ref()
    else {
        return None;
    };
    if factory.args.len() != 1 || !view_path(path, "template", true) {
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
    let is_rsx = body.path.is_ident("rsx") || view_path(&body.path, "rsx", false);
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

fn view_path(path: &Path, name: &str, generics: bool) -> bool {
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
    member.ident == name && (generics || member.arguments.is_empty()) && segments.next().is_none()
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

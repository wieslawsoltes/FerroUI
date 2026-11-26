use crate::blocks::keep_comments;
use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_designer_project::{
    file_path, file_uri, is_packaged_asset_uri, relative_asset_path, relative_asset_uri,
    relative_file_path, relative_file_reference, resolve_uri, rust_string_like, RustIncludeSyntax,
    RustModuleSources, RustPathAnchor,
};
use std::cell::OnceCell;
use syn::{Expr, Lit, LitStr, Macro, Path};

#[cfg(test)]
mod tests;

pub(crate) struct ImageReference {
    pub(crate) uri: String,
    pub(crate) preserve_expression: bool,
}

pub(crate) fn read(expression: &Expr) -> Option<String> {
    let (function, argument) = image_call(expression)?;
    if absolute_helper(function, "asset_image") {
        return packaged_uri(argument);
    }
    if !absolute_helper(function, "embedded_image") {
        return None;
    }
    relative_asset_uri(&relative_literal(include_operand(argument)?)?.value())
}

pub(crate) fn projected_reference(
    expression: &Expr,
    source: &str,
    modules: Option<&RustModuleSources>,
    includes: &OnceCell<Option<RustIncludeSyntax>>,
    resolve: impl FnOnce(&Expr) -> Option<String>,
) -> Option<ImageReference> {
    if let Some(uri) = read(expression) {
        return Some(ImageReference {
            uri,
            preserve_expression: false,
        });
    }
    let (function, argument) = image_call(expression)?;
    let embedded = absolute_helper(function, "embedded_image");
    let resolved = if embedded { None } else { resolve(function) };
    if resolved.as_deref() == Some("ferroui_view::asset_image") {
        return Some(ImageReference {
            uri: packaged_uri(argument)?,
            preserve_expression: true,
        });
    }
    if !embedded && resolved.as_deref() != Some("ferroui_view::embedded_image") {
        return None;
    }
    let site = includes
        .get_or_init(|| RustIncludeSyntax::parse(source).ok())
        .as_ref()?
        .include_bytes(include_operand(argument)?)?;
    let reference = relative_asset_uri(&site.path)?;
    let uri = match site.anchor {
        RustPathAnchor::SourceFile => Some(reference),
        RustPathAnchor::Package => {
            let modules = modules?;
            let manifest = file_uri(modules.package_root().join("Cargo.toml")).ok()?;
            let asset = resolve_uri(&manifest, &reference).ok()?;
            relative_file_reference(&modules.active().uri, &asset).ok()
        }
        _ => None,
    }?;
    Some(ImageReference {
        uri,
        preserve_expression: true,
    })
}

pub(crate) fn edit(
    source: &str,
    value: SourceSpan,
    reference: &str,
    modules: Option<&RustModuleSources>,
) -> Option<TextPatch> {
    let authored = source.get(value.start..value.end)?;
    let expression = syn::parse_str(authored).ok()?;
    // `value` is the checked field range returned by the current projection.
    // Reparse its operand to retain the original constructor and argument trivia.
    let (_, argument) = image_call(&expression)?;
    let packaged = is_packaged_asset_uri(reference);
    if let Some(literal) = string_operand(argument).filter(|_| packaged) {
        let token = literal.span().byte_range();
        return Some(TextPatch {
            span: SourceSpan::new(value.start + token.start, value.start + token.end),
            replacement: rust_string_like(authored.get(token)?, reference),
        });
    }
    if packaged || string_operand(argument).is_some() {
        return Some(TextPatch {
            span: value,
            replacement: format!("{}{}", keep_comments(authored).ok()?, rust(reference)?),
        });
    }
    let path = relative_asset_path(reference)?;
    let include = include_operand(argument)?;
    let (token, replacement) = if let Some(literal) = relative_literal(include) {
        let token = literal.span().byte_range();
        let replacement = rust_string_like(authored.get(token.clone())?, &path);
        (token, replacement)
    } else {
        let site = RustIncludeSyntax::parse(source)
            .ok()?
            .include_bytes(include)?;
        let path = match site.anchor {
            RustPathAnchor::SourceFile => path,
            RustPathAnchor::Package => {
                let modules = modules?;
                let asset = resolve_uri(&modules.active().uri, reference).ok()?;
                relative_file_path(modules.package_root(), &file_path(&asset).ok()?).ok()?
            }
            _ => return None,
        };
        let replacement = site.replacement(authored, &path);
        (site.range, replacement)
    };
    let span = SourceSpan::new(value.start + token.start, value.start + token.end);
    Some(TextPatch { span, replacement })
}

fn image_call(expression: &Expr) -> Option<(&Expr, &Expr)> {
    let Expr::Call(call) = expression else {
        return None;
    };
    if !call.attrs.is_empty() || call.args.len() != 1 {
        return None;
    }
    Some((&call.func, call.args.first()?))
}

fn include_operand(argument: &Expr) -> Option<&Macro> {
    match argument {
        Expr::Macro(image) => Some(&image.mac),
        _ => None,
    }
}

fn string_operand(argument: &Expr) -> Option<&LitStr> {
    match argument {
        Expr::Lit(literal) if literal.attrs.is_empty() => match &literal.lit {
            Lit::Str(value) => Some(value),
            _ => None,
        },
        Expr::Paren(paren) if paren.attrs.is_empty() => string_operand(&paren.expr),
        _ => None,
    }
}

fn packaged_uri(argument: &Expr) -> Option<String> {
    let uri = string_operand(argument)?.value();
    is_packaged_asset_uri(&uri).then_some(uri)
}

fn absolute_helper(function: &Expr, name: &str) -> bool {
    matches!(function, Expr::Path(function) if function.qself.is_none()
        && qualified(&function.path, &["ferroui_view", name]))
}

fn relative_literal(include: &Macro) -> Option<LitStr> {
    if qualified(&include.path, &["core", "include_bytes"])
        || qualified(&include.path, &["std", "include_bytes"])
    {
        include.parse_body().ok()
    } else {
        None
    }
}

pub(crate) fn rust(reference: &str) -> Option<String> {
    if is_packaged_asset_uri(reference) {
        return Some(format!("::ferroui_view::asset_image({reference:?})"));
    }
    let path = relative_asset_path(reference)?;
    Some(format!(
        "::ferroui_view::embedded_image(::core::include_bytes!({path:?}))"
    ))
}

fn qualified(path: &Path, names: &[&str]) -> bool {
    path.leading_colon.is_some()
        && path.segments.len() == names.len()
        && path
            .segments
            .iter()
            .zip(names)
            .all(|(segment, name)| segment.ident == *name && segment.arguments.is_empty())
}

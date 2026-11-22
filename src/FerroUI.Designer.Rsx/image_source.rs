use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_designer_project::{
    file_path, file_uri, relative_asset_path, relative_asset_uri, relative_file_path,
    relative_file_reference, resolve_uri, rust_string_like, RustIncludeSyntax, RustModuleSources,
    RustPathAnchor,
};
use std::cell::OnceCell;
use syn::{Expr, LitStr, Macro, Path};

#[cfg(test)]
mod tests;

pub(crate) struct ImageReference {
    pub(crate) uri: String,
    pub(crate) preserve_expression: bool,
}

pub(crate) fn read(expression: &Expr) -> Option<String> {
    let (function, include) = image_parts(expression)?;
    if !absolute_embedded_image(function) {
        return None;
    }
    relative_asset_uri(&relative_literal(include)?.value())
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
    let (function, include) = image_parts(expression)?;
    if !absolute_embedded_image(function)
        && resolve(function).as_deref() != Some("ferroui_view::embedded_image")
    {
        return None;
    }
    let site = includes
        .get_or_init(|| RustIncludeSyntax::parse(source).ok())
        .as_ref()?
        .include_bytes(include)?;
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
    let path = relative_asset_path(reference)?;
    let authored = source.get(value.start..value.end)?;
    let expression = syn::parse_str(authored).ok()?;
    // `value` is the checked field range returned by the current projection.
    // Reparse its operand to retain the original constructor and argument trivia.
    let (_, include) = image_parts(&expression)?;
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

fn image_parts(expression: &Expr) -> Option<(&Expr, &Macro)> {
    let Expr::Call(call) = expression else {
        return None;
    };
    if !call.attrs.is_empty() || call.args.len() != 1 {
        return None;
    }
    let Expr::Macro(image) = call.args.first()? else {
        return None;
    };
    Some((&call.func, &image.mac))
}

fn absolute_embedded_image(function: &Expr) -> bool {
    matches!(function, Expr::Path(function) if function.qself.is_none()
        && qualified(&function.path, &["ferroui_view", "embedded_image"]))
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

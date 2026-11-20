use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_designer_project::{relative_asset_path, relative_asset_uri, rust_string_like};
use syn::{Expr, LitStr, Path};

pub(crate) fn read(expression: &Expr) -> Option<String> {
    relative_asset_uri(&path_literal(expression)?.value())
}

pub(crate) fn edit(source: &str, value: SourceSpan, reference: &str) -> Option<TextPatch> {
    let path = relative_asset_path(reference)?;
    let authored = source.get(value.start..value.end)?;
    let expression = syn::parse_str(authored).ok()?;
    let token = path_literal(&expression)?.span().byte_range();
    let replacement = rust_string_like(authored.get(token.clone())?, &path);
    let span = SourceSpan::new(value.start + token.start, value.start + token.end);
    Some(TextPatch { span, replacement })
}

fn path_literal(expression: &Expr) -> Option<LitStr> {
    let Expr::Call(call) = expression else {
        return None;
    };
    let Expr::Path(function) = call.func.as_ref() else {
        return None;
    };
    if function.qself.is_some()
        || !qualified(&function.path, &["ferroui_view", "embedded_image"])
        || call.args.len() != 1
    {
        return None;
    }
    let Expr::Macro(image) = call.args.first()? else {
        return None;
    };
    if !qualified(&image.mac.path, &["core", "include_bytes"]) {
        return None;
    }
    image.mac.parse_body().ok()
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

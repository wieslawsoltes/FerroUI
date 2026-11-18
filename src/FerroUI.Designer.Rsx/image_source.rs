use ferroui_designer_project::{relative_asset_path, relative_asset_uri};
use syn::{Expr, LitStr, Path};

pub(crate) fn read(expression: &Expr) -> Option<String> {
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
    relative_asset_uri(&image.mac.parse_body::<LitStr>().ok()?.value())
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

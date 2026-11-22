use ferroui_base::{diagnostics::RsxSourceInfo, FerroObject};
use ferroui_markup_xaml::diagnostics::XamlSourceInfo;
use serde_json::{json, Value};

pub(super) fn location(object: &FerroObject) -> Option<Value> {
    XamlSourceInfo::for_object(object)
        .and_then(xaml_location)
        .or_else(|| rust_location(RsxSourceInfo::get(object)?, false))
}

pub(super) fn has_location(object: &FerroObject) -> bool {
    XamlSourceInfo::for_object(object).is_some_and(|source| source.source_uri().is_some())
        || RsxSourceInfo::get(object).is_some()
}

pub(super) fn xaml_location(info: XamlSourceInfo) -> Option<Value> {
    let uri = info.source_uri()?;
    let mut source =
        json!({"uri":uri.as_str(),"line":info.line_number(),"column":info.line_position()});
    if let Some(revision) = info.source_revision() {
        source["revision"] = json!(format!("{revision:016x}"));
    }
    Some(source)
}

pub(super) fn rust_location(info: RsxSourceInfo, use_invocation: bool) -> Option<Value> {
    let uri = info.source_uri()?;
    let location = info.location();
    let (invocation_line, invocation_column) = location.invocation;
    let (line, column) = if use_invocation {
        location.invocation
    } else {
        location.element
    };
    let mut source = json!({
        "uri":uri.as_str(), "language":"rust", "line":line, "column":column,
        "columnEncoding":"unicodeScalar",
        "invocation":{"line":invocation_line,"column":invocation_column,"index":location.index}
    });
    if let Some(revision) = location.revision {
        source["revision"] = json!(format!("{revision:016x}"));
    }
    Some(source)
}

use ferroui_base::{diagnostics::RsxSourceInfo, BoxedValue, FerroObject, Ref};
use ferroui_markup_xaml::diagnostics::XamlSourceInfo;
use serde_json::{json, Value};
use std::rc::Rc;

pub(super) fn location(object: Ref<FerroObject>) -> Value {
    let boxed: BoxedValue = Rc::new(object.clone());
    let xaml = XamlSourceInfo::get_xaml_source_info(&boxed).and_then(|info| {
        let uri = info.source_uri()?;
        Some(json!({"uri":uri.as_str(),"line":info.line_number(),"column":info.line_position()}))
    });
    xaml.or_else(|| {
        let info = RsxSourceInfo::get(&object)?;
        let uri = info.source_uri()?;
        let location = info.location();
        let (line, column) = location.element;
        let (invocation_line, invocation_column) = location.invocation;
        let mut source = json!({
            "uri":uri.as_str(), "language":"rust", "line":line, "column":column,
            "columnEncoding":"unicodeScalar",
            "invocation":{"line":invocation_line,"column":invocation_column,"index":location.index}
        });
        if let Some(revision) = location.revision {
            source["revision"] = json!(format!("{revision:016x}"));
        }
        Some(source)
    })
    .unwrap_or(Value::Null)
}

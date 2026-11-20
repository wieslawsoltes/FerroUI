use crate::{DesignerSession, property_trace::TraceTarget};
use ferroui_designer_rsx::RsxSourceMap;
use serde_json::Value;

/// Build the same read-only report for Studio or an editor's accepted projection.
pub fn property_trace_report(
    session: &DesignerSession,
    arguments: &Value,
    projection: Option<(&str, &RsxSourceMap)>,
) -> Result<Value, String> {
    let target = match (
        arguments["node"].as_u64(),
        arguments["runtime_node"].as_u64(),
    ) {
        (Some(node), None) => TraceTarget::Authored(node),
        (None, Some(id)) => TraceTarget::Runtime(id),
        _ => return Err("Choose exactly one source node or runtime node".into()),
    };
    let property = arguments["property"]
        .as_str()
        .filter(|name| (1..=256).contains(&name.len()))
        .ok_or("Choose one registered property")?;
    let trace = session.property_trace(target, property)?;
    let (source, map) = projection.map_or_else(
        || (session.authored_source(), session.rust_map()),
        |(source, map)| (source, Some(map)),
    );
    Ok(trace.json(session.document(), source, map))
}

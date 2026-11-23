//! Workspace presentation shared with Studio's Timeline toolbar.
use super::{decode, ensure_revision, schema as s};
use crate::DesignerWorkspace;
use ferroui_designer_animation::CurveMetric;
use serde_json::{json, Value};
use std::rc::Rc;

const READ: &str = "designer_timeline_view";
const SET_GRAPH: &str = "designer_timeline_set_graph";
const SET_LAYOUT: &str = "designer_timeline_set_layout";
const MODES: [&str; 3] = ["keys", "value", "velocity"];
const LAYOUTS: [&str; 2] = ["tracks", "clips"];

pub(super) fn handles(name: &str) -> bool {
    matches!(name, READ | SET_GRAPH | SET_LAYOUT)
}

pub(super) fn tools() -> [Value; 3] {
    [
        s::tool(READ, "Read Studio's Timeline layout, graph preference, and current document identity.",
            s::object(json!({}), &[]), false),
        presentation_tool(SET_GRAPH, "Open Studio's Timeline and choose Keys, Value, or Velocity. Pass the document URI and revision from designer_timeline_view.",
            "graph_mode", &MODES),
        presentation_tool(SET_LAYOUT, "Open Studio's Timeline and switch between property tracks and clip comparison. Pass the document URI and revision from designer_timeline_view.",
            "layout", &LAYOUTS),
    ]
}

fn presentation_tool(name: &str, description: &str, field: &str, choices: &[&str]) -> Value {
    let mut tool = s::tool(name, description,
        s::object(json!({(field):{"enum":choices}, "expected_uri":s::string(), "expected_revision":s::revision()}),
            &[field, "expected_uri", "expected_revision"]), true);
    tool["annotations"]["destructiveHint"] = false.into();
    tool
}

pub(super) fn call(workspace: &Rc<DesignerWorkspace>, name: &str, args: &Value) -> Result<Value, String> {
    let owner = workspace.session();
    if name != READ {
        {
            let session = owner.borrow();
            ensure_revision(&session, args)?;
            if args["expected_uri"].as_str() != Some(session.document_uri()) {
                return Err("Read the Timeline view of the current Studio document first".into());
            }
        }
        workspace.set_timeline_visible(true);
        if name == SET_GRAPH {
            let metric = if args["graph_mode"] == "keys" { None } else { Some(decode(args, "graph_mode")?) };
            workspace.set_timeline_curves(metric);
        } else {
            workspace.set_clip_comparison(args["layout"] == "clips");
        }
    }
    let session = owner.borrow();
    let mode = match workspace.timeline_curve_metric() {
        None => "keys",
        Some(CurveMetric::Value) => "value",
        Some(CurveMetric::Velocity) => "velocity",
    };
    let layout = if workspace.comparing_animation_clips() { "clips" } else { "tracks" };
    Ok(json!({"mode":mode, "layout":layout, "available_modes":MODES, "visible":workspace.timeline_visible(),
        "uri":session.document_uri(), "revision":session.document().revision()}))
}

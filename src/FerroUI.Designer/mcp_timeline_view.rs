//! Workspace presentation shared with Studio's Timeline toolbar.
use super::{decode, ensure_revision, schema as s};
use crate::DesignerWorkspace;
use ferroui_designer_animation::CurveMetric;
use serde_json::{json, Value};
use std::rc::Rc;

const READ: &str = "designer_timeline_view";
const SET_GRAPH: &str = "designer_timeline_set_graph";
const SET_LAYOUT: &str = "designer_timeline_set_layout";
const SET_VIEWPORT: &str = "designer_timeline_set_viewport";
const MODES: [&str; 3] = ["keys", "value", "velocity"];
const LAYOUTS: [&str; 2] = ["tracks", "clips"];

pub(super) fn handles(name: &str) -> bool {
    matches!(name, READ | SET_GRAPH | SET_LAYOUT | SET_VIEWPORT)
}

pub(super) fn tools() -> [Value; 4] {
    [
        s::tool(READ, "Read Studio's Timeline layout, graph preference, document identity and visible ruler viewport.",
            s::object(json!({}), &[]), false),
        presentation_tool(SET_GRAPH, "Open Studio's Timeline and choose Keys, Value, or Velocity. Pass the document URI and revision from designer_timeline_view.",
            "graph_mode", &MODES),
        presentation_tool(SET_LAYOUT, "Open Studio's Timeline and switch between property tracks and clip comparison. Pass the document URI and revision from designer_timeline_view.",
            "layout", &LAYOUTS),
        presentation_tool_with_schema(SET_VIEWPORT, "Open Studio's Timeline and set the visible ruler viewport. Use ruler, program if present, URI and revision from designer_timeline_view. Window takes start and end in seconds. Fit and fit_selection use the corresponding view controls.",
            "viewport", s::object(json!({"ruler":{"enum":LAYOUTS}, "program":s::revision(),
                "action":{"enum":["window","fit","fit_selection"]}, "start":{"type":"number","minimum":0}, "end":{"type":"number","minimum":0}}),
                &["ruler","action"])),
    ]
}

fn presentation_tool(name: &str, description: &str, field: &str, choices: &[&str]) -> Value {
    presentation_tool_with_schema(name, description, field, json!({"enum":choices}))
}

fn presentation_tool_with_schema(name: &str, description: &str, field: &str, schema: Value) -> Value {
    let mut tool = s::tool(name, description,
        s::object(json!({(field):schema, "expected_uri":s::string(), "expected_revision":s::revision()}),
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
        if name == SET_GRAPH {
            let metric = if args["graph_mode"] == "keys" { None } else { Some(decode(args, "graph_mode")?) };
            workspace.set_timeline_visible(true);
            workspace.set_timeline_curves(metric);
        } else if name == SET_LAYOUT {
            workspace.set_timeline_visible(true);
            workspace.set_clip_comparison(args["layout"] == "clips");
        } else {
            workspace.change_timeline_viewport(decode(args, "viewport")?)?;
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
        "uri":session.document_uri(), "revision":session.document().revision(), "viewport":workspace.timeline_viewport()}))
}

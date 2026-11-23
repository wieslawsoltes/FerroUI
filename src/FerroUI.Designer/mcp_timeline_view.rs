//! Workspace presentation shared with Studio's graph menu.
use super::{decode, ensure_revision, schema as s};
use crate::DesignerWorkspace;
use ferroui_designer_animation::CurveMetric;
use serde_json::{json, Value};
use std::rc::Rc;

const READ: &str = "designer_timeline_view";
const SET: &str = "designer_timeline_set_graph";
const MODES: [&str; 3] = ["keys", "value", "velocity"];

pub(super) fn handles(name: &str) -> bool {
    matches!(name, READ | SET)
}

pub(super) fn tools() -> [Value; 2] {
    let mut set = s::tool(SET, "Open Studio's Timeline and choose Keys, Value, or Velocity. Pass the document URI and revision from designer_timeline_view.",
        s::object(json!({"graph_mode":{"enum":MODES}, "expected_uri":s::string(), "expected_revision":s::revision()}),
            &["graph_mode", "expected_uri", "expected_revision"]), true);
    set["annotations"]["destructiveHint"] = false.into();
    [
        s::tool(READ, "Read Studio's Timeline graph preference and current document identity.",
            s::object(json!({}), &[]), false),
        set,
    ]
}

pub(super) fn call(workspace: &Rc<DesignerWorkspace>, name: &str, args: &Value) -> Result<Value, String> {
    let owner = workspace.session();
    if name == SET {
        {
            let session = owner.borrow();
            ensure_revision(&session, args)?;
            if args["expected_uri"].as_str() != Some(session.document_uri()) {
                return Err("Read the Timeline view of the current Studio document first".into());
            }
        }
        let metric = if args["graph_mode"] == "keys" {
            None
        } else {
            Some(decode(args, "graph_mode")?)
        };
        workspace.set_timeline_visible(true);
        workspace.set_timeline_curves(metric);
    }
    let session = owner.borrow();
    let mode = match workspace.timeline_curve_metric() {
        None => "keys",
        Some(CurveMetric::Value) => "value",
        Some(CurveMetric::Velocity) => "velocity",
    };
    Ok(json!({"mode":mode, "available_modes":MODES, "visible":workspace.timeline_visible(),
        "uri":session.document_uri(), "revision":session.document().revision()}))
}

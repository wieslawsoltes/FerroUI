//! Read-only draft operations. A completed brush uses the existing component edit.
use super::{decode, node_id, schema as s, text};
use crate::{editors::brush_draft::BrushDraft, DesignerSession};
use serde_json::{json, Value};

pub(super) const NAME: &str = "designer_brush_draft";

pub(super) fn tool() -> Value {
    let change = |name, mut properties: Value, required: &[&str]| {
        properties["edit"] = json!({"const":name});
        let mut required = required.to_vec();
        required.push("edit");
        s::object(properties, &required)
    };
    s::actions(NAME, "Open or change a detached Studio brush draft. Read the typed brush and gradient stops, edit their properties, or add, move and remove stops. Publish the returned source with designer_component set_object_property after review.", vec![
        s::action("open", json!({"node":s::id(),"property":s::string()}), &["node", "property"], false),
        s::action("change", json!({"source":s::string(),"change":{"oneOf":[
            change("preset", json!({"kind":s::string()}), &["kind"]),
            change("set", json!({"node":s::id(),"property":s::string(),"value":s::string()}), &["node", "property", "value"]),
            change("insert_stop", json!({}), &[]),
            change("remove_stop", json!({"node":s::id()}), &["node"]),
            change("move_stop", json!({"node":s::id(),"direction":{"enum":[-1,1]}}), &["node", "direction"])
        ]}}), &["source", "change"], false),
    ], false)
}

pub(super) fn call(session: &DesignerSession, args: &Value) -> Result<Value, String> {
    let model = match args["action"].as_str() {
        Some("open") => BrushDraft::open(
            session.document(),
            node_id(args, "node")?,
            &text(args, "property")?,
        )?,
        Some("change") => BrushDraft::change(&text(args, "source")?, decode(args, "change")?)?,
        _ => return Err("Choose a brush draft action".into()),
    };
    serde_json::to_value(model).map_err(|error| error.to_string())
}

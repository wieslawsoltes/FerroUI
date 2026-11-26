//! Grid definitions and child cell edits share the native Studio transaction.
use super::{integer, node_id, schema as s, state, text};
use crate::{DesignerSession, NodeId};
use ferroui_controls::Grid;
use ferroui_designer_tools::{grid_track_values, Axis, GridLength, GridTrackChange};
use serde_json::{json, Value};

pub(super) fn handles(name: &str) -> bool {
    matches!(name, "designer_grid_tracks" | "designer_grid_track_edit")
}

pub(super) fn tools() -> Vec<Value> {
    let axes = json!({"enum":["rows","columns"]});
    let schema = json!({"panel":s::id(),"axis":axes,"index":{"type":"integer","minimum":0},"length":s::string()});
    vec![
        s::tool(
            "designer_grid_tracks",
            "Find the nearest authored Grid and inspect its current row and column definitions.",
            s::object(json!({"node":s::id()}), &["node"]),
            false,
        ),
        s::actions(
            "designer_grid_track_edit",
            "Set a Grid track length, insert a track before a zero-based index, or remove a track. Uses Studio's measured layout to update affected child placements. A bound cell or span rejects structural edits.",
            vec![
                s::action("set", schema.clone(), &["panel","axis","index","length"], true),
                s::action("insert", schema.clone(), &["panel","axis","index","length"], true),
                s::action("remove", json!({"panel":schema["panel"],"axis":schema["axis"],"index":schema["index"]}),
                    &["panel","axis","index"], true),
            ],
            true,
        ),
    ]
}

pub(super) fn call(
    session: &mut DesignerSession,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    if name == "designer_grid_tracks" {
        let panel = nearest_grid(session, node_id(args, "node")?);
        return Ok(json!({
            "panel":panel,
            "writable":session.can_manipulate(),
            "axes":panel.map(|panel| vec![
                axis_model(session, panel, Axis::Vertical, "rows"),
                axis_model(session, panel, Axis::Horizontal, "columns"),
            ]).unwrap_or_default(),
        }));
    }
    let panel = node_id(args, "panel")?;
    let axis = match args["axis"].as_str() {
        Some("rows") => Axis::Vertical,
        Some("columns") => Axis::Horizontal,
        _ => return Err("Choose Grid rows or columns".into()),
    };
    let index = usize::try_from(integer(args, "index")?).map_err(|_| "Invalid Grid track index")?;
    let length = || GridLength::parse(&text(args, "length")?).map_err(|error| error.to_string());
    let change = match args["action"].as_str() {
        Some("set") => GridTrackChange::Set {
            index,
            length: length()?,
        },
        Some("insert") => GridTrackChange::Insert {
            index,
            length: length()?,
        },
        Some("remove") => GridTrackChange::Remove { index },
        _ => return Err("Unknown Grid track action".into()),
    };
    session.edit_grid_tracks(panel, axis, change)?;
    Ok(state(session))
}

fn nearest_grid(session: &DesignerSession, mut node: NodeId) -> Option<NodeId> {
    let ast = session.document().ast();
    loop {
        let authored = ast.node(node)?;
        if session
            .preview
            .visual(node)
            .and_then(|visual| visual.cast::<Grid>())
            .is_some()
        {
            return Some(node);
        }
        node = authored.parent?;
    }
}

fn axis_model(session: &DesignerSession, panel: NodeId, axis: Axis, name: &str) -> Value {
    match grid_track_values(session.document(), panel, axis) {
        Ok(values) => {
            let tracks = values
                .into_iter()
                .map(|length| {
                    json!({
                        "editable":GridLength::parse(&length).is_ok(),"length":length
                    })
                })
                .collect::<Vec<_>>();
            json!({"axis":name,"tracks":tracks})
        }
        Err(error) => json!({"axis":name,"error":error.to_string()}),
    }
}

use super::{node_id, schema as s, state, text};
use crate::DesignerSession;
use ferroui_designer_language::{Schema, plan_event_handler, routed_events};
use serde_json::{Value, json};
use std::sync::OnceLock;

pub(super) fn handles(name: &str) -> bool {
    matches!(name, "designer_events" | "designer_set_event")
}

pub(super) fn tools() -> Vec<Value> {
    vec![
        s::tool(
            "designer_events",
            "List routed events and code-behind handler declarations for an authored element.",
            s::object(json!({"node":s::id()}), &["node"]),
            false,
        ),
        s::actions(
            "designer_set_event",
            "Assign or clear a named XAML event handler using the current document revision.",
            vec![
                s::action(
                    "set",
                    json!({"node":s::id(),"event":s::string(),"handler":s::string(),"expected_revision":s::revision()}),
                    &["node", "event", "handler", "expected_revision"],
                    true,
                ),
                s::action(
                    "clear",
                    json!({"node":s::id(),"event":s::string(),"expected_revision":s::revision()}),
                    &["node", "event", "expected_revision"],
                    true,
                ),
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
    static SCHEMA: OnceLock<Schema> = OnceLock::new();
    let schema = SCHEMA.get_or_init(Schema::ferroui);
    let document = session.document();
    let node = document
        .ast()
        .node(node_id(args, "node")?)
        .ok_or("Select an authored element")?;
    if name == "designer_events" {
        let rows: Vec<_> = routed_events(schema, document, node)
            .into_iter()
            .map(|(name, info)| {
                let attribute = node.attribute_info(&name);
                json!({
                    "name": name, "argument_type": info.type_name,
                    "handler": attribute.map(|attribute| &attribute.value),
                    "value_span": attribute.map(|attribute| attribute.value_span),
                })
            })
            .collect();
        return Ok(json!({"revision":document.revision(),"events":rows}));
    }
    let handler = if args["action"] == "set" {
        Some(text(args, "handler")?)
    } else {
        None
    };
    let event = text(args, "event")?;
    let edit = plan_event_handler(schema, document, node, &event, handler.as_deref())?;
    session.apply_visual("Edit event handler", vec![edit])?;
    Ok(state(session))
}

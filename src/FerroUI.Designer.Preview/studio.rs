use crate::document::DesignDocument;
use crate::edits::editor_edits;
use crate::protocol::{EditResult, Source};
use crate::renderer::{Renderer, Stage};
use ferroui_designer::mcp::DesignerMcp;
use ferroui_designer_core::{NodeId, TextPatch};
use ferroui_mcp::{Permissions, UiToolProvider};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[derive(Deserialize)]
pub(crate) struct Query {
    pub(crate) frame: u64,
    pub(crate) name: String,
    pub(crate) arguments: Value,
    pub(crate) selection: Option<Vec<NodeId>>,
}

#[derive(Deserialize)]
pub(crate) struct Edit {
    #[serde(flatten)]
    document: Source,
    #[serde(flatten)]
    query: Query,
}

pub(crate) fn catalog() -> Value {
    let (tools, edits): (Vec<_>, Vec<_>) = DesignerMcp::session_tool_definitions()
        .into_iter()
        .partition(is_query);
    json!({"tools":tools,"xamlEdits":edits.into_iter().filter_map(edit_definition).collect::<Vec<_>>()})
}

fn is_query(tool: &Value) -> bool {
    tool["annotations"]["readOnlyHint"] == true
        && !matches!(
            tool["name"].as_str(),
            Some("designer_document" | "designer_collaboration_state")
        )
}

pub(crate) fn query(stage: &Stage, query: &Query) -> Result<Value, String> {
    let provider = DesignerMcp::new(&stage.session);
    if !DesignerMcp::session_tool_definitions()
        .iter()
        .any(|definition| definition["name"] == query.name && is_query(definition))
    {
        return Err(
            "Use the corresponding editor action or the attached Designer Studio MCP server".into(),
        );
    }
    if let Some(selection) = &query.selection {
        select(stage, selection)?;
    }
    provider.call(&query.name, &query.arguments, Permissions::default())
}

pub(crate) fn edit(renderer: &mut Renderer, request: Edit) -> Result<EditResult, String> {
    let design = DesignDocument::parse(&request.document)?;
    if design.rsx.is_some() {
        return Err("Studio authoring edits require XAML. Use the mapped designer tools or edit Rust source for RSX.".into());
    }
    let query = request.query;
    let stage = renderer.stage(query.frame)?;
    if !source_action(&query.name, query.arguments["action"].as_str()) {
        return Err("Choose a source edit from the Studio authoring catalog".into());
    }
    let mut arguments = query.arguments;
    let properties = arguments
        .as_object_mut()
        .ok_or("arguments must be an object")?;
    {
        let session = stage.session.borrow();
        if session.document_uri() != request.document.uri
            || session.document().source() != design.source
        {
            return Err("The rendered design does not match the current source".into());
        }
        properties.insert(
            "expected_revision".into(),
            json!(session.document().revision()),
        );
    }
    let provider = DesignerMcp::new(&stage.session);
    provider.validate(&query.name, &arguments)?;
    if let Some(selection) = &query.selection {
        select(stage, selection)?;
    }
    // Retire the frame before touching the native session. VS Code owns the undo
    // transaction and renders a new frame from its buffer even on rejection.
    let stage = renderer.stage.take().expect("frame checked above");
    provider.call(
        &query.name,
        &arguments,
        Permissions {
            allow_property_writes: true,
            ..Default::default()
        },
    )?;
    let session = stage.session.borrow();
    let patch = TextPatch::between(design.source, session.document().source());
    editor_edits(&design, patch.into_iter().collect())
}

fn select(stage: &Stage, selection: &[NodeId]) -> Result<(), String> {
    let mut session = stage.session.borrow_mut();
    let mut distinct = BTreeSet::new();
    if selection.len() > 128
        || selection.iter().any(|&id| {
            !distinct.insert(id)
                || !session
                    .document()
                    .ast()
                    .node(id)
                    .is_some_and(|node| node.is_element())
        })
    {
        return Err("Select up to 128 distinct authored nodes from the current frame".into());
    }
    if session.selection() != selection {
        session.select(selection.iter().copied());
    }
    Ok(())
}

fn source_action(tool: &str, action: Option<&str>) -> bool {
    match tool {
        "designer_program" => matches!(
            action,
            Some("create" | "rename" | "duplicate" | "remove" | "selector" | "source" | "update")
        ),
        "designer_state" => matches!(action, Some("upsert" | "remove")),
        "designer_behavior" => matches!(action, Some("upsert" | "remove" | "source")),
        "designer_design_data_edit" => {
            matches!(action, Some("set_value" | "set_object" | "sample_items"))
        }
        // All actions for these schemas are source-backed.
        "designer_component"
        | "designer_property"
        | "designer_set_event"
        | "designer_focus_edit"
        | "designer_selection_layout_edit"
        | "designer_property_transfer_edit" => true,
        _ => false,
    }
}

fn edit_definition(mut tool: Value) -> Option<Value> {
    let name = tool["name"].as_str()?.to_owned();
    let schema = &mut tool["inputSchema"];
    if let Some(actions) = schema.get_mut("oneOf").and_then(Value::as_array_mut) {
        actions.retain(|action| {
            source_action(&name, action["properties"]["action"]["const"].as_str())
        });
        if actions.is_empty() {
            return None;
        }
        let (names, keys): (Vec<_>, BTreeSet<_>) = (
            actions
                .iter()
                .map(|action| action["properties"]["action"]["const"].clone())
                .collect(),
            actions
                .iter()
                .flat_map(|action| action["properties"].as_object().unwrap().keys().cloned())
                .collect(),
        );
        actions.iter_mut().for_each(hide_native_revision);
        schema["properties"]
            .as_object_mut()
            .unwrap()
            .retain(|key, _| keys.contains(key));
        schema["properties"]["action"]["enum"] = json!(names);
    } else if !source_action(&name, None) {
        return None;
    }
    hide_native_revision(schema);
    Some(tool)
}

fn hide_native_revision(schema: &mut Value) {
    schema["required"]
        .as_array_mut()
        .unwrap()
        .retain(|key| key != "expected_revision");
    schema["properties"]
        .as_object_mut()
        .unwrap()
        .remove("expected_revision");
}

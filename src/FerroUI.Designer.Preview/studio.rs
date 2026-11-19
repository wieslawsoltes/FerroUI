use crate::document::DesignDocument;
use crate::edits::editor_edits;
use crate::protocol::{EditResult, Source};
use crate::renderer::{Presentation, Renderer, Stage};
use crate::rsx_animation;
use ferroui_designer::mcp::DesignerMcp;
use ferroui_designer_core::{NodeId, TextPatch};
use ferroui_mcp::{Permissions, UiToolProvider};
use serde::Deserialize;
use serde_json::{Value, json};
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Present {
    #[serde(flatten)]
    query: Query,
    next_frame: u64,
}

pub(crate) fn catalog() -> Value {
    let (tools, edits): (Vec<_>, Vec<_>) = DesignerMcp::session_tool_definitions()
        .into_iter()
        .partition(is_query);
    let presentations = edits.iter().cloned().filter_map(|tool| action_definition(tool, presentation_action)).collect::<Vec<_>>();
    let rsx_edits = edits.iter().filter(|tool| tool["name"] == "designer_program").cloned()
        .filter_map(|tool| action_definition(tool, rsx_animation::keyframe_edit)).collect::<Vec<_>>();
    json!({"tools":tools,"presentations":presentations,"rsxEdits":rsx_edits,
        "xamlEdits":edits.into_iter().filter_map(|tool| action_definition(tool, source_action)).collect::<Vec<_>>()})
}

pub(crate) fn present(renderer: &mut Renderer, request: Present) -> Result<Presentation, String> {
    let query = request.query;
    if request.next_frame <= query.frame || !presentation_action(&query.name, query.arguments["action"].as_str()) {
        return Err("Choose a current Studio presentation action".into());
    }
    let stage = renderer.stage(query.frame)?;
    let mut arguments = query.arguments;
    let revision = stage.session.borrow().document().revision();
    arguments.as_object_mut().ok_or("arguments must be an object")?
        .insert("expected_revision".into(), json!(revision));
    let provider = DesignerMcp::new(&stage.session);
    provider.validate(&query.name, &arguments)?;
    if let Some(selection) = &query.selection { select(stage, selection)?; }
    let result = stage.sources.run(|| provider.call(&query.name, &arguments, Permissions { allow_property_writes: true, ..Default::default() }))?;
    let mut presentation = renderer.present(request.next_frame, query.name == "designer_layer")?;
    presentation.timeline_position = result.get("timelinePosition").cloned();
    presentation.timeline_targets = result.get("timelineTargets").cloned();
    presentation.transition_position = result.get("transitionPosition").cloned();
    Ok(presentation)
}

fn presentation_action(tool: &str, action: Option<&str>) -> bool {
    match tool {
        "designer_program" => matches!(
            action,
            Some("preview" | "preview_targets" | "scrub" | "play_program" | "advance_program" | "pause_program" | "stop_program" | "play_all")
        ),
        "designer_state" => matches!(action, Some("preview" | "preview_transition" | "advance_preview" | "clear_preview")),
        "designer_definition" => matches!(action, Some("enter" | "exit")),
        "designer_design_data_edit" => matches!(action, Some("enabled" | "fixture")),
        "designer_layer" => action.is_some(),
        _ => false,
    }
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
    if let Some(rsx) = stage.authored_rsx.as_ref().filter(|_| query.name == "designer_keyframe_draft") {
        rsx.patches(&query.arguments)?;
    }
    let mut result = provider.call(&query.name, &query.arguments, Permissions::default())?;
    if let Some(rsx) = stage.authored_rsx.as_ref().filter(|_| query.name == "designer_programs") {
        rsx.annotate_programs(&mut result);
    }
    Ok(result)
}

pub(crate) fn edit(renderer: &mut Renderer, request: Edit, projects: &crate::projects::Projects) -> Result<EditResult, String> {
    let design = DesignDocument::parse(&request.document, projects.source(&request.document)?)?;
    let query = request.query;
    let rust_keyframe = design.rsx.is_some() && rsx_animation::keyframe_edit(&query.name, query.arguments["action"].as_str());
    if design.rsx.is_some()
        && !rust_keyframe
        && (!matches!(query.name.as_str(), "designer_selection_layout_edit" | "designer_property" | "designer_grid_track_edit")
            || !design.editable_structure())
    {
        return Err("This Studio operation requires XAML or a fully mapped RSX layout.".into());
    }
    let stage = renderer.stage(query.frame)?;
    if let Some(rsx) = &stage.authored_rsx {
        rsx.require_source(&design)?;
    }
    if !source_action(&query.name, query.arguments["action"].as_str()) {
        return Err("Choose a source edit from the Studio authoring catalog".into());
    }
    let mut arguments = query.arguments;
    let properties = arguments
        .as_object_mut()
        .ok_or("arguments must be an object")?;
    {
        let session = stage.session.borrow();
        if stage.project != request.document.project
            || session.document_uri() != request.document.uri
            || session.document().source() != design.xaml.source()
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
    if rust_keyframe {
        let rsx = stage.authored_rsx.as_ref().ok_or("Render the current Rust animation before editing")?;
        let patches = rsx.patches(&arguments)?;
        // Studio checks the native target, timeline and easing. The result is applied
        // to Rust by the editor without mutating the generated projection.
        let mut draft = arguments;
        let object = draft.as_object_mut().expect("validated action");
        object.remove("action");
        object.remove("expected_revision");
        provider.call("designer_keyframe_draft", &draft, Permissions::default())?;
        let result = editor_edits(&design, patches)?;
        if !result.edits.is_empty() {
            renderer.stage.take();
        }
        return Ok(result);
    }
    // Retire the frame before touching the native session. VS Code owns the undo
    // transaction and renders a new frame from its buffer even on rejection.
    let stage = renderer.stage.take().expect("frame checked above");
    stage.sources.run(|| provider.call(
        &query.name,
        &arguments,
        Permissions {
            allow_property_writes: true,
            ..Default::default()
        },
    ))?;
    let session = stage.session.borrow();
    let source = session.document().source();
    let patches = if let Some(projection) = &design.rsx {
        projection.apply_xaml(source)?
    } else {
        TextPatch::between(design.source, source)
            .into_iter()
            .collect()
    };
    let recorded = matches!(query.name.as_str(), "designer_record_property" | "designer_record_geometry")
        || query.name == "designer_program" && arguments["action"] == "record_keyframe";
    let retained_selection = if recorded {
        session.selection().iter().filter_map(|&node| crate::edit_selection::authored_range(
            &design, session.document(), &patches, node)).collect()
    } else { Vec::new() };
    let mut edit = editor_edits(&design, patches)?;
    if recorded { edit.timeline_pose = session.timeline_preview_pose(); }
    edit.retained_selection = retained_selection;
    Ok(edit)
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
            Some("create" | "rename" | "duplicate" | "remove" | "selector" | "source" | "update" | "update_timing" |
                "upsert_keyframe" | "record_keyframe" | "remove_keyframe" | "move_keyframes" | "duplicate_keyframes" | "remove_keyframes")
        ),
        "designer_state" => matches!(action, Some("upsert" | "remove")),
        "designer_behavior" => matches!(action, Some("upsert" | "remove" | "source")),
        "designer_design_data_edit" => {
            matches!(action, Some("set_value" | "set_object" | "sample_items"))
        }
        // All actions for these schemas are source-backed.
        "designer_component"
        | "designer_property"
        | "designer_record_property"
        | "designer_record_geometry"
        | "designer_binding_edit"
        | "designer_grid_track_edit"
        | "designer_set_event"
        | "designer_focus_edit"
        | "designer_selection_layout_edit"
        | "designer_property_transfer_edit" => true,
        _ => false,
    }
}

fn action_definition(mut tool: Value, allowed: impl Fn(&str, Option<&str>) -> bool) -> Option<Value> {
    let name = tool["name"].as_str()?.to_owned();
    let schema = &mut tool["inputSchema"];
    if let Some(actions) = schema.get_mut("oneOf").and_then(Value::as_array_mut) {
        actions.retain(|action| {
            allowed(&name, action["properties"]["action"]["const"].as_str())
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
    } else if !allowed(&name, None) {
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

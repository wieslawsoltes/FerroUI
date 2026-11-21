use crate::document::DesignDocument;
use crate::edits::{editor_edits, module_edits, rust_name_edit};
use crate::protocol::{EditResult, Source};
use crate::renderer::{Presentation, Renderer, Stage};
use crate::rsx_animation;
use ferroui_designer::mcp::{DesignerMcp, RSX_STYLES, RSX_STYLE_EDIT};
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
pub(crate) struct RustModuleSourceCheck {
    frame: u64,
    uri: String,
    source: String,
}

pub(crate) fn verify_rust_module_source(renderer: &Renderer, check: RustModuleSourceCheck) -> Result<(), String> {
    renderer.stage(check.frame)?.authored_rsx.as_ref()
        .ok_or("Render the current Rust invocation first")?
        .require_module_source(&check.uri, &check.source)
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
    let rsx_edits = edits.iter().filter(|tool| matches!(tool["name"].as_str(), Some("designer_program" | "designer_state" | "designer_item_template" | RSX_STYLE_EDIT))).cloned()
        .filter_map(|tool| action_definition(tool, |name, action| rsx_animation::program_edit(name, action)
            || rsx_animation::keyframe_edit(name, action) || rsx_animation::state_edit(name, action)
            || rsx_animation::clip_placement_edit(name, action) || rust_template_edit(name, action) || name == RSX_STYLE_EDIT))
        .collect::<Vec<_>>();
    json!({"tools":tools,"presentations":presentations,"rsxEdits":rsx_edits,
        "xamlEdits":edits.into_iter().filter(|tool| tool["name"] != RSX_STYLE_EDIT)
            .filter_map(|tool| action_definition(tool, source_action)).collect::<Vec<_>>()})
}

pub(crate) fn present(renderer: &mut Renderer, request: Present) -> Result<Presentation, String> {
    let query = request.query;
    if request.next_frame <= query.frame || !presentation_action(&query.name, query.arguments["action"].as_str()) {
        return Err("Choose a current Studio presentation action".into());
    }
    let stage = renderer.stage(query.frame)?;
    if query.name == "designer_open_style_source" && stage.authored_rsx.is_some() {
        return Err("Edit Rust-owned style builders in Rust source.".into());
    }
    if query.name == "designer_behavior" && stage.authored_rsx.is_some() {
        return Err("Behavior previews require an authored XAML view".into());
    }
    let mut arguments = query.arguments;
    let revision = stage.session.borrow().document().revision();
    arguments.as_object_mut().ok_or("arguments must be an object")?
        .insert("expected_revision".into(), json!(revision));
    let provider = DesignerMcp::new(&stage.session);
    provider.validate(&query.name, &arguments)?;
    if let Some(selection) = &query.selection { select(stage, selection)?; }
    if let Some(rust) = stage.authored_rsx.as_ref().filter(|_| query.name == "designer_item_template") {
        let projection = rust.projection()?;
        let navigation = stage.sources.run(|| ferroui_designer::mcp::rust_template_navigation(
            &stage.session.borrow(), rust.source(), &projection, &arguments,
        ))?;
        let mut result = renderer.present(request.next_frame, false)?;
        result.open_rust_template = Some(navigation);
        return Ok(result);
    }
    let route_source = if query.name == "designer_state" && arguments["action"] == "preview_transition" {
        stage.authored_rsx.as_ref().and_then(|source| source.state_route_range(&arguments))
    } else { None };
    let permissions = Permissions {
        allow_property_writes: true,
        allow_input: query.name == "designer_behavior" && arguments["action"] == "preview",
    };
    let result = stage.sources.run(|| provider.call(&query.name, &arguments, permissions))?;
    let mut presentation = renderer.present(request.next_frame, query.name == "designer_layer")?;
    presentation.timeline_position = result.get("timelinePosition").cloned();
    presentation.timeline_targets = result.get("timelineTargets").cloned();
    presentation.transition_position = result.get("transitionPosition").cloned();
    presentation.open_definition = result.get("openDefinition").cloned();
    presentation.return_to_consumer = result["returnToConsumer"].as_str().map(str::to_owned);
    if let (Some(position), Some(range)) = (&mut presentation.transition_position, route_source) {
        position["sourceRange"] = json!(range);
    }
    Ok(presentation)
}

fn presentation_action(tool: &str, action: Option<&str>) -> bool {
    match tool {
        "designer_program" => matches!(
            action,
            Some("preview" | "preview_targets" | "scrub" | "scrub_all" | "play_program" | "play_range" | "advance_program" | "pause_program" | "stop_program" | "play_all")
        ),
        "designer_state" => matches!(action, Some("preview" | "preview_transition" | "advance_preview" | "clear_preview")),
        "designer_behavior" => matches!(action, Some("preview" | "clear_preview")),
        "designer_definition" => matches!(action, Some("enter" | "enter_included_template" | "exit")),
        "designer_item_template" => action == Some("preview"),
        "designer_open_style_source" => action.is_none(),
        "designer_design_data_edit" => matches!(action, Some("enabled" | "fixture")),
        "designer_layer" => action.is_some(),
        _ => false,
    }
}

fn is_query(tool: &Value) -> bool {
    tool["annotations"]["readOnlyHint"] == true
        && !matches!(
            tool["name"].as_str(),
            Some("designer_document" | "designer_collaboration_state" | "designer_rsx_views")
        )
}

pub(crate) fn query(stage: &Stage, query: &Query) -> Result<Value, String> {
    if !DesignerMcp::session_tool_definitions()
        .iter()
        .any(|definition| definition["name"] == query.name && is_query(definition))
    {
        return Err(
            "Use the corresponding editor action or the attached Designer Studio MCP server".into(),
        );
    }
    let needs_selection = match query.name.as_str() {
        "designer_style_catalog" => query.arguments.get("target").is_none(),
        "designer_style_definition" | "designer_style_source" | "designer_style_resources" | "designer_palette"
        | "designer_item_templates" | RSX_STYLES => false,
        _ => true,
    };
    // A direct style or template lookup can run while another view holds a
    // selection-based plan. Leave its selection in place.
    if let (true, Some(selection)) = (needs_selection, &query.selection) {
        select(stage, selection)?;
    }
    if query.name == "designer_preview_checks" {
        return crate::preview_checks::query(stage, &query.arguments);
    }
    if query.name == "designer_behaviors" && stage.authored_rsx.is_some() {
        return Err("Behavior collections use authored XAML".into());
    }
    if query.name == RSX_STYLES {
        stage.studio.validate(&query.name, &query.arguments)?;
        let rust = stage.authored_rsx.as_ref().ok_or("Open an authored RSX view")?;
        let node = query.arguments["node"].as_u64().and_then(|node| usize::try_from(node).ok());
        return ferroui_designer::rust_style_report(rust.source(), &rust.projection()?, node);
    }
    if let Some(rust) = stage.authored_rsx.as_ref().filter(|_| query.name == "designer_property_trace") {
        stage.studio.validate(&query.name, &query.arguments)?;
        let project = rust.projection()?;
        let (_, map) = project.mapped_markup();
        return stage.sources.run(|| ferroui_designer::mcp::property_trace_report(
            &stage.session.borrow(), &query.arguments, Some((rust.source(), &map)),
        ));
    }
    if let Some(rust) = stage.authored_rsx.as_ref().filter(|_| query.name == "designer_columns") {
        stage.studio.validate(&query.name, &query.arguments)?;
        let node = query.arguments["node"].as_u64().ok_or("Select an authored table")?;
        let projection = rust.projection()?;
        return stage.sources.run(|| ferroui_designer::mcp::columns_report(
            &stage.session.borrow(), node, Some((rust.source(), &projection)),
        ));
    }
    if let Some(rust) = stage.authored_rsx.as_ref().filter(|_| query.name == "designer_item_templates") {
        stage.studio.validate(&query.name, &query.arguments)?;
        let projection = rust.projection()?;
        return stage.sources.run(|| ferroui_designer::mcp::rust_template_report(
            &stage.session.borrow(), rust.source(), &projection, &query.arguments,
        ));
    }
    if stage.authored_rsx.is_some() && matches!(query.name.as_str(),
        "designer_style_catalog" | "designer_style_definition" | "designer_style_source" | "designer_style_resources" | "designer_palette")
    {
        return Err("Edit Rust-owned style builders in Rust source".into());
    }
    if query.name == "designer_wrap_selection" {
        let rsx = stage.authored_rsx.as_ref().map(crate::rsx_animation::AuthoredRsxFrame::projection).transpose()?;
        return stage.sources.run(|| stage.studio.wrap_query(&query.arguments, rsx));
    }
    if let Some(rsx) = stage.authored_rsx.as_ref().filter(|_| query.name == "designer_keyframe_draft") {
        rsx.validate_keyframe_draft(&query.arguments)?;
    }
    let mut result = stage.sources.run(|| stage.studio.call(&query.name, &query.arguments, Permissions::default()))?;
    if let Some(owners) = &stage.rsx_spacing {
        owners.check_query(&query.name, &mut result)?;
    }
    if let Some(rsx) = stage.authored_rsx.as_ref().filter(|_| query.name == "designer_programs") {
        rsx.annotate_programs(&mut result);
    }
    if let Some(rsx) = stage.authored_rsx.as_ref().filter(|_| query.name == "designer_states") {
        rsx.annotate_states(&mut result, query.arguments["owner"].as_u64());
    }
    Ok(result)
}

pub(crate) fn edit(renderer: &mut Renderer, request: Edit, projects: &crate::projects::Projects) -> Result<EditResult, String> {
    let query = request.query;
    let stage = renderer.stage(query.frame)?;
    // Source validation and patch translation can outlive the retired frame.
    let modules = stage.rust_modules(&request.document);
    let mut design = DesignDocument::parse_with_rust_modules(
        &request.document,
        projects.source(&request.document)?,
        modules.as_deref(),
    )?;
    let action = query.arguments["action"].as_str();
    let rust_keyframe = design.rsx.is_some() && rsx_animation::keyframe_edit(&query.name, action);
    let rust_program = design.rsx.is_some() && rsx_animation::program_edit(&query.name, action);
    let rust_state = design.rsx.is_some() && rsx_animation::state_edit(&query.name, action);
    let rust_placement = design.rsx.is_some() && rsx_animation::clip_placement_edit(&query.name, action);
    let rust_columns = design.rsx.is_some() && query.name == "designer_columns_edit";
    let rust_style = design.rsx.is_some() && query.name == RSX_STYLE_EDIT;
    let rust_template = design.rsx.is_some() && rust_template_edit(&query.name, action);
    let rust_names = design.rsx.is_some()
        && query.name == "designer_property"
        && query.arguments["property"] == "Name";
    let spacing = query.name == "designer_spacing_edit";
    let wrapping = query.name == "designer_wrap_selection_edit";
    if design.rsx.is_some()
        && !rust_keyframe
        && !rust_program
        && !rust_state
        && !rust_placement
        && !rust_names
        && !rust_columns
        && !rust_style
        && !rust_template
        && !spacing
        && !wrapping
        && (!matches!(query.name.as_str(), "designer_selection_layout_edit" | "designer_property" | "designer_grid_track_edit")
            || !design.editable_structure())
    {
        return Err("This Studio operation requires XAML or a fully mapped RSX layout.".into());
    }
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
    stage.studio.validate(&query.name, &arguments)?;
    if let Some(selection) = &query.selection {
        select(stage, selection)?;
    }
    if spacing {
        return crate::spacing::source_edit(stage, &design, &arguments);
    }
    if rust_columns || rust_template || rust_style {
        let projection = design.rsx.as_ref().ok_or("Open the current RSX invocation")?;
        let (patches, selection) = {
            let session = stage.session.borrow();
            let patches = if rust_style {
                let plan = stage.sources.run(|| ferroui_designer::mcp::rust_style_source_plan(&session, projection, &arguments))?;
                if let Some(uri) = plan.source_uri {
                    let source = stage.authored_rsx.as_ref().and_then(|rust| rust.module_source(&uri))
                        .ok_or("The supplying Rust module is outside the accepted frame")?;
                    return module_edits(uri, source, plan.patches);
                }
                plan.patches
            } else {
                let plan = if rust_columns {
                    ferroui_designer::mcp::rust_column_source_plan
                } else {
                    ferroui_designer::mcp::rust_template_source_plan
                };
                stage.sources.run(|| plan(&session, design.source, projection, &arguments))?
            };
            let selection = crate::edit_selection::rsx_control_ranges(&design, &patches, session.selection())?;
            (patches, selection)
        };
        let mut result = editor_edits(&design, patches)?;
        if !result.edits.is_empty() {
            result.retained_selection = selection;
            renderer.stage.take();
        }
        return Ok(result);
    }
    if rust_names {
        let nodes: Vec<NodeId> = serde_json::from_value(arguments["nodes"].clone())
            .map_err(|error| error.to_string())?;
        let value = match arguments["action"].as_str() {
            Some("reset") => None,
            Some("set") => Some(
                arguments["value"]
                    .as_str()
                    .ok_or("Enter a text control name")?,
            ),
            _ => return Err("Choose set or reset for the control name".into()),
        };
        let edit = rust_name_edit(&design, &nodes, value)?;
        if !edit.edits.is_empty() {
            renderer.stage.take();
        }
        return Ok(edit);
    }
    if wrapping {
        use ferroui_designer::WrapSourceChange;
        let block = design.rsx.as_ref().map(|rsx| design.range(rsx.invocation)).transpose()?;
        let plan = stage.sources.run(|| stage.studio.prepare_wrap_edit(&arguments, design.rsx.take()))?;
        let edited = match &plan.change {
            WrapSourceChange::Xaml(_) => plan.basis.source(),
            WrapSourceChange::Rust(edit) => &edit.authored_source,
        };
        let range = ferroui_designer_language::LineIndex::new(edited).range(plan.wrapper).map_err(|error| error.to_string())?;
        let mut result = match plan.change {
            WrapSourceChange::Xaml(patches) => crate::layer_edits::xaml(&design, &plan.basis, patches)?,
            WrapSourceChange::Rust(edit) => crate::layer_edits::rust_in_block(&design, edit, block.ok_or("Open the current RSX view")?)?,
        };
        result.retained_selection = vec![range];
        return Ok(result);
    }
    if rust_state || rust_keyframe || rust_program || rust_placement {
        let rsx = stage.authored_rsx.as_ref().ok_or("Render the current Rust view before editing motion")?;
        let (patches, pose, program_key, keys, retained_selection) = {
            let session = stage.session.borrow();
            let (patches, pose, program_key, keys) = if rust_state {
                (rsx_animation::state_patches(&design, &arguments)?, None, None, None)
            } else if rust_placement {
                (rsx_animation::clip_placement_patches(&design, &session, &arguments)?, None, None, None)
            } else if rust_program {
                let plan = stage.sources.run(|| rsx_animation::program_patches(&design, &arguments))?;
                (plan.patches, None, plan.selected_key, None)
            } else {
                let plan = stage.sources.run(|| rsx.keyframe_patches(&session, &arguments))?;
                (plan.patches, plan.pose, None, plan.keys)
            };
            let ranges = crate::edit_selection::rsx_control_ranges(&design, &patches, session.selection())?;
            (patches, pose, program_key, keys, ranges)
        };
        let mut result = editor_edits(&design, patches)?;
        if !result.edits.is_empty() {
            result.timeline_pose = pose;
            result.timeline_program_key = program_key;
            result.timeline_keys = keys;
            result.retained_selection = retained_selection;
            renderer.stage.take();
        }
        return Ok(result);
    }
    // Retire the frame before touching the native session. VS Code owns the undo
    // transaction and renders a new frame from its buffer even on rejection.
    let stage = renderer.take_stage().expect("frame checked above");
    let operation = stage.sources.run(|| stage.studio.call(
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
    let mut edit = if design.rsx.is_none() {
        crate::layer_edits::xaml(&design, session.document(), patches)?
    } else {
        editor_edits(&design, patches)?
    };
    if recorded { edit.timeline_pose = session.timeline_preview_pose(); }
    if query.name == "designer_program" {
        edit.timeline_keys = operation.get("keyframes")
            .filter(|keys| !keys.is_null())
            .map(|keys| serde_json::from_value(keys.clone()).map_err(|error| error.to_string()))
            .transpose()?;
    }
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

fn rust_template_edit(tool: &str, action: Option<&str>) -> bool {
    tool == "designer_item_template" && matches!(action, Some("create_inline" | "reset"))
}

fn source_action(tool: &str, action: Option<&str>) -> bool {
    match tool {
        "designer_program" => matches!(
            action,
            Some("create" | "rename" | "duplicate" | "remove" | "selector" | "source" | "update" | "update_timing" | "place_clips" |
                "upsert_keyframe" | "record_keyframe" | "remove_keyframe" | "move_keyframes" | "scale_keyframes" |
                "mirror_keyframes" | "paste_keyframes" | "duplicate_keyframes" | "remove_keyframes")
        ),
        "designer_state" => matches!(action, Some("upsert" | "remove")),
        "designer_behavior" => matches!(action, Some(
            "upsert" | "remove" | "source" | "set_trigger" | "append_action" | "set_action" | "move_action" | "remove_action"
        )),
        "designer_design_data_edit" => {
            matches!(action, Some("set_value" | "set_object" | "sample_items"))
        }
        "designer_item_template" => matches!(action, Some("create_inline" | "copy_inline" | "reset" | "assign_resource" | "assign_included_resource")),
        // All actions for these schemas are source-backed.
        "designer_component"
        | "designer_style_edit"
        | RSX_STYLE_EDIT
        | "designer_palette_edit"
        | "designer_property"
        | "designer_record_property"
        | "designer_record_geometry"
        | "designer_binding_edit"
        | "designer_grid_track_edit"
        | "designer_columns_edit"
        | "designer_spacing_edit"
        | "designer_wrap_selection_edit"
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

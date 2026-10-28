//! Workspace-local preview automation with exact source/board optimistic guards.
use super::schema as s;
use super::{decode, integer, text, Permissions};
use crate::{
    DesignerWorkspace, PreviewEnvironment, PreviewStageConfiguration, PreviewStageManifest,
    PreviewStageMode,
};
use ferroui_inspect::TreeKind;
use serde_json::{json, Value};
use std::rc::Rc;

pub(super) fn handles(name: &str) -> bool {
    matches!(name, "designer_preview_stages" | "designer_preview_stage")
}

fn bounded_string(maximum: usize) -> Value {
    json!({"type":"string","maxLength":maximum})
}

fn environment_schema() -> Value {
    let dimension = json!({"type":"number","minimum":1,"maximum":16384});
    s::object(
        json!({"width":dimension,"height":dimension,"theme":{"enum":["light","dark","inherit"]},
        "flow_direction":{"enum":["inherit","left_to_right","right_to_left"]}}),
        &["width", "height", "theme", "flow_direction"],
    )
}

fn configuration_schema() -> Value {
    let segment = s::object(
        json!({"namespace":bounded_string(4096),"type_name":bounded_string(512),
        "name":s::nullable(bounded_string(512)),"key":s::nullable(bounded_string(512))}),
        &["namespace", "type_name", "name", "key"],
    );
    let anchor = s::object(json!({"path":s::array(segment,128)}), &["path"]);
    let options = s::object(
        json!({"target_type":s::nullable(bounded_string(512)),"sample_content":bounded_string(65536),
        "width":{"type":"number","minimum":1,"maximum":8192},"height":{"type":"number","minimum":1,"maximum":8192}}),
        &["target_type", "sample_content", "width", "height"],
    );
    let target = json!({"oneOf":[s::object(json!({"kind":{"const":"document"}}), &["kind"]),
        s::object(json!({"kind":{"const":"definition"},"anchor":anchor,"options":options}), &["kind", "anchor", "options"])]});
    let state = s::object(
        json!({"owner":anchor,"group":bounded_string(256),"state":bounded_string(256)}),
        &["owner", "group", "state"],
    );
    s::object(
        json!({"name":bounded_string(128),"source_uri":bounded_string(8192),"target":target,
        "environment":environment_schema(),"design_time_data":{"type":"boolean"},"states":s::array(state,32),
        "pinned":{"type":"boolean"},"auto_update":{"type":"boolean"},"mode":{"enum":["select","live"]}}),
        &[
            "name",
            "source_uri",
            "target",
            "environment",
            "design_time_data",
            "states",
            "pinned",
            "auto_update",
            "mode",
        ],
    )
}

fn manifest_schema() -> Value {
    s::object(
        json!({"version":{"const":1},"stages":s::array(configuration_schema(),crate::MAX_PREVIEW_STAGES),
        "active":s::nullable(json!({"type":"integer","minimum":0,"maximum":15}))}),
        &["version", "stages", "active"],
    )
}

fn action(name: &str, mut properties: Value, required: &[&str]) -> Value {
    properties["expected_model"] = bounded_string(16);
    let mut required = required.to_vec();
    required.push("expected_model");
    s::action(name, properties, &required, false)
}

pub(super) fn tools() -> Vec<Value> {
    let mut edit = s::actions(
        "designer_preview_stage",
        "Configure workspace-local native preview stages using the model from designer_preview_stages. Changes never edit authored XAML or undo history. Sources must already be open; missing references retain a diagnostic. Live previews require allow_input as well as allow_property_writes. Focus/select activate the correct document and definition before changing the inspector. Restore accepts a versioned manifest; no filesystem paths are opened.",
        vec![
            action(
                "add",
                json!({"configuration":configuration_schema()}),
                &["configuration"],
            ),
            action(
                "update",
                json!({"stage":s::id(),"configuration":configuration_schema()}),
                &["stage", "configuration"],
            ),
            action("duplicate", json!({"stage":s::id()}), &["stage"]),
            action("remove", json!({"stage":s::id()}), &["stage"]),
            action(
                "move",
                json!({"stage":s::id(),"index":{"type":"integer","minimum":0,"maximum":15}}),
                &["stage", "index"],
            ),
            action(
                "pause",
                json!({"stage":s::id(),"paused":{"type":"boolean"}}),
                &["stage", "paused"],
            ),
            action("refresh", json!({"stage":s::id()}), &["stage"]),
            action("refresh_all", json!({}), &[]),
            action("focus", json!({"stage":s::id()}), &["stage"]),
            action(
                "select",
                json!({"stage":s::id(),"inspector_id":s::id()}),
                &["stage", "inspector_id"],
            ),
            action("show", json!({"visible":{"type":"boolean"}}), &["visible"]),
            action(
                "restore",
                json!({"manifest":manifest_schema()}),
                &["manifest"],
            ),
            action(
                "compare",
                json!({"stage":s::id(),"variants":s::array(s::object(json!({"name":bounded_string(128),"environment":environment_schema()}), &["name", "environment"]),crate::MAX_PREVIEW_STAGES)}),
                &["stage", "variants"],
            ),
        ],
        false,
    );
    edit["annotations"]["readOnlyHint"] = json!(false);
    edit["annotations"]["destructiveHint"] = json!(true);
    edit["inputSchema"]["required"] = json!(["action", "expected_model"]);
    vec![
        s::tool(
            "designer_preview_stages",
            "Read independent preview configurations, accepted/pending/failed status, manifest and optimistic model without rebuilding controls or applying source. Optional stage/tree includes its native runtime rows. Source IDs are absent when a retained preview is stale. Pin follows its open source across tabs; pause stops automatic rebuilds.",
            s::object(
                json!({"stage":s::id(),"tree":{"enum":["visual","logical"]}}),
                &[],
            ),
            false,
        ),
        edit,
    ]
}

fn report(workspace: &DesignerWorkspace, args: &Value) -> Result<Value, String> {
    let stages = workspace
        .preview_stages()
        .into_iter()
        .map(|stage| {
            let current = workspace.preview_stage_source_is_current(stage.id);
            let mut value = json!(stage);
            value["current_source"] = json!(current);
            value
        })
        .collect::<Vec<_>>();
    let mut value = json!({"model":workspace.preview_stage_model(),"epoch":workspace.preview_stage_epoch(),
        "visible":workspace.preview_stages_visible(),"stages":stages,"manifest":workspace.preview_stage_manifest(),
        "active_source_uri":workspace.session().borrow().document_uri()});
    if let Some(stage) = args["stage"].as_u64() {
        let kind = if args["tree"] == "logical" {
            TreeKind::Logical
        } else {
            TreeKind::Visual
        };
        let rows = workspace.preview_stage_runtime_tree(stage, kind)?;
        value["tree_truncated"] = json!(rows.len() > 10000);
        value["tree"] = json!(rows.iter().take(10000).map(|row| json!({"inspector_id":row.inspector_id,
            "source_id":row.source_id,"parent":row.parent,"depth":row.depth,"type_name":row.type_name,"name":row.name,
            "bounds":row.bounds.map(|bounds| json!({"x":bounds.x,"y":bounds.y,"width":bounds.width,"height":bounds.height}))})).collect::<Vec<_>>());
    } else if args.get("tree").is_some() {
        return Err("Select a stage to read its native tree".into());
    }
    Ok(value)
}

fn needs_input(workspace: &DesignerWorkspace, action: &str, args: &Value) -> Result<bool, String> {
    if action == "show"
        && args["visible"] == true
        && workspace.preview_stages().is_empty()
        && workspace.design_surface().interactive()
    {
        return Ok(true);
    }
    if let Some(value) = args.get("configuration") {
        let config: PreviewStageConfiguration =
            serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
        config.validate()?;
        if config.mode == PreviewStageMode::Live {
            return Ok(true);
        }
    }
    if action == "restore" {
        let manifest: PreviewStageManifest = decode(args, "manifest")?;
        manifest.validate()?;
        if manifest
            .stages
            .iter()
            .any(|config| config.mode == PreviewStageMode::Live)
        {
            return Ok(true);
        }
    }
    // Synchronizing an open source can rebuild another automatically updating Live
    // stage. Guard the entire native operation, not only the requested stage's mode.
    Ok(workspace.preview_stages().iter().any(|stage| {
        stage.configuration.mode == PreviewStageMode::Live
            && !(action == "remove" && args["stage"].as_u64() == Some(stage.id))
            && (stage.configuration.auto_update
                || args["stage"].as_u64() == Some(stage.id)
                || action == "refresh_all")
    }))
}

pub(super) fn call(
    workspace: &Rc<DesignerWorkspace>,
    name: &str,
    args: &Value,
    permissions: Permissions,
) -> Result<Value, String> {
    if name == "designer_preview_stages" {
        return report(workspace, args);
    }
    if text(args, "expected_model")? != workspace.preview_stage_model() {
        return Err(
            "The documents or stage board changed; read designer_preview_stages again".into(),
        );
    }
    let action = text(args, "action")?;
    if needs_input(workspace, &action, args)? && !permissions.allow_input {
        return Err("Creating, resuming or focusing a Live preview requires allow_input".into());
    }
    let mut result = json!({});
    match action.as_str() {
        "add" => {
            result["created_stage"] =
                json!(workspace.add_preview_stage(decode(args, "configuration")?)?)
        }
        "update" => workspace
            .update_preview_stage(integer(args, "stage")?, decode(args, "configuration")?)?,
        "duplicate" => {
            result["created_stage"] =
                json!(workspace.duplicate_preview_stage(integer(args, "stage")?)?)
        }
        "remove" => workspace.remove_preview_stage(integer(args, "stage")?)?,
        "move" => workspace
            .move_preview_stage(integer(args, "stage")?, integer(args, "index")? as usize)?,
        "pause" => workspace
            .set_preview_stage_paused(integer(args, "stage")?, args["paused"].as_bool().unwrap())?,
        "refresh" => workspace.refresh_preview_stage(integer(args, "stage")?)?,
        "refresh_all" => {
            result["refresh_errors"] = json!(workspace
                .refresh_all_preview_stages()
                .iter()
                .map(|(stage, message)| json!({"stage":stage,"message":message}))
                .collect::<Vec<_>>())
        }
        "focus" => workspace.set_active_preview_stage(integer(args, "stage")?)?,
        "select" => workspace.select_preview_stage_runtime(
            integer(args, "stage")?,
            integer(args, "inspector_id")?,
        )?,
        "show" => workspace.set_preview_stages_visible(args["visible"].as_bool().unwrap())?,
        "restore" => workspace.restore_preview_stage_manifest(decode(args, "manifest")?)?,
        "compare" => {
            let variants = args["variants"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| {
                    let environment: PreviewEnvironment = decode(value, "environment")?;
                    Ok((text(value, "name")?, environment))
                })
                .collect::<Result<Vec<_>, String>>()?;
            result["created_stages"] =
                json!(workspace.compare_preview_stage(integer(args, "stage")?, variants)?);
        }
        _ => return Err("Unknown preview-stage action".into()),
    }
    result["board"] = report(workspace, &json!({}))?;
    Ok(result)
}

#[cfg(test)]
#[path = "mcp_preview_stage_permission_tests.rs"]
mod permission_tests;
#[cfg(test)]
#[path = "mcp_preview_stages_tests.rs"]
mod tests;

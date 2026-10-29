//! Observational graph/playback reports and explicitly guarded prototype commands.
use super::{decode, integer, schema as s, text, Permissions};
use crate::DesignerWorkspace;
use serde_json::{json, Value};
use std::rc::Rc;

pub(super) fn handles(name: &str) -> bool {
    matches!(name, "designer_prototype" | "designer_prototype_action")
}

pub(super) fn graph_schema(indices: bool) -> Value {
    let stage = if indices {
        json!({"type":"integer","minimum":0,"maximum":15})
    } else {
        s::id()
    };
    let label = json!({"type":"string","maxLength":128});
    let flow = s::object(json!({"name":label,"start":stage}), &["name", "start"]);
    let action = json!({"oneOf":[
        s::object(json!({"kind":{"const":"navigate"},"stage":stage}), &["kind","stage"]),
        s::object(json!({"kind":{"const":"back"}}), &["kind"])
    ]});
    let link = s::object(
        json!({"name":label,"source":stage,"trigger":super::preview_stages::anchor_schema(),"action":action}),
        &["name", "source", "trigger", "action"],
    );
    s::object(
        json!({"flows":s::array(flow,crate::MAX_PREVIEW_PROTOTYPE_FLOWS),"links":s::array(link,crate::MAX_PREVIEW_PROTOTYPE_LINKS)}),
        &[],
    )
}

fn action(name: &str, mut properties: Value, required: &[&str]) -> Value {
    properties["expected_model"] = json!({"type":"string","maxLength":16});
    let mut required = required.to_vec();
    required.push("expected_model");
    s::action(name, properties, &required, false)
}

pub(super) fn tools() -> Vec<Value> {
    let mut edit = s::actions(
        "designer_prototype_action",
        "Edit a local prototype graph or navigate its isolated native player using the model from designer_prototype. set_graph atomically accepts named starts and source-anchored Navigate/Back interactions; it never runs factories or edits XAML. Play creates isolated Live views for reachable stages. Back and restart retain their native input; stop then play creates fresh views. Playing or navigating requires allow_input as well as allow_property_writes; stop requires only allow_property_writes. Source or stage changes invalidate a running journey. No arbitrary scripts, URLs or paths are executed.",
        vec![
            action("set_graph", json!({"graph":graph_schema(false)}), &["graph"]),
            action("play", json!({"flow":{"type":"string","maxLength":128}}), &["flow"]),
            action("activate", json!({"link":{"type":"integer","minimum":0,"maximum":127}}), &["link"]),
            action("back", json!({}), &[]),
            action("restart", json!({}), &[]),
            action("stop", json!({}), &[]),
        ], false,
    );
    edit["annotations"]["readOnlyHint"] = json!(false);
    edit["annotations"]["destructiveHint"] = json!(true);
    edit["inputSchema"]["required"] = json!(["action", "expected_model"]);
    vec![
        s::tool("designer_prototype",
            "Read the named flows, source-anchored interactions, reachable screens, original stage IDs and current player history without creating native views, synchronizing source, changing layout or advancing playback. Optional stage includes eligible authored hotspot choices from its existing native map. current_source is false when retained playback no longer matches the workspace. Persist through the preview-stage manifest, which uses remapped stage indices.",
            s::object(json!({"stage":s::id()}), &[]), false),
        edit,
    ]
}

fn report(workspace: &DesignerWorkspace, args: &Value) -> Result<Value, String> {
    let graph = workspace.preview_prototype();
    let flows = graph.flows.iter().map(|flow| json!({"name":flow.name,"start":flow.start,"reachable":graph.reachable(flow.start)})).collect::<Vec<_>>();
    let stages = workspace.preview_stages().into_iter().map(|stage| json!({"id":stage.id,"name":stage.configuration.name,"source_uri":stage.configuration.source_uri,"status":stage.status})).collect::<Vec<_>>();
    let mut value = json!({"model":workspace.preview_prototype_model(),"graph":graph,"flows":flows,"stages":stages,"playback":workspace.preview_prototype_playback(),"visible":workspace.preview_prototype_visible()});
    if let Some(stage) = args["stage"].as_u64() {
        workspace
            .preview_stage(stage)
            .ok_or("The prototype source stage was removed")?;
        value["elements"] = json!(workspace
            .preview_prototype_elements()
            .into_iter()
            .filter(|choice| choice.stage == stage)
            .map(|choice| json!({"stage":choice.stage,"label":choice.label,"anchor":choice.anchor}))
            .collect::<Vec<_>>());
        value["current_source"] = json!(workspace.preview_stage_source_is_current(stage));
    }
    Ok(value)
}

pub(super) fn call(
    workspace: &Rc<DesignerWorkspace>,
    name: &str,
    args: &Value,
    permissions: Permissions,
) -> Result<Value, String> {
    if name == "designer_prototype" {
        return report(workspace, args);
    }
    if text(args, "expected_model")? != workspace.preview_prototype_model() {
        return Err(
            "The prototype, player or source changed; read designer_prototype again".into(),
        );
    }
    let action = text(args, "action")?;
    if matches!(action.as_str(), "play" | "activate" | "back" | "restart")
        && !permissions.allow_input
    {
        return Err("Playing or navigating native prototypes requires allow_input".into());
    }
    match action.as_str() {
        "set_graph" => workspace.set_preview_prototype(decode(args, "graph")?)?,
        "play" => workspace.start_preview_prototype(&text(args, "flow")?)?,
        "activate" => workspace.activate_preview_prototype_link(integer(args, "link")? as usize)?,
        "back" => workspace.back_preview_prototype()?,
        "restart" => workspace.restart_preview_prototype()?,
        "stop" => workspace.stop_preview_prototype(),
        _ => return Err("Unknown prototype action".into()),
    }
    report(workspace, &json!({}))
}

#[cfg(test)]
#[path = "mcp_prototype_tests.rs"]
mod tests;

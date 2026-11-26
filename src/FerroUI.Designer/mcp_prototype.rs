//! Observational graph/playback reports and explicitly guarded prototype commands.
use super::{decode, integer, schema as s, text, Permissions};
use crate::DesignerWorkspace;
use base64::Engine;
use serde_json::{json, Value};
use std::rc::Rc;

pub(super) fn handles(name: &str) -> bool {
    matches!(name, "designer_prototype" | "designer_prototype_action" | "designer_prototype_input" | "designer_prototype_image")
}

pub(super) fn graph_schema(indices: bool) -> Value {
    let stage = if indices {
        json!({"type":"integer","minimum":0,"maximum":15})
    } else {
        s::id()
    };
    let label = json!({"type":"string","maxLength":128});
    let flow = s::object(json!({"name":label,"start":stage}), &["name", "start"]);
    let preset = s::object(
        json!({"kind":{"enum":["center","top_left","top_center","top_right","center_left","center_right","bottom_left","bottom_center","bottom_right"]}}),
        &["kind"],
    );
    let offset = json!({"type":"integer","minimum":-crate::MAX_PREVIEW_PROTOTYPE_OVERLAY_OFFSET,"maximum":crate::MAX_PREVIEW_PROTOTYPE_OVERLAY_OFFSET});
    let manual = s::object(
        json!({"kind":{"const":"manual"},"x":offset,"y":offset}),
        &["kind", "x", "y"],
    );
    let options = s::object(
        json!({
            "placement":{"oneOf":[preset,manual]},
            "dismiss_on_outside":{"type":"boolean"},"dim_background":{"type":"boolean"}
        }),
        &[],
    );
    let action = json!({"oneOf":[
        s::object(json!({"kind":{"const":"navigate"},"stage":stage}), &["kind","stage"]),
        s::object(json!({"kind":{"const":"back"}}), &["kind"]),
        s::object(json!({"kind":{"const":"open_overlay"},"stage":stage,"options":options}), &["kind","stage"]),
        s::object(json!({"kind":{"const":"swap_overlay"},"stage":stage,"options":options}), &["kind","stage"]),
        s::object(json!({"kind":{"const":"close_overlay"}}), &["kind"])
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

pub(super) fn transitions_schema() -> Value {
    let transition = s::object(json!({
        "effect":{"enum":["dissolve","slide_left","slide_right","slide_up","slide_down"]},
        "duration_ms":{"type":"integer","minimum":1,"maximum":crate::MAX_PROTOTYPE_TRANSITION_MS},
        "easing":{"enum":["linear","ease_in","ease_out","ease_in_out"]}
    }), &["effect", "duration_ms"]);
    s::array(s::object(json!({
        "link":{"type":"integer","minimum":0,"maximum":crate::MAX_PREVIEW_PROTOTYPE_LINKS - 1},
        "transition":transition
    }), &["link", "transition"]), crate::MAX_PREVIEW_PROTOTYPE_LINKS)
}

pub(super) fn tools() -> Vec<Value> {
    let mut edit = s::actions(
        "designer_prototype_action",
        "Edit a local prototype graph or navigate its isolated native player using the model from designer_prototype. set_graph atomically accepts source-anchored interactions and optional transitions indexed by link order. Omitting transitions preserves existing motion for the same source anchors. Use an empty list for instant playback on every link. Play creates isolated views. Back closes the top overlay before consuming screen history and can reverse its opening motion. reduced_motion skips animations for this play session. Restart retains native state. Stop then play creates fresh views. Playing and navigating require allow_input and allow_property_writes.",
        vec![
            action("set_graph", json!({"graph":graph_schema(false),"transitions":transitions_schema()}), &["graph"]),
            action("play", json!({"flow":{"type":"string","maxLength":128},"reduced_motion":{"type":"boolean"}}), &["flow"]),
            action("activate", json!({"link":{"type":"integer","minimum":0,"maximum":127}}), &["link"]),
            action("back", json!({}), &[]),
            action("close_overlay", json!({}), &[]),
            action("restart", json!({}), &[]),
            action("stop", json!({}), &[]),
            action("set_reduced_motion", json!({"reduced_motion":{"type":"boolean"}}), &["reduced_motion"]),
        ], false,
    );
    edit["annotations"]["readOnlyHint"] = json!(false);
    edit["annotations"]["destructiveHint"] = json!(true);
    edit["inputSchema"]["required"] = json!(["action", "expected_model"]);
    let model = json!({"type":"string","maxLength":16});
    let input = json!({"oneOf":[
        s::object(json!({"kind":{"const":"keyboard"},"input":super::preview_stages::keyboard_schema()}), &["kind","input"]),
        s::object(json!({"kind":{"const":"pointer"},"input":super::preview_stages::pointer_schema()}), &["kind","input"])
    ]});
    vec![
        s::tool("designer_prototype",
            "Read the named flows, source-anchored interactions, reachable screens, original stage IDs and current player history without creating native views, synchronizing source, changing layout or advancing playback. Optional stage includes eligible authored hotspot choices from its existing native map. current_source is false when retained playback no longer matches the workspace. Persist through the preview-stage manifest, which uses remapped stage indices.",
            s::object(json!({"stage":s::id()}), &[]), false),
        edit,
        s::tool("designer_prototype_image", "Capture the accepted native prototype artboard including its modal stack. Use expected_model and the playback epoch from designer_prototype.",
            s::object(json!({"expected_model":model,"expected_frame":s::revision()}), &["expected_model","expected_frame"]), false),
        s::tool("designer_prototype_input", "Send a key, text, or pointer contact to the accepted native prototype. Only the active layer receives input. Use expected_model and the playback epoch from designer_prototype. Requires input and write permission.",
            s::object(json!({"expected_model":model,"expected_frame":s::revision(),"input":input}), &["expected_model","expected_frame","input"]), true),
    ]
}

fn report(workspace: &DesignerWorkspace, args: &Value) -> Result<Value, String> {
    let graph = workspace.preview_prototype();
    let flows = graph.flows.iter().map(|flow| json!({"name":flow.name,"start":flow.start,"reachable":graph.reachable(flow.start)})).collect::<Vec<_>>();
    let stages = workspace.preview_stages().into_iter().map(|stage| json!({"id":stage.id,"name":stage.configuration.name,"source_uri":stage.configuration.source_uri,"status":stage.status})).collect::<Vec<_>>();
    let mut value = json!({"model":workspace.preview_prototype_model(),"graph":graph,
        "transitions":workspace.preview_prototype_transitions(),"flows":flows,"stages":stages,
        "playback":workspace.preview_prototype_playback(),"visible":workspace.preview_prototype_visible()});
    if let Some(playback) = value["playback"].as_object_mut() {
        playback.insert("animating".to_owned(), json!(workspace.preview_prototype_motion_running()));
    }
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
    if name == "designer_prototype_image" {
        let image = workspace.preview_prototype_image(integer(args, "expected_frame")?)?;
        return Ok(json!({"mimeType":"image/png", "data":base64::engine::general_purpose::STANDARD.encode(image.png),
            "width":image.pixel_width,"height":image.pixel_height}));
    }
    if name == "designer_prototype_input" {
        if !permissions.allow_input { return Err("Native prototype input requires allow_input".into()); }
        let focus = workspace.preview_prototype_input(integer(args, "expected_frame")?, decode(args, "input")?)?;
        return Ok(json!({"focus":focus,"playback":workspace.preview_prototype_playback(),"model":workspace.preview_prototype_model()}));
    }
    let action = text(args, "action")?;
    if matches!(
        action.as_str(),
        "play" | "activate" | "back" | "close_overlay" | "restart"
    ) && !permissions.allow_input
    {
        return Err("Playing or navigating native prototypes requires allow_input".into());
    }
    match action.as_str() {
        "set_graph" => {
            let graph = decode(args, "graph")?;
            if args.get("transitions").is_some() {
                workspace.set_preview_prototype_with_transitions(graph, decode(args, "transitions")?)?;
            } else {
                workspace.set_preview_prototype(graph)?;
            }
        }
        "play" => workspace.start_preview_prototype_with_motion_preference(
            &text(args, "flow")?, args["reduced_motion"].as_bool().unwrap_or(false),
        )?,
        "activate" => workspace.activate_preview_prototype_link(integer(args, "link")? as usize)?,
        "back" => workspace.back_preview_prototype()?,
        "close_overlay" => workspace.close_preview_prototype_overlay()?,
        "restart" => workspace.restart_preview_prototype()?,
        "stop" => workspace.stop_preview_prototype(),
        "set_reduced_motion" => workspace.set_preview_prototype_reduced_motion(decode(args, "reduced_motion")?)?,
        _ => return Err("Unknown prototype action".into()),
    }
    report(workspace, &json!({}))
}

#[cfg(test)]
#[path = "mcp_prototype_tests.rs"]
mod tests;

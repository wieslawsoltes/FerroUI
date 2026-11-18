//! Toolbox and drop operations use the same measured authoring plans as editor hosts.
use super::{decode, integer, node_id, number, schema as s};
use crate::{
    DesignerAsset, DesignerSession, DesignerWorkspace, OutlinePlacement, PlacementOrigin,
    PlacementTarget,
};
use ferroui_base::Point;
use ferroui_designer_tools::Point as LayoutPoint;
use serde_json::{json, Value};

pub(super) fn handles(name: &str) -> bool {
    matches!(
        name,
        "designer_toolbox" | "designer_place" | "designer_record_geometry"
    )
}

pub(super) fn tools() -> Vec<Value> {
    vec![
        s::tool("designer_toolbox", "List control assets available for insertion in this designer.", s::object(json!({}), &[]), false),
        s::tool("designer_place", "Insert a registered Toolbox asset or move a source control. Choose an outline anchor with inside/before/after, or x/y in artboard coordinates.",
            s::object(json!({"expected_revision":s::revision(),"asset":s::string(),"node":s::id(),
                "anchor":s::id(),"position":{"enum":["inside","before","after"]},"x":s::number(),"y":s::number()}),
                &["expected_revision"]), true),
        s::actions("designer_record_geometry", "Record a measured move or resize as animation keys at the scrubbed playhead. Canvas coordinates and deltas use logical artboard units. Resize handles run clockwise from zero at the top left.", vec![
            s::action("move", json!({"program":s::id(),"time":s::time(),"nodes":s::array(s::id(),128),"anchor":s::id(),
                "grab":s::object(json!({"x":s::number(),"y":s::number()}), &["x","y"]),"x":s::number(),"y":s::number()}),
                &["program","time","nodes","anchor","x","y"], true),
            s::action("resize", json!({"program":s::id(),"time":s::time(),"node":s::id(),
                "handle":{"type":"integer","minimum":0,"maximum":7},"dx":s::number(),"dy":s::number()}),
                &["program","time","node","handle","dx","dy"], true),
        ], true),
    ]
}

pub(super) fn call(
    session: &mut DesignerSession,
    workspace: Option<&DesignerWorkspace>,
    name: &str,
    args: &Value,
) -> Result<Value, String> {
    let assets = assets(workspace);
    if name == "designer_toolbox" {
        return Ok(json!({ "assets": assets.iter().map(|asset| json!({
            "id": asset.id, "title": asset.title, "category": asset.category,
            "description": asset.description, "icon": asset.icon.path().trim_start_matches("F1 "),
        })).collect::<Vec<_>>() }));
    }
    let revision = super::integer(args, "expected_revision")?;
    if session.document().revision() != revision {
        return Err("Read the current designer revision before placing a control".into());
    }
    if name == "designer_record_geometry" {
        return record_geometry(session, args);
    }
    let origin = match (args["asset"].as_str(), args["node"].as_u64()) {
        (Some(asset), None) => PlacementOrigin::Asset(
            &assets
                .iter()
                .find(|item| item.id == asset)
                .ok_or("Unknown toolbox control")?
                .source,
        ),
        (None, Some(node)) => PlacementOrigin::Node { node, grab: None },
        _ => return Err("Specify one asset ID or one source node".into()),
    };
    let target = match (
        args["anchor"].as_u64(),
        args["position"].as_str(),
        args["x"].as_f64(),
        args["y"].as_f64(),
    ) {
        (Some(node), Some(position), None, None) => PlacementTarget::Outline {
            node,
            position: match position {
                "inside" => OutlinePlacement::Inside,
                "before" => OutlinePlacement::Before,
                "after" => OutlinePlacement::After,
                _ => return Err("Unknown outline placement".into()),
            },
        },
        (None, None, Some(x), Some(y)) => PlacementTarget::Canvas(Point::new(x, y)),
        _ => return Err("Specify an outline anchor and position or a canvas point".into()),
    };
    let plan = session.plan_placement(origin, target)?;
    session.apply_visual(&plan.description, plan.edits)?;
    Ok(super::state(session))
}

fn record_geometry(session: &mut DesignerSession, args: &Value) -> Result<Value, String> {
    let program = node_id(args, "program")?;
    let time = number(args, "time")?;
    let (edits, label) = match args["action"].as_str() {
        Some("move") => {
            let nodes: Vec<_> = decode(args, "nodes")?;
            let grab = args.get("grab").map(|_| decode(args, "grab")).transpose()?;
            let destination = LayoutPoint::new(number(args, "x")?, number(args, "y")?);
            let plan =
                session.plan_recorded_move(&nodes, node_id(args, "anchor")?, grab, destination)?;
            (plan.edits, "Record control move")
        }
        Some("resize") => {
            let handle =
                u8::try_from(integer(args, "handle")?).map_err(|_| "Unknown resize handle")?;
            let edits = session.plan_resize_handle(
                node_id(args, "node")?,
                handle,
                number(args, "dx")?,
                number(args, "dy")?,
            )?;
            (
                session.animation_geometry_edits(edits)?,
                "Record control resize",
            )
        }
        _ => return Err("Choose a move or resize to record".into()),
    };
    session.record_geometry(program, time, label, edits)?;
    Ok(json!({"document": super::state(session),"timelinePose":session.timeline_preview_pose()}))
}

fn assets(workspace: Option<&DesignerWorkspace>) -> Vec<DesignerAsset> {
    workspace
        .and_then(DesignerWorkspace::asset_toolbox)
        .map(|toolbox| toolbox.assets())
        .unwrap_or_else(crate::default_designer_assets)
}

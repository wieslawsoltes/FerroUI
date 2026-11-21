//! Toolbox and drop operations use the same measured authoring plans as editor hosts.
use super::{decode, integer, node_id, number, schema as s};
use crate::{
    DesignerAsset, DesignerSession, DesignerWorkspace, OutlinePlacement, PlacementOrigin,
    PlacementTarget,
};
use ferroui_base::Point;
use ferroui_designer_tools::Point as LayoutPoint;
use serde_json::{json, Value};

#[path = "mcp_placement_snap.rs"]
mod snap;

pub(super) fn handles(name: &str) -> bool {
    matches!(
        name,
        "designer_toolbox" | "designer_place" | "designer_placement_hint" | "designer_resize" | "designer_resize_hint" | "designer_record_geometry"
    )
}

pub(super) fn tools() -> Vec<Value> {
    let placement = s::object(json!({"expected_revision":s::revision(),"asset":s::string(),"node":s::id(),
        "selection":s::array(s::id(),128),"moving_anchor":s::id(),
        "anchor":s::id(),"position":{"enum":["inside","before","after"]},"x":s::number(),"y":s::number(),"snap":snap::schema()}),
        &["expected_revision"]);
    let resize = s::object(json!({"expected_revision":s::revision(),"node":s::id(),"handle":{"type":"integer","minimum":0,"maximum":7},
        "dx":s::number(),"dy":s::number(),"snap":snap::schema()}), &["expected_revision", "node", "handle", "dx", "dy"]);
    vec![
        s::tool("designer_toolbox", "List control assets available for insertion in this designer.", s::object(json!({}), &[]), false),
        s::tool("designer_place", "Insert a Toolbox asset or move an authored selection. Use an outline anchor or canvas coordinates with optional snapping.", placement.clone(), true),
        s::tool("designer_placement_hint", "Read the native landing bounds and snap lines for a proposed placement.", placement, false),
        s::tool("designer_resize", "Commit a native resize at an optional snap adjustment. Handles run clockwise from the top left.", resize.clone(), true),
        s::tool("designer_resize_hint", "Read the effective native resize displacement and snap lines.", resize, false),
        s::actions("designer_record_geometry", "Record a measured move or resize as animation keys at the scrubbed playhead. Canvas coordinates and deltas use logical artboard units. Resize handles run clockwise from zero at the top left.", vec![
            s::action("move", json!({"program":s::id(),"time":s::time(),"nodes":s::array(s::id(),128),"anchor":s::id(),
                "grab":s::object(json!({"x":s::number(),"y":s::number()}), &["x","y"]),"x":s::number(),"y":s::number(),"snap":snap::schema()}),
                &["program","time","nodes","anchor","x","y"], true),
            s::action("resize", json!({"program":s::id(),"time":s::time(),"node":s::id(),
                "handle":{"type":"integer","minimum":0,"maximum":7},"dx":s::number(),"dy":s::number(),"snap":snap::schema()}),
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
        return record_geometry(session, workspace, args);
    }
    if matches!(name, "designer_resize" | "designer_resize_hint") {
        return resize(session, workspace, name, args);
    }
    let nodes: Option<Vec<u64>> = args.get("selection").map(|_| decode(args, "selection")).transpose()?;
    let origin = match (args["asset"].as_str(), args["node"].as_u64(), nodes.as_deref()) {
        (Some(asset), None, None) => PlacementOrigin::Asset(
            &assets
                .iter()
                .find(|item| item.id == asset)
                .ok_or("Unknown toolbox control")?
                .source,
        ),
        (None, Some(node), None) => PlacementOrigin::Node { node, grab: None },
        (None, None, Some(nodes)) if (2..=128).contains(&nodes.len()) => {
            let anchor = args["moving_anchor"].as_u64().unwrap_or(nodes[0]);
            if !nodes.contains(&anchor) || nodes.iter().collect::<std::collections::BTreeSet<_>>().len() != nodes.len() {
                return Err("Choose a distinct selection and an anchor within it".into());
            }
            PlacementOrigin::Nodes { nodes, anchor, grab: None }
        }
        _ => return Err("Specify one asset, one source node, or an ordered selection".into()),
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
    let (target, adjustment) = match target {
        PlacementTarget::Canvas(point) => {
            let excluded = match &origin {
                PlacementOrigin::Asset(_) => &[][..],
                PlacementOrigin::Node { node, .. } => std::slice::from_ref(node),
                PlacementOrigin::Nodes { nodes, .. } => *nodes,
            };
            let adjustment = snap::resolve(workspace, session, args, excluded,
                |query| session.snap_canvas_placement(&origin, point, query, false))?;
            let point = Point::new(point.x + adjustment.delta.x, point.y + adjustment.delta.y);
            (PlacementTarget::Canvas(point), adjustment)
        }
        _ if args.get("snap").is_some() => return Err("Snapping applies to canvas placement".into()),
        _ => (target, Default::default()),
    };
    let after = match target {
        PlacementTarget::Outline { node, position: OutlinePlacement::After } => Some(node),
        _ => None,
    };
    let moving = match &origin {
        PlacementOrigin::Asset(_) => Vec::new(),
        PlacementOrigin::Node { node, .. } => vec![*node],
        PlacementOrigin::Nodes { nodes, .. } => nodes.to_vec(),
    };
    let plan = session.plan_placement(origin, target)?;
    if name == "designer_placement_hint" {
        return Ok(json!({"bounds":adjustment.bounds.or(plan.bounds),"description":plan.description,"snapLines":adjustment.guides}));
    }
    if session.uses_rust_source() && ferroui_designer_rsx::is_child_list_edit(&plan.edits) {
        session.apply_rust_placement(revision, &plan.description, plan.edits, after, &moving)?;
    } else {
        session.apply_visual(&plan.description, plan.edits)?;
    }
    Ok(super::state(session))
}

fn resize(session: &mut DesignerSession, workspace: Option<&DesignerWorkspace>, name: &str, args: &Value) -> Result<Value, String> {
    let node = node_id(args, "node")?;
    let handle = u8::try_from(integer(args, "handle")?).map_err(|_| "Unknown resize handle")?;
    let dx = number(args, "dx")?;
    let dy = number(args, "dy")?;
    let adjustment = snap::resolve(workspace, session, args, &[node],
        |query| session.snap_canvas_resize(node, handle, dx, dy, query))?;
    let dx = dx + adjustment.delta.x;
    let dy = dy + adjustment.delta.y;
    let edits = session.plan_resize_handle(node, handle, dx, dy)?;
    if name == "designer_resize_hint" {
        return Ok(json!({"node":node,"handle":handle,"dx":dx,"dy":dy,"bounds":adjustment.bounds,"snapLines":adjustment.guides}));
    }
    session.apply_visual("Resize control", edits)?;
    Ok(super::state(session))
}

fn record_geometry(session: &mut DesignerSession, workspace: Option<&DesignerWorkspace>, args: &Value) -> Result<Value, String> {
    let program = node_id(args, "program")?;
    let time = number(args, "time")?;
    let (edits, label) = match args["action"].as_str() {
        Some("move") => {
            let nodes: Vec<_> = decode(args, "nodes")?;
            let grab: Option<LayoutPoint> = args.get("grab").map(|_| decode(args, "grab")).transpose()?;
            let anchor = node_id(args, "anchor")?;
            let destination = LayoutPoint::new(number(args, "x")?, number(args, "y")?);
            let origin = PlacementOrigin::Nodes { nodes: &nodes, anchor, grab: grab.map(|point| Point::new(point.x, point.y)) };
            let adjustment = snap::resolve(workspace, session, args, &nodes, |query| {
                session.snap_canvas_placement(&origin, Point::new(destination.x, destination.y), query, true)
            })?;
            let destination = LayoutPoint::new(destination.x + adjustment.delta.x, destination.y + adjustment.delta.y);
            let plan = session.plan_recorded_move(&nodes, anchor, grab, destination)?;
            (plan.edits, "Record control move")
        }
        Some("resize") => {
            let handle =
                u8::try_from(integer(args, "handle")?).map_err(|_| "Unknown resize handle")?;
            let node = node_id(args, "node")?;
            let dx = number(args, "dx")?;
            let dy = number(args, "dy")?;
            let adjustment = snap::resolve(workspace, session, args, &[node], |query| session.snap_canvas_resize(node, handle, dx, dy, query))?;
            let edits = session.plan_resize_handle(node, handle, dx + adjustment.delta.x, dy + adjustment.delta.y)?;
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

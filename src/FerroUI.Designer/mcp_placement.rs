//! Toolbox and drop operations use the same measured authoring plans as editor hosts.
use super::schema as s;
use crate::{
    DesignerAsset, DesignerSession, DesignerWorkspace, OutlinePlacement, PlacementOrigin,
    PlacementTarget,
};
use ferroui_base::Point;
use serde_json::{json, Value};

pub(super) fn handles(name: &str) -> bool {
    matches!(name, "designer_toolbox" | "designer_place")
}

pub(super) fn tools() -> Vec<Value> {
    vec![
        s::tool("designer_toolbox", "List control assets available for insertion in this designer.", s::object(json!({}), &[]), false),
        s::tool("designer_place", "Insert a registered Toolbox asset or move a source control. Choose an outline anchor with inside/before/after, or x/y in artboard coordinates.",
            s::object(json!({"expected_revision":s::revision(),"asset":s::string(),"node":s::id(),
                "anchor":s::id(),"position":{"enum":["inside","before","after"]},"x":s::number(),"y":s::number()}),
                &["expected_revision"]), true),
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

fn assets(workspace: Option<&DesignerWorkspace>) -> Vec<DesignerAsset> {
    workspace
        .and_then(DesignerWorkspace::asset_toolbox)
        .map(|toolbox| toolbox.assets())
        .unwrap_or_else(crate::default_designer_assets)
}

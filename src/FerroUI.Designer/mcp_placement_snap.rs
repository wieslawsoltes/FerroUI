use super::super::{decode, schema as s};
use crate::{CanvasSnapAdjustment, CanvasSnapQuery, DesignerSession, DesignerWorkspace};
use ferroui_designer_core::NodeId;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapInput {
    epoch: u64,
    zoom: f64,
    #[serde(default)]
    bypass: bool,
}

pub(super) fn schema() -> Value {
    s::object(
        json!({"epoch":s::revision(),"zoom":{"type":"number","minimum":0.01,"maximum":8},
        "bypass":{"type":"boolean"}}),
        &["epoch", "zoom"],
    )
}

pub(super) fn resolve(
    workspace: Option<&DesignerWorkspace>,
    session: &DesignerSession,
    args: &Value,
    excluded: &[NodeId],
    adjust: impl FnOnce(&CanvasSnapQuery<'_>) -> Result<CanvasSnapAdjustment, String>,
) -> Result<CanvasSnapAdjustment, String> {
    if args.get("snap").is_none() {
        return Ok(CanvasSnapAdjustment::default());
    }
    let input: SnapInput = decode(args, "snap")?;
    let workspace =
        workspace.ok_or("Snapping settings belong to an open Designer Studio workspace")?;
    if workspace.canvas_guide_epoch() != input.epoch || !session.can_manipulate() {
        return Err(
            "The design or snapping settings changed. Read the current canvas guides".into(),
        );
    }
    let settings = workspace.canvas_guides();
    if input.bypass || !settings.snapping().enabled() {
        return Ok(CanvasSnapAdjustment::default());
    }
    let index = session.prepare_canvas_snap(&settings, excluded)?;
    adjust(&CanvasSnapQuery::new(&index, &settings, input.zoom)?)
}

//! Native stage coordinates are independent of the player's outer Viewbox scale.
//! Each overlay retains its own layout size and is clipped by the base screen.
use super::{PreviewPrototypeSession, RunningPrototype};
use crate::{
    PreviewPrototypeOverlayPlacement, PreviewPrototypePlayerLayer, PreviewStageId, PreviewStageSize,
};
use ferroui_base::Rect;

pub(super) fn layers(
    player: &RunningPrototype,
    session: &PreviewPrototypeSession,
) -> Result<(PreviewStageSize, Vec<PreviewPrototypePlayerLayer>), String> {
    let base = *player
        .stages
        .get(&session.screen())
        .ok_or("The prototype's base screen was removed")?;
    let size = player
        .board
        .stage(base)
        .ok_or("The native base screen was removed")?
        .layout_size;
    valid_size(size)?;
    let viewport = Rect::new(0.0, 0.0, size.width, size.height);
    let mut result = Vec::new();
    for (index, original) in session.active_stages().into_iter().enumerate() {
        let native = *player
            .stages
            .get(&original)
            .ok_or("The presented stage is outside this prototype flow")?;
        let info = player
            .board
            .stage(native)
            .ok_or("The native prototype stage was removed")?;
        valid_size(info.layout_size)?;
        let overlay = index.checked_sub(1).map(|i| session.overlays()[i].options);
        let bounds = overlay.map_or(viewport, |options| {
            placement(size, info.layout_size, options.placement)
        });
        if !has_area(bounds.intersect(viewport)) {
            return Err(format!(
                "The overlay '{}' is entirely outside the base screen; change its position",
                info.configuration.name
            ));
        }
        result.push(PreviewPrototypePlayerLayer {
            stage: original,
            host: player
                .board
                .control(native)
                .ok_or("The native prototype view is unavailable")?,
            bounds,
            overlay,
        });
    }
    Ok((size, result))
}

pub(super) fn signature(layers: &[PreviewPrototypePlayerLayer]) -> Vec<(PreviewStageId, Rect)> {
    layers
        .iter()
        .map(|layer| (layer.stage, layer.bounds))
        .collect()
}

/// Return the visible hotspot in the active stage's local coordinates. Native
/// stage clipping alone does not include a partially off-frame overlay's parent.
pub(super) fn hotspot(
    board: &crate::PreviewStageBoard,
    native: PreviewStageId,
    anchor: &crate::PreviewSourceAnchor,
    size: PreviewStageSize,
    top: Rect,
) -> Result<Rect, String> {
    let local = board.prototype_hotspot_bounds(native, anchor)?;
    let translated = Rect::new(top.x + local.x, top.y + local.y, local.width, local.height);
    let visible = translated.intersect(Rect::new(0.0, 0.0, size.width, size.height));
    if !has_area(visible) {
        return Err("The prototype hotspot is clipped outside the player frame".into());
    }
    Ok(Rect::new(
        visible.x - top.x,
        visible.y - top.y,
        visible.width,
        visible.height,
    ))
}

fn valid_size(size: PreviewStageSize) -> Result<(), String> {
    if size.width.is_finite() && size.height.is_finite() && size.width > 0.0 && size.height > 0.0 {
        Ok(())
    } else {
        Err("The prototype frame has no finite native layout size".into())
    }
}

fn has_area(rect: Rect) -> bool {
    rect.width > 0.0
        && rect.height > 0.0
        && [rect.x, rect.y, rect.width, rect.height]
            .into_iter()
            .all(f64::is_finite)
}

fn placement(
    frame: PreviewStageSize,
    size: PreviewStageSize,
    placement: PreviewPrototypeOverlayPlacement,
) -> Rect {
    use PreviewPrototypeOverlayPlacement::*;
    let right = frame.width - size.width;
    let bottom = frame.height - size.height;
    let (x, y) = match placement {
        Center => (right / 2.0, bottom / 2.0),
        TopLeft => (0.0, 0.0),
        TopCenter => (right / 2.0, 0.0),
        TopRight => (right, 0.0),
        CenterLeft => (0.0, bottom / 2.0),
        CenterRight => (right, bottom / 2.0),
        BottomLeft => (0.0, bottom),
        BottomCenter => (right / 2.0, bottom),
        BottomRight => (right, bottom),
        Manual { x, y } => (f64::from(x), f64::from(y)),
    };
    Rect::new(x, y, size.width, size.height)
}

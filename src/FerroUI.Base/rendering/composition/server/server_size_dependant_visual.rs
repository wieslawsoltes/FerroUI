use super::ServerCompositionVisual;
use crate::platform::LtrbRect;

/// The behaviour of a visual whose content covers its size: the base of
/// the solid color and surface visuals.
pub struct ServerSizeDependantVisual;

impl ServerSizeDependantVisual {
    pub fn compute_own_content_bounds(visual: &ServerCompositionVisual) -> Option<LtrbRect> {
        let size = visual.size();
        if size.x == 0.0 || size.y == 0.0 {
            return None;
        }
        Some(LtrbRect::new(0.0, 0.0, size.x, size.y))
    }

    pub fn size_changed(visual: &ServerCompositionVisual) {
        visual.enqueue_for_own_bounds_recompute();
    }
}

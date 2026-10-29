//! Ported from `Composition/ParallaxAnimation.cs`.

use ferroui_base::numerics::Vector3;
use ferroui_base::rendering::composition::{CompositionVisual, ElementComposition};
use ferroui_base::{Ref, Vector};
use ferroui_controls::Control;
use std::rc::Rc;

/// Applies composition-based parallax offsets without requiring a behavior.
pub struct ParallaxAnimation {
    target: Ref<Control>,
    visual: Rc<CompositionVisual>,
}

impl ParallaxAnimation {
    fn new(target: Ref<Control>, visual: Rc<CompositionVisual>) -> Self {
        Self { target, visual }
    }

    /// Calculates the composition offset for a scroll offset and parallax ratio (the proportion
    /// of the scroll offset to apply). Returns the parallax delta to add to the target's layout
    /// offset.
    pub fn calculate_offset(scroll_offset: Vector, parallax_ratio: f64) -> Vector3 {
        Vector3::new((scroll_offset.x * parallax_ratio) as f32, (scroll_offset.y * parallax_ratio) as f32, 0.0)
    }

    /// Creates a parallax animation session that retains the target's composition visual, or
    /// `None` when the target visual is unavailable.
    pub fn try_create(target: Option<&Control>) -> Option<ParallaxAnimation> {
        let target = target?;
        let visual = ElementComposition::get_element_visual(target)?;
        Some(ParallaxAnimation::new(target.to_ref(), visual))
    }

    /// Applies a calculated parallax offset to the retained composition visual.
    pub fn apply(&self, scroll_offset: Vector, parallax_ratio: f64) {
        let delta = Self::calculate_offset(scroll_offset, parallax_ratio);
        let bounds = self.target.bounds();
        self.visual.set_offset(Vector3::new(bounds.x as f32 + delta.x, bounds.y as f32 + delta.y, delta.z).into());
    }

    /// Applies a calculated parallax offset to a control's composition visual. Returns `true`
    /// when a composition visual was updated; otherwise, `false`.
    ///
    /// Deviation (DEVIATIONS.md, Behaviors): the static `Apply(Control, Vector, double)` overload
    /// of the reference; Rust has no overloads.
    pub fn apply_to(target: Option<&Control>, scroll_offset: Vector, parallax_ratio: f64) -> bool {
        let Some(animation) = Self::try_create(target) else {
            return false;
        };

        animation.apply(scroll_offset, parallax_ratio);
        true
    }
}

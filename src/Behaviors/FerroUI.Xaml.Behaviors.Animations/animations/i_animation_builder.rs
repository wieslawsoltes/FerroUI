//! Ported from `Animations/IAnimationBuilder.cs`.

use ferroui_base::animation::Animation;
use ferroui_base::Ref;
use ferroui_controls::Control;

/// Provides a way to build [`Animation`] instances in code.
pub trait IAnimationBuilder {
    /// Creates an animation for the specified control: the control that will run the animation.
    /// Returns the created animation or `None`.
    fn build(&self, control: &Control) -> Option<Ref<Animation>>;
}

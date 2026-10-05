use super::IAnimationInstance;
use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::server::ServerObjectId;
use std::rc::Rc;

/// Base of the things that can be started on a composition object: a single
/// animation or a group of animations.
pub trait ICompositionAnimationBase: 'static {
    /// The animation when it is a single animation rather than a group
    /// (the `is CompositionAnimation` test of upstream).
    fn as_composition_animation(&self) -> Option<&dyn ICompositionAnimation> {
        None
    }
}

/// The part of a composition animation the generated property code needs:
/// creating the instance that runs on the server.
///
/// The animation classes themselves implement it.
pub trait ICompositionAnimation: ICompositionAnimationBase {
    /// The name of the property the animation targets, if set.
    fn target(&self) -> Option<String>;

    /// Creates the server-side instance of the animation for a property of
    /// `target_object`. `final_value` is the value the property was just
    /// set to when the animation is implicit.
    fn create_instance(
        &self,
        target_object: ServerObjectId,
        final_value: Option<ExpressionVariant>,
    ) -> Rc<dyn IAnimationInstance>;
}

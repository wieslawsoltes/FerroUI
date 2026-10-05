use super::{CompositionAnimationGroup, IAnimationInstance};
use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::server::ServerObjectId;
use std::rc::Rc;

/// Base class for composition animations.
///
/// What can be started on a composition object: a single animation
/// ([`ICompositionAnimation`]) or a group of animations
/// ([`CompositionAnimationGroup`]). Upstream the interface is closed to
/// other implementations by an internal member; here the two accessors
/// stand for the `is CompositionAnimation` and `is CompositionAnimationGroup`
/// tests of upstream.
pub trait ICompositionAnimationBase: 'static {
    /// The animation when it is a single animation rather than a group.
    fn as_composition_animation(&self) -> Option<&dyn ICompositionAnimation> {
        None
    }

    /// The group when it is a group of animations.
    fn as_composition_animation_group(&self) -> Option<&CompositionAnimationGroup> {
        None
    }
}

/// The members of `CompositionAnimation` that code outside the animation
/// classes uses: the target property and the creation of the instance that
/// runs on the server.
///
/// The animation classes implement it; `Rc<dyn ICompositionAnimation>` is
/// the handle of upstream's abstract `CompositionAnimation`.
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

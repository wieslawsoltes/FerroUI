//! Port of `Animations/PlayAnimationBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::animation::{Animatable, Animation};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref, StaticType,
    StyledElementImpl, StyledProperty, Visual,
};
use ferroui_xaml_behaviors_animations::AnimationRunner;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Plays a specified [`Animation`] when the associated element is attached to the visual tree.
#[repr(C)]
pub struct PlayAnimationBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(PlayAnimationBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(PlayAnimationBehavior { new: PlayAnimationBehavior::new });
ferroui_base::ferro_impl_classes!(
    PlayAnimationBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl PlayAnimationBehavior {
        /// Identifies the `Animation` property.
        pub fn animation_property() -> StyledProperty<Option<Ref<Animation>>> {
            FerroProperty::register::<PlayAnimationBehavior, _>("Animation", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for PlayAnimationBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let animation = this.animation();
            let associated_object = this.associated_object_as::<Visual>();
            let target: Option<&Animatable> = associated_object.as_deref().map(crate::part_c::visual_as_animatable);
            AnimationRunner::try_run(animation.as_deref(), target);
            DisposableAction::empty()
        }
    }
}

impl PlayAnimationBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<Visual as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the animation that will be played. This is a property of the object model.
    pub fn animation(&self) -> Option<Ref<Animation>> {
        self.get_value(Self::animation_property())
    }

    pub fn set_animation(&self, value: Option<Ref<Animation>>) {
        self.set_value(Self::animation_property(), value)
    }
}

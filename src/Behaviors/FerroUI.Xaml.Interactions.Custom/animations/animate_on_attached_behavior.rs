//! Port of `Animations/AnimateOnAttachedBehavior.cs`.

use crate::{AnimationBuilderRef, AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::animation::Animation;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref, StaticType,
    StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_behaviors_animations::AnimationRunner;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Runs an animation when the associated control is attached to the visual tree.
#[repr(C)]
pub struct AnimateOnAttachedBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(AnimateOnAttachedBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(AnimateOnAttachedBehavior { new: AnimateOnAttachedBehavior::new });
ferroui_base::ferro_impl_classes!(
    AnimateOnAttachedBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl AnimateOnAttachedBehavior {
        /// Identifies the `Animation` property.
        pub fn animation_property() -> StyledProperty<Option<Ref<Animation>>> {
            FerroProperty::register::<AnimateOnAttachedBehavior, _>("Animation", None)
        }

        /// Identifies the `AnimationBuilder` property.
        pub fn animation_builder_property() -> StyledProperty<Option<AnimationBuilderRef>> {
            FerroProperty::register::<AnimateOnAttachedBehavior, _>("AnimationBuilder", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for AnimateOnAttachedBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return DisposableAction::empty();
            };

            let animation = this.animation();
            let animation_builder = this.animation_builder();
            AnimationRunner::try_build_and_run(
                Some(&*associated_object),
                animation.as_deref(),
                animation_builder.as_deref(),
            );

            DisposableAction::empty()
        }
    }
}

impl AnimateOnAttachedBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the animation to run.
    pub fn animation(&self) -> Option<Ref<Animation>> {
        self.get_value(Self::animation_property())
    }

    pub fn set_animation(&self, value: Option<Ref<Animation>>) {
        self.set_value(Self::animation_property(), value)
    }

    /// Gets or sets the animation builder used to create an animation.
    pub fn animation_builder(&self) -> Option<AnimationBuilderRef> {
        self.get_value(Self::animation_builder_property())
    }

    pub fn set_animation_builder(&self, value: Option<AnimationBuilderRef>) {
        self.set_value(Self::animation_builder_property(), value)
    }
}

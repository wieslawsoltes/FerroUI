//! Port of `Animations/RunAnimationTrigger.cs`.

use crate::{AnimationBuilderRef, AttachedToVisualTreeTriggerBase, AttachedToVisualTreeTriggerBaseImpl};
use ferroui_base::animation::Animation;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_behaviors_animations::AnimationRunner;
use ferroui_xaml_interactivity::{DisposableAction, DisposingTriggerImpl, Interaction, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Runs an animation and invokes actions when the associated control is attached to the visual
/// tree.
#[repr(C)]
pub struct RunAnimationTrigger {
    base: AttachedToVisualTreeTriggerBase,
}

ferro_class!(RunAnimationTrigger: AttachedToVisualTreeTriggerBase);
ferro_class_info!(RunAnimationTrigger { new: RunAnimationTrigger::new });
ferroui_base::ferro_impl_classes!(
    RunAnimationTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
);

ferro_properties! {
    impl RunAnimationTrigger {
        /// Identifies the `Animation` property.
        pub fn animation_property() -> StyledProperty<Option<Ref<Animation>>> {
            FerroProperty::register::<RunAnimationTrigger, _>("Animation", None)
        }

        /// Identifies the `AnimationBuilder` property.
        pub fn animation_builder_property() -> StyledProperty<Option<AnimationBuilderRef>> {
            FerroProperty::register::<RunAnimationTrigger, _>("AnimationBuilder", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeTriggerBaseImpl for RunAnimationTrigger {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return DisposableAction::empty();
            };

            let animation = this.animation();
            let animation_builder = this.animation_builder();
            let task = AnimationRunner::try_build_and_run_async(
                Some(&*associated_object),
                animation.as_deref(),
                animation_builder.as_deref(),
            );
            if let Some(task) = task {
                // Deviation (DEVIATIONS.md, Behaviors): the `ContinueWith` of the task of the
                // reference is a continuation of the task of the animation.
                let trigger: Ref<RunAnimationTrigger> = this.to_ref();
                task.on_completed(move || {
                    Dispatcher::ui_thread().post_local(
                        move || {
                            let sender =
                                trigger.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
                            Interaction::execute_actions(sender.as_ref(), trigger.actions().as_ref(), None);
                        },
                        DispatcherPriority::DEFAULT,
                    );
                });
            }

            DisposableAction::empty()
        }
    }
}

impl RunAnimationTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeTriggerBase::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a trigger.
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

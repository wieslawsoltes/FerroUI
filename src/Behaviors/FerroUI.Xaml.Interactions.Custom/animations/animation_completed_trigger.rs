//! Port of `Animations/AnimationCompletedTrigger.cs`.

use crate::AttachedToVisualTreeTrigger;
use ferroui_base::animation::Animation;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, Visual,
};
use ferroui_xaml_behaviors_animations::AnimationRunner;
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Runs a specified [`Animation`] and executes actions when it completes.
#[repr(C)]
pub struct AnimationCompletedTrigger {
    base: AttachedToVisualTreeTrigger,
}

ferro_class!(AnimationCompletedTrigger: AttachedToVisualTreeTrigger);
ferro_class_info!(AnimationCompletedTrigger { new: AnimationCompletedTrigger::new });
ferroui_base::ferro_impl_classes!(AnimationCompletedTrigger: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl AnimationCompletedTrigger {
        /// Identifies the `Animation` property.
        pub fn animation_property() -> StyledProperty<Option<Ref<Animation>>> {
            FerroProperty::register::<AnimationCompletedTrigger, _>("Animation", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for AnimationCompletedTrigger {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Visual>() else {
                return;
            };

            let Some(animation) = this.animation() else {
                this.execute(None);
                return;
            };

            // Deviation (DEVIATIONS.md, Behaviors): the asynchronous run of the reference is a
            // continuation of the task of the animation.
            let task = AnimationRunner::run_async(&animation, &associated_object);
            let trigger: Ref<AnimationCompletedTrigger> = this.to_ref();
            task.on_completed(move || {
                Dispatcher::ui_thread().post_local(move || trigger.execute(None), DispatcherPriority::DEFAULT);
            });
        }
    }
}

impl AnimationCompletedTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeTrigger::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the animation to run. This is a property of the object model.
    pub fn animation(&self) -> Option<Ref<Animation>> {
        self.get_value(Self::animation_property())
    }

    pub fn set_animation(&self, value: Option<Ref<Animation>>) {
        self.set_value(Self::animation_property(), value)
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        if !self.is_enabled() {
            return;
        }

        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        Interaction::execute_actions(sender.as_ref(), self.actions().as_ref(), parameter);
    }
}

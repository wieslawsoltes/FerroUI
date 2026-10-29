//! Port of `Animations/FadeInBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::animation::{Animatable, TimeSpan};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref, StaticType,
    StyledElementImpl, StyledProperty, Visual,
};
use ferroui_xaml_behaviors_animations::{AnimationFactory, AnimationRunner};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Plays a simple fade in animation when the associated control is attached.
#[repr(C)]
pub struct FadeInBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(FadeInBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(FadeInBehavior { new: FadeInBehavior::new });
ferroui_base::ferro_impl_classes!(
    FadeInBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl FadeInBehavior {
        /// Gets or sets the delay before the animation starts.
        pub fn initial_delay_property() -> StyledProperty<TimeSpan> {
            FerroProperty::register::<FadeInBehavior, _>("InitialDelay", TimeSpan::from_milliseconds(500.0))
        }

        /// Gets or sets the duration of the fade in animation.
        pub fn duration_property() -> StyledProperty<TimeSpan> {
            FerroProperty::register::<FadeInBehavior, _>("Duration", TimeSpan::from_milliseconds(250.0))
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for FadeInBehavior {
        /// Called when the behavior is attached to the visual tree.
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<Visual>() else {
                return DisposableAction::empty();
            };

            let animation = AnimationFactory::create_fade_in(this.initial_delay(), this.duration());
            let target: &Animatable = &associated_object;
            AnimationRunner::try_run(Some(&*animation), Some(target));

            DisposableAction::empty()
        }
    }
}

impl FadeInBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<Visual as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The delay before the animation starts.
    pub fn initial_delay(&self) -> TimeSpan {
        self.get_value(Self::initial_delay_property())
    }

    pub fn set_initial_delay(&self, value: TimeSpan) {
        self.set_value(Self::initial_delay_property(), value)
    }

    /// The duration of the fade in animation.
    pub fn duration(&self) -> TimeSpan {
        self.get_value(Self::duration_property())
    }

    pub fn set_duration(&self, value: TimeSpan) {
        self.set_value(Self::duration_property(), value)
    }
}

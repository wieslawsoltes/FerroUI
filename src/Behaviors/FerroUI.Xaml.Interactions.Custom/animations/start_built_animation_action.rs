//! Port of `Animations/StartBuiltAnimationAction.cs`.

use crate::AnimationBuilderRef;
use ferroui_base::animation::Animation;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_behaviors_animations::AnimationRunner;
use ferroui_xaml_interactivity::IAction;
use std::rc::Rc;

/// Starts an animation built in code on the associated control.
///
/// The action is a plain object of the object model (not a styled element) that implements the
/// action contract.
#[repr(C)]
pub struct StartBuiltAnimationAction {
    base: FerroObject,
}

ferro_class!(StartBuiltAnimationAction: FerroObject);
ferro_class_info!(StartBuiltAnimationAction { new: StartBuiltAnimationAction::new });
ferroui_base::ferro_impl_classes!(StartBuiltAnimationAction: FerroObjectImpl);

ferro_properties! {
    impl StartBuiltAnimationAction {
        /// Identifies the `Animation` property.
        pub fn animation_property() -> StyledProperty<Option<Ref<Animation>>> {
            FerroProperty::register::<StartBuiltAnimationAction, _>("Animation", None)
        }

        /// Identifies the `AnimationBuilder` property.
        pub fn animation_builder_property() -> StyledProperty<Option<AnimationBuilderRef>> {
            FerroProperty::register::<StartBuiltAnimationAction, _>("AnimationBuilder", None)
        }
    }
}

impl StartBuiltAnimationAction {
    /// The static constructor: the class implements the action contract.
    fn static_constructor() {
        <dyn IAction>::register::<StartBuiltAnimationAction>();
    }

    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates an action.
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

    /// Executes the action.
    pub fn execute(&self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let control = crate::part_c::sender_as::<Control>(sender);
        let animation = self.animation();
        let animation_builder = self.animation_builder();
        Some(Rc::new(AnimationRunner::try_build_and_run(
            control.as_deref(),
            animation.as_deref(),
            animation_builder.as_deref(),
        )))
    }
}

impl IAction for StartBuiltAnimationAction {
    fn execute(&self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        StartBuiltAnimationAction::execute(self, sender, parameter)
    }
}

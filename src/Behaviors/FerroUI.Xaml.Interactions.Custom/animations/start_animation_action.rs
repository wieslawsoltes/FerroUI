//! Port of `Animations/StartAnimationAction.cs`.

use ferroui_base::animation::{Animatable, Animation};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_behaviors_animations::AnimationRunner;
use ferroui_xaml_interactivity::IAction;
use std::rc::Rc;

/// Starts an [`Animation`] on the associated control.
///
/// The action is a plain object of the object model (not a styled element) that implements the
/// action contract.
#[repr(C)]
pub struct StartAnimationAction {
    base: FerroObject,
}

ferro_class!(StartAnimationAction: FerroObject);
ferro_class_info!(StartAnimationAction { new: StartAnimationAction::new });
ferroui_base::ferro_impl_classes!(StartAnimationAction: FerroObjectImpl);

ferro_properties! {
    impl StartAnimationAction {
        /// Gets or sets the animation to run.
        pub fn animation_property() -> StyledProperty<Option<Ref<Animation>>> {
            FerroProperty::register::<StartAnimationAction, _>("Animation", None)
        }
    }
}

impl StartAnimationAction {
    /// The static constructor: the class implements the action contract.
    fn static_constructor() {
        <dyn IAction>::register::<StartAnimationAction>();
    }

    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The animation to run.
    pub fn animation(&self) -> Option<Ref<Animation>> {
        self.get_value(Self::animation_property())
    }

    pub fn set_animation(&self, value: Option<Ref<Animation>>) {
        self.set_value(Self::animation_property(), value)
    }

    /// Executes the action: `sender` is the control initiating the action, `parameter` an optional
    /// parameter.
    ///
    /// Returns true if the animation was started; otherwise, false.
    pub fn execute(&self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let animation = self.animation();
        let control = crate::part_c::sender_as::<Control>(sender);
        let target: Option<&Animatable> = control.as_deref().map(crate::part_c::as_animatable);
        Some(Rc::new(AnimationRunner::try_run(animation.as_deref(), target)))
    }
}

impl IAction for StartAnimationAction {
    fn execute(&self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        StartAnimationAction::execute(self, sender, parameter)
    }
}

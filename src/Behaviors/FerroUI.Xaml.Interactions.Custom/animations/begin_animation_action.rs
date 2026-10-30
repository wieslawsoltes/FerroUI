//! Port of `Animations/BeginAnimationAction.cs`.

use ferroui_base::animation::{Animatable, Animation};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_behaviors_animations::AnimationRunner;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Starts an [`Animation`] on a specified control when executed.
#[repr(C)]
pub struct BeginAnimationAction {
    base: StyledElementAction,
}

ferro_class!(BeginAnimationAction: StyledElementAction);
ferro_class_info!(BeginAnimationAction {
    new: BeginAnimationAction::new,
    markup: {
        content: Animation,
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(BeginAnimationAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl BeginAnimationAction {
        /// Identifies the `Animation` property.
        pub fn animation_property() -> StyledProperty<Option<Ref<Animation>>> {
            FerroProperty::register::<BeginAnimationAction, _>("Animation", None)
        }

        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the control, so the value
        /// is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<BeginAnimationAction, _>("TargetControl", None)
        }
    }
}

impl StyledElementActionImpl for BeginAnimationAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let control = this.target_control().or_else(|| crate::part_c::sender_as::<Control>(sender));
        let animation = this.animation();
        let target: Option<&Animatable> = control.as_deref().map(crate::part_c::as_animatable);
        Some(Rc::new(AnimationRunner::try_run(animation.as_deref(), target)))
    }
}

impl BeginAnimationAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the animation to run. This is a property of the object model, and the content
    /// of the action in markup.
    pub fn animation(&self) -> Option<Ref<Animation>> {
        self.get_value(Self::animation_property())
    }

    pub fn set_animation(&self, value: Option<Ref<Animation>>) {
        self.set_value(Self::animation_property(), value)
    }

    /// Gets or sets the control on which the animation will run. This is a property of the object
    /// model; markup resolves a name given for it to the named control.
    pub fn target_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value.into().0))
    }
}

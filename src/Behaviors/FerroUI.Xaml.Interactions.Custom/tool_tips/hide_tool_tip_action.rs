//! Port of `ToolTips/HideToolTipAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::{Control, ToolTip};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Hides the [`ToolTip`] of the associated or target control when executed.
#[repr(C)]
pub struct HideToolTipAction {
    base: StyledElementAction,
}

ferro_class!(HideToolTipAction: StyledElementAction);
ferro_class_info!(HideToolTipAction {
    new: HideToolTipAction::new,
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(HideToolTipAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl HideToolTipAction {
        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the control, so the value
        /// is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<HideToolTipAction, _>("TargetControl", None)
        }
    }
}

impl StyledElementActionImpl for HideToolTipAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let control = this.target_control().or_else(|| crate::part_c::sender_as::<Control>(sender));
        let Some(control) = control else {
            return Some(Rc::new(false));
        };

        ToolTip::set_is_open(&control, false);
        Some(Rc::new(true))
    }
}

impl HideToolTipAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the control whose tooltip will be hidden. This is a property of the object
    /// model; markup resolves a name given for it to the named control.
    pub fn target_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value.into().0))
    }
}

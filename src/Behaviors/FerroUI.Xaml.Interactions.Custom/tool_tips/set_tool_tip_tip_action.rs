//! Port of `ToolTips/SetToolTipTipAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::{Control, ToolTip};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Sets the [`tip_property`](ToolTip::tip_property) of the associated or target control when
/// executed.
#[repr(C)]
pub struct SetToolTipTipAction {
    base: StyledElementAction,
}

ferro_class!(SetToolTipTipAction: StyledElementAction);
ferro_class_info!(SetToolTipTipAction {
    new: SetToolTipTipAction::new,
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(SetToolTipTipAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl SetToolTipTipAction {
        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the control, so the value
        /// is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<SetToolTipTipAction, _>("TargetControl", None)
        }

        /// Identifies the `Tip` property.
        pub fn tip_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<SetToolTipTipAction, _>("Tip", None)
        }
    }
}

impl StyledElementActionImpl for SetToolTipTipAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let control = this.target_control().or_else(|| crate::part_c::sender_as::<Control>(sender));
        let Some(control) = control else {
            return Some(Rc::new(false));
        };

        ToolTip::set_tip(&control, this.tip());
        Some(Rc::new(true))
    }
}

impl SetToolTipTipAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the control whose tooltip will be updated. This is a property of the object
    /// model; markup resolves a name given for it to the named control.
    pub fn target_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the new tooltip content. This is a property of the object model.
    pub fn tip(&self) -> Option<BoxedValue> {
        self.get_value(Self::tip_property())
    }

    pub fn set_tip(&self, value: Option<BoxedValue>) {
        self.set_value(Self::tip_property(), value)
    }
}

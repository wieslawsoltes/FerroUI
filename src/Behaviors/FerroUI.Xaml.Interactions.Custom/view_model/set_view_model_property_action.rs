//! Port of `ViewModel/SetViewModelPropertyAction.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::helpers::PropertyHelper;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Sets a view model property to a specified value when invoked.
#[repr(C)]
pub struct SetViewModelPropertyAction {
    base: StyledElementAction,
}

ferro_class!(SetViewModelPropertyAction: StyledElementAction);
ferro_class_info!(SetViewModelPropertyAction { new: SetViewModelPropertyAction::new });
ferroui_base::ferro_impl_classes!(SetViewModelPropertyAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl SetViewModelPropertyAction {
        /// Identifies the `PropertyName` property.
        pub fn property_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<SetViewModelPropertyAction, _>("PropertyName", None)
        }

        /// Identifies the `Value` property.
        pub fn value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<SetViewModelPropertyAction, _>("Value", None)
        }
    }
}

impl StyledElementActionImpl for SetViewModelPropertyAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let Some(control) = crate::part_c::sender_as::<Control>(sender) else {
            return Some(Rc::new(false));
        };

        let Some(target) = control.data_context() else {
            return Some(Rc::new(false));
        };

        let Some(property_name) = this.property_name().filter(|name| !name.is_empty()) else {
            return Some(Rc::new(false));
        };

        // The property helper throws in the reference when the property is not found or the
        // value does not convert.
        PropertyHelper::update_property_value(&target, &property_name, this.value().as_ref(), false)
            .unwrap_or_else(|error| panic!("{error}"));
        Some(Rc::new(true))
    }
}

impl SetViewModelPropertyAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the name of the property to change. This is a property of the object model.
    pub fn property_name(&self) -> Option<String> {
        self.get_value(Self::property_name_property())
    }

    pub fn set_property_name(&self, value: Option<String>) {
        self.set_value(Self::property_name_property(), value)
    }

    /// Gets or sets the value to assign. This is a property of the object model.
    pub fn value(&self) -> Option<BoxedValue> {
        self.get_value(Self::value_property())
    }

    /// Sets the value to assign (named apart from the setter of every property of the object).
    // Deviation (DEVIATIONS.md, Behaviors): the setter of `Value` is not named `set_value`, which is
    // the setter of every property of the object.
    pub fn set_action_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::value_property(), value)
    }
}

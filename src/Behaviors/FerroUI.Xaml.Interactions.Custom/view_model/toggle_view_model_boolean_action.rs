//! Port of `ViewModel/ToggleViewModelBooleanAction.cs`.

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::helpers::member_lookup::find_plain_property;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Toggles a boolean view model property when invoked.
#[repr(C)]
pub struct ToggleViewModelBooleanAction {
    base: StyledElementAction,
}

ferro_class!(ToggleViewModelBooleanAction: StyledElementAction);
ferro_class_info!(ToggleViewModelBooleanAction { new: ToggleViewModelBooleanAction::new });
ferroui_base::ferro_impl_classes!(ToggleViewModelBooleanAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ToggleViewModelBooleanAction {
        /// Identifies the `PropertyName` property.
        pub fn property_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ToggleViewModelBooleanAction, _>("PropertyName", None)
        }
    }
}

impl StyledElementActionImpl for ToggleViewModelBooleanAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        Some(Rc::new(this.toggle(sender)))
    }
}

impl ToggleViewModelBooleanAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the name of the property to toggle. This is a property of the object model.
    pub fn property_name(&self) -> Option<String> {
        self.get_value(Self::property_name_property())
    }

    pub fn set_property_name(&self, value: Option<String>) {
        self.set_value(Self::property_name_property(), value)
    }

    fn toggle(&self, sender: Option<&BoxedValue>) -> bool {
        if !self.is_enabled() {
            return false;
        }

        let Some(control) = crate::part_c::sender_as::<Control>(sender) else {
            return false;
        };

        let Some(target) = control.data_context() else {
            return false;
        };

        let Some(property_name) = self.property_name().filter(|name| !name.is_empty()) else {
            return false;
        };

        let Some(info) = find_plain_property(&*target, &property_name) else {
            return false;
        };
        // Deviation (DEVIATIONS.md, Behaviors): a property without a getter is not toggled (the
        // reference throws when it reads it).
        let (Some(get), Some(set)) = (info.get, info.set) else {
            return false;
        };
        if (info.type_)() != ValueType::of::<bool>() {
            return false;
        }

        let current = get(&[Some(target.clone())]).ok().flatten().and_then(ValueTypes::normalize);
        let Some(value) = current.and_then(|current| current.downcast_ref::<bool>().copied()) else {
            return false;
        };

        let toggled: BoxedValue = Rc::new(!value);
        if let Err(error) = set(&[Some(target.clone()), Some(toggled)]) {
            panic!("Cannot set the property {property_name}: {error:?}");
        }

        true
    }
}

//! Port of `ViewModel/IncrementViewModelPropertyAction.cs`.

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::helpers::member_lookup::find_plain_property;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Increments a numeric view model property when invoked.
#[repr(C)]
pub struct IncrementViewModelPropertyAction {
    base: StyledElementAction,
}

ferro_class!(IncrementViewModelPropertyAction: StyledElementAction);
ferro_class_info!(IncrementViewModelPropertyAction { new: IncrementViewModelPropertyAction::new });
ferroui_base::ferro_impl_classes!(IncrementViewModelPropertyAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl IncrementViewModelPropertyAction {
        /// Identifies the `PropertyName` property.
        pub fn property_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<IncrementViewModelPropertyAction, _>("PropertyName", None)
        }

        /// Identifies the `Delta` property.
        pub fn delta_property() -> StyledProperty<f64> {
            FerroProperty::register::<IncrementViewModelPropertyAction, _>("Delta", 1.0)
        }
    }
}

impl StyledElementActionImpl for IncrementViewModelPropertyAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        Some(Rc::new(this.increment(sender)))
    }
}

impl IncrementViewModelPropertyAction {
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

    /// Gets or sets the value to add. This is a property of the object model.
    pub fn delta(&self) -> f64 {
        self.get_value(Self::delta_property())
    }

    pub fn set_delta(&self, value: f64) {
        self.set_value(Self::delta_property(), value)
    }

    fn increment(&self, sender: Option<&BoxedValue>) -> bool {
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
        // Deviation (DEVIATIONS.md, Behaviors): a property without a getter is not incremented (the
        // reference throws when it reads it).
        let (Some(get), Some(set)) = (info.get, info.set) else {
            return false;
        };

        let Some(current) = get(&[Some(target.clone())]).ok().flatten().and_then(ValueTypes::normalize) else {
            return false;
        };

        // `Convert.ToDouble`: a value that is not a number is not incremented.
        let Some(mut value) = ValueTypes::try_convert(Some(&current), ValueType::of::<f64>())
            .flatten()
            .and_then(|value| value.downcast_ref::<f64>().copied())
        else {
            return false;
        };

        value += self.delta();

        // `Convert.ChangeType` throws when the sum does not convert to the type of the property.
        let property_type = (info.type_)();
        let boxed: BoxedValue = Rc::new(value);
        let Some(result) = ValueTypes::try_convert(Some(&boxed), property_type) else {
            panic!("Invalid cast from 'Double' to '{}'.", property_type.name());
        };

        if let Err(error) = set(&[Some(target.clone()), result]) {
            panic!("Cannot set the property {property_name}: {error:?}");
        }

        true
    }
}

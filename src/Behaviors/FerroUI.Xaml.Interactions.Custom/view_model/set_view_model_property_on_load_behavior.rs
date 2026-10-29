//! Port of `ViewModel/SetViewModelPropertyOnLoadBehavior.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::helpers::PropertyHelper;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT};

/// Sets a view model property when the associated control is loaded.
#[repr(C)]
pub struct SetViewModelPropertyOnLoadBehavior {
    base: StyledElementBehaviorOfT,
}

ferro_class!(SetViewModelPropertyOnLoadBehavior: StyledElementBehaviorOfT);
ferro_class_info!(SetViewModelPropertyOnLoadBehavior { new: SetViewModelPropertyOnLoadBehavior::new });
ferroui_base::ferro_impl_classes!(SetViewModelPropertyOnLoadBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl SetViewModelPropertyOnLoadBehavior {
        /// Identifies the `PropertyName` property.
        pub fn property_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<SetViewModelPropertyOnLoadBehavior, _>("PropertyName", None)
        }

        /// Identifies the `Value` property.
        pub fn value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<SetViewModelPropertyOnLoadBehavior, _>("Value", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for SetViewModelPropertyOnLoadBehavior {
        fn on_loaded(this: &Self) {
            Self::parent_on_loaded(this);

            let Some(target) = this.associated_object_as::<Control>().and_then(|control| control.data_context()) else {
                return;
            };

            let Some(property_name) = this.property_name().filter(|name| !name.is_empty()) else {
                return;
            };

            // The property helper throws in the reference when the property is not found or the
            // value does not convert.
            PropertyHelper::update_property_value(&target, &property_name, this.value().as_ref(), false)
                .unwrap_or_else(|error| panic!("{error}"));
        }
    }
}

impl SetViewModelPropertyOnLoadBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the property name to change. This is a property of the object model.
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
    pub fn set_behavior_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::value_property(), value)
    }
}

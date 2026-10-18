//! `TreeDataGridBindingProbe.cs`: the element whose two-way property a binding of a declarative
//! column is bound to, so that the binding is evaluated against a model by the framework's binding
//! system.

use ferroui_base::data::BindingMode;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, ferro_property, instantiate, BoxedValue, FerroObjectImpl,
    FerroProperty, Ref, StyledElement, StyledElementImpl, StyledProperty, StyledPropertyOptions,
};

/// A styled element with a two-way `Value` property (`TreeDataGridBindingProbe`).
#[repr(C)]
pub struct TreeDataGridBindingProbe {
    base: StyledElement,
}

ferro_class!(TreeDataGridBindingProbe: StyledElement);
ferroui_base::ferro_class_info!(TreeDataGridBindingProbe { new: TreeDataGridBindingProbe::new });
ferro_impl_classes!(TreeDataGridBindingProbe: FerroObjectImpl, StyledElementImpl);

ferro_properties! { impl TreeDataGridBindingProbe {
    ferro_property!(
        /// Defines the `Value` property, two-way by default.
        pub fn value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register_with::<TreeDataGridBindingProbe, _>(
                "Value",
                StyledPropertyOptions::new(None).default_binding_mode(BindingMode::TwoWay),
            )
        }
    );
} }

impl TreeDataGridBindingProbe {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElement::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The value the binding produced.
    pub fn value(&self) -> Option<BoxedValue> {
        self.get_value(Self::value_property())
    }

    /// Sets [`value`](Self::value).
    pub fn set_probe_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::value_property(), value)
    }
}

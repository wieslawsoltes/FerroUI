//! The element a binding of a [`TableViewColumn`](crate::TableViewColumn) is bound to so that the
//! value it selects from an item is read (the value a column sorts by) and written (an edited
//! cell), by the framework's binding system, without a cell.

use ferroui_base::data::{BindingBase, BindingExpressionBase, BindingMode, BindingNotification, BindingOperations};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObjectImpl,
    FerroProperty, Ref, StyledElement, StyledElementImpl, StyledProperty, StyledPropertyOptions,
};
use std::rc::Rc;

/// A styled element with a two-way `Value` property.
#[repr(C)]
pub struct TableViewBindingProbe {
    base: StyledElement,
}

ferro_class!(TableViewBindingProbe: StyledElement);
ferro_class_info!(TableViewBindingProbe { new: TableViewBindingProbe::new });
ferro_impl_classes!(TableViewBindingProbe: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl TableViewBindingProbe {
        /// Defines the `Value` property, two-way by default.
        pub fn value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register_with::<TableViewBindingProbe, _>(
                "Value",
                StyledPropertyOptions::new(None).default_binding_mode(BindingMode::TwoWay),
            )
        }
    }
}

impl TableViewBindingProbe {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElement::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// Reads (and writes) the value a binding selects from an item.
pub(crate) struct TableViewBindingAccessor {
    probe: Ref<TableViewBindingProbe>,
    expression: Rc<dyn BindingExpressionBase>,
}

impl TableViewBindingAccessor {
    pub(crate) fn new(binding: &dyn BindingBase) -> Self {
        let probe = TableViewBindingProbe::new();
        let expression = probe.bind_binding(TableViewBindingProbe::value_property().as_property(), binding);
        Self { probe, expression }
    }

    fn set_item(&self, item: &Option<BoxedValue>) {
        let same = match (self.probe.data_context(), item) {
            (Some(current), Some(item)) => std::ptr::addr_eq(Rc::as_ptr(&current), Rc::as_ptr(item)),
            (None, None) => true,
            _ => false,
        };
        if !same {
            self.probe.set_data_context(item.clone());
        }
    }

    /// The value the binding selects from `item`.
    pub(crate) fn read(&self, item: &Option<BoxedValue>) -> Option<BoxedValue> {
        self.set_item(item);
        self.expression.update_target();
        let value = self.probe.get_value(TableViewBindingProbe::value_property());
        let value = BindingNotification::extract_value(value.as_ref());
        if BindingOperations::is_unset(value.as_ref()) {
            return None;
        }
        value.and_then(ValueTypes::normalize)
    }

    /// Writes `value` to `item` through the binding (which converts it to the type of the
    /// source), then reads the value back.
    pub(crate) fn write(&self, item: &Option<BoxedValue>, value: Option<BoxedValue>) -> Option<BoxedValue> {
        self.set_item(item);
        self.probe.set_current_value(TableViewBindingProbe::value_property(), value);
        self.expression.update_source();
        self.read(item)
    }
}

impl Drop for TableViewBindingAccessor {
    fn drop(&mut self) {
        self.expression.dispose();
    }
}

use crate::collections::FerroList;
use crate::data::compiled_binding::binding_properties;
use crate::data::converters::{IMultiValueConverter, StringFormatMultiValueConverter};
use crate::data::core::plugins::property_value_type;
use crate::data::core::{MultiBindingExpression, ValueType};
use crate::data::{BindingBase, BindingExpressionBase, BindingMode, BindingPriority, RelativeSource};
use crate::utilities::CultureInfo;
use crate::{BoxedValue, FerroObject, FerroProperty, Ref};
use std::rc::Rc;

binding_properties! {
    /// A binding that aggregates multiple bindings through a converter.
    ///
    /// A binding is a mutable reference object: it is created with
    /// [`MultiBinding::new`], configured property by property (or with the
    /// chaining `with_*` forms) and shared as `Rc<MultiBinding>`, which
    /// converts to `Rc<dyn BindingBase>`. Handles compare by identity. The
    /// properties, and the contents of the collection of child bindings, are
    /// read when the binding is instantiated on a target: changing them
    /// afterwards affects later instances only.
    pub struct MultiBinding {
        /// The collection of child bindings. The list is a shared handle:
        /// children are added to the list the getter returns.
        bindings / set_bindings / with_bindings_list: FerroList<Rc<dyn BindingBase>> = FerroList::new(),
        /// The converter to use.
        converter / set_converter / with_converter_value: Option<Rc<dyn IMultiValueConverter>> = None,
        /// The culture in which to evaluate the converter. `None` (the
        /// default) uses the current culture.
        converter_culture / set_converter_culture / with_converter_culture: Option<CultureInfo> = None,
        /// A parameter to pass to the converter.
        converter_parameter / set_converter_parameter / with_converter_parameter: Option<BoxedValue> = None,
        /// The value to use when the binding is unable to produce a value;
        /// the unset marker for none.
        fallback_value / set_fallback_value / with_fallback_value: Option<BoxedValue> = Some(FerroProperty::unset_value()),
        /// The binding mode. Only one-time and one-way are supported.
        mode / set_mode / with_mode: BindingMode = BindingMode::OneWay,
        /// The binding priority.
        priority / set_priority / with_priority: BindingPriority = BindingPriority::LocalValue,
        /// The relative source for the binding.
        relative_source / set_relative_source / with_relative_source: Option<Rc<RelativeSource>> = None,
        /// The string format.
        string_format / set_string_format / with_string_format: Option<String> = None,
        /// The value to use when the binding produces null; the unset marker
        /// for none.
        target_null_value / set_target_null_value / with_target_null_value: Option<BoxedValue> = Some(FerroProperty::unset_value()),
    }
}

impl MultiBinding {
    /// Creates a multi-binding without child bindings.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The chaining form of adding child bindings: replaces the contents of
    /// the collection of child bindings.
    pub fn with_bindings(self: Rc<Self>, bindings: Vec<Rc<dyn BindingBase>>) -> Rc<Self> {
        let list = self.bindings();
        list.clear();
        list.add_range(bindings);
        self
    }

    /// The chaining form of [`set_converter`](Self::set_converter), for a
    /// converter.
    pub fn with_converter(self: Rc<Self>, converter: Rc<dyn IMultiValueConverter>) -> Rc<Self> {
        self.with_converter_value(Some(converter))
    }
}

impl BindingBase for MultiBinding {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn create_instance(
        &self,
        _target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
        _anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        let target_type = target_property.map_or_else(ValueType::object, property_value_type);
        let mut converter = self.converter();

        // The string format is only respected if the type of the target
        // property accepts a string.
        let string_format = self.string_format();
        if let Some(format) = string_format.as_deref().filter(|s| !s.trim().is_empty()) {
            if target_type.is_string() || target_type.is_object() {
                converter = Some(Rc::new(StringFormatMultiValueConverter::new(format, converter)));
            }
        }

        MultiBindingExpression::new(
            self.priority(),
            self.bindings().to_vec(),
            converter,
            self.converter_culture(),
            self.converter_parameter(),
            self.fallback_value(),
            self.target_null_value(),
        )
    }
}

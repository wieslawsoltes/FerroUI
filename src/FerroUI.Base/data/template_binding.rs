use crate::data::compiled_binding::binding_properties;
use crate::data::converters::IValueConverter;
use crate::data::{BindingBase, BindingExpressionBase, BindingMode, TemplateBindingExpression};
use crate::{BoxedValue, FerroObject, FerroProperty, Ref};
use std::rc::Rc;

binding_properties! {
    /// A lightweight binding to a property of an element's templated parent.
    ///
    /// A binding is a mutable reference object: it is created with
    /// [`TemplateBinding::empty`] or [`TemplateBinding::new`], configured
    /// property by property (or with the chaining `with_*` forms) and shared
    /// as `Rc<TemplateBinding>`, which converts to `Rc<dyn BindingBase>`.
    /// Handles compare by identity. The properties are read when the binding
    /// is instantiated on a target: changing them afterwards affects later
    /// instances only.
    pub struct TemplateBinding {
        /// The converter to use.
        converter / set_converter / with_converter_value: Option<Rc<dyn IValueConverter>> = None,
        /// A parameter to pass to the converter.
        converter_parameter / set_converter_parameter / with_converter_parameter: Option<BoxedValue> = None,
        /// The binding mode.
        mode / set_mode / with_mode: BindingMode = BindingMode::default(),
        /// The property of the templated parent being bound; `None` binds to
        /// the templated parent itself.
        property / set_property / with_property: Option<&'static FerroProperty> = None,
    }
}

impl TemplateBinding {
    /// Creates a binding without a property (the parameterless
    /// constructor).
    pub fn empty() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Creates a binding to the given property of the templated parent.
    pub fn new(property: &'static FerroProperty) -> Rc<Self> {
        Self::empty().with_property(Some(property))
    }

    /// The chaining form of [`set_converter`](Self::set_converter), for a
    /// converter.
    pub fn with_converter(self: Rc<Self>, converter: Rc<dyn IValueConverter>) -> Rc<Self> {
        self.with_converter_value(Some(converter))
    }

    /// The binding itself: what the markup extension form of the binding
    /// provides.
    pub fn provide_value(self: &Rc<Self>) -> Rc<dyn BindingBase> {
        self.clone()
    }
}

impl BindingBase for TemplateBinding {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn create_instance(
        &self,
        _target: &FerroObject,
        _target_property: Option<&'static FerroProperty>,
        _anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        let mode = self.mode();
        if matches!(mode, BindingMode::OneTime | BindingMode::OneWayToSource) {
            panic!("TemplateBinding does not support OneTime or OneWayToSource bindings.");
        }
        TemplateBindingExpression::new(
            self.property(),
            self.converter(),
            self.converter_parameter(),
            mode,
        )
    }
}

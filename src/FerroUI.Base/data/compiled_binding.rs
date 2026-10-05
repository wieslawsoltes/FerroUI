use crate::data::converters::IValueConverter;
use crate::data::core::expression_nodes::{DataContextNode, ExpressionNode, ParentDataContextNode};
use crate::data::compiled_binding_path::TypedPropertyElement;
use crate::data::core::plugins::property_value_type;
use crate::data::core::{BindingExpression, BindingExpressionOptions, TargetTypeConverter};
use crate::data::{
    BindingBase, BindingExpressionBase, BindingMode, BindingPriority, CompiledBindingPath, UpdateSourceTrigger,
};
use crate::{BoxedValue, FerroObject, FerroProperty, Ref, StyledElement, WeakRef};
use std::rc::Rc;

/// Resolves the binding mode and update source trigger defaults of a binding
/// against the metadata of its target property.
pub(crate) fn resolve_defaults_from_metadata(
    mode: BindingMode,
    update_source_trigger: UpdateSourceTrigger,
    target: &FerroObject,
    target_property: Option<&'static FerroProperty>,
) -> (BindingMode, UpdateSourceTrigger) {
    let trigger = if update_source_trigger == UpdateSourceTrigger::Default {
        UpdateSourceTrigger::PropertyChanged
    } else {
        update_source_trigger
    };
    let mode = if mode == BindingMode::Default {
        match target_property {
            Some(p) => p.get_metadata_for(target).default_binding_mode(),
            None => BindingMode::OneWay,
        }
    } else {
        mode
    };
    (mode, trigger)
}

/// The data context source node for a binding to `target_property`.
pub(crate) fn create_data_context_node(target_property: Option<&'static FerroProperty>) -> Rc<dyn ExpressionNode> {
    if target_property.is_some_and(|p| p.id() == StyledElement::data_context_property().id()) {
        ParentDataContextNode::new()
    } else {
        DataContextNode::new()
    }
}


/// Declares the properties of a binding class: the fields (with interior
/// mutability, as the binding is a mutable reference object), the getter and
/// setter named after the property, and the chaining form of the setter used
/// to configure a binding in one expression (`X::new(..).with_mode(..)`).
macro_rules! binding_properties {
    (
        $(#[$struct_meta:meta])*
        pub struct $name:ident {
            $(
                $(#[$meta:meta])*
                $field:ident / $set:ident / $with:ident : $ty:ty = $default:expr,
            )*
        }
    ) => {
        $(#[$struct_meta])*
        pub struct $name {
            $($field: ::std::cell::RefCell<$ty>,)*
        }

        impl Default for $name {
            fn default() -> Self {
                Self { $($field: ::std::cell::RefCell::new($default),)* }
            }
        }

        /// Compares by identity (reference equality), as the reference type
        /// this mirrors.
        impl PartialEq for $name {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                ::std::ptr::eq(self, other)
            }
        }

        impl $name {
            $(
                $(#[$meta])*
                pub fn $field(&self) -> $ty {
                    self.$field.borrow().clone()
                }

                pub fn $set(&self, value: $ty) {
                    *self.$field.borrow_mut() = value;
                }

                /// The chaining form of the setter.
                pub fn $with(self: ::std::rc::Rc<Self>, value: $ty) -> ::std::rc::Rc<Self> {
                    self.$set(value);
                    self
                }
            )*
        }
    };
}
pub(crate) use binding_properties;

/// Whether an untyped binding value is the unset marker.
pub(crate) fn is_unset(value: &Option<BoxedValue>) -> bool {
    value.as_ref().is_some_and(|v| v.is::<crate::UnsetValueType>())
}

/// The source of a binding in the form the source nodes take: `None` when
/// the source is the unset marker.
pub(crate) fn explicit_source(source: Option<BoxedValue>) -> Option<Option<BoxedValue>> {
    if is_unset(&source) {
        None
    } else {
        Some(source)
    }
}

binding_properties! {
    /// A binding that uses compiled paths: typed accessors instead of member
    /// lookup by name.
    ///
    /// A binding is a mutable reference object: it is created with
    /// [`CompiledBinding::empty`] or [`CompiledBinding::new`], configured
    /// property by property (or with the chaining `with_*` forms) and shared
    /// as `Rc<CompiledBinding>`, which converts to `Rc<dyn BindingBase>`.
    /// Handles compare by identity. The properties are read when the binding
    /// is instantiated on a target: changing them afterwards affects later
    /// instances only.
    pub struct CompiledBinding {
        /// The converter to use.
        converter / set_converter / with_converter_value: Option<Rc<dyn IValueConverter>> = None,
        /// A parameter to pass to the converter.
        converter_parameter / set_converter_parameter / with_converter_parameter: Option<BoxedValue> = None,
        /// The amount of time, in milliseconds, to wait before updating the
        /// binding source after the value on the target changes.
        delay / set_delay / with_delay: i32 = 0,
        /// The value to use when the binding is unable to produce a value;
        /// the unset marker for none.
        fallback_value / set_fallback_value / with_fallback_value: Option<BoxedValue> = Some(FerroProperty::unset_value()),
        /// The binding mode.
        mode / set_mode / with_mode: BindingMode = BindingMode::Default,
        /// The binding path.
        path / set_path / with_path: Option<CompiledBindingPath> = None,
        /// The binding priority.
        priority / set_priority / with_priority: BindingPriority = BindingPriority::LocalValue,
        /// The source for the binding; the unset marker (the default) binds
        /// to the data context or to the element selected by the path.
        source / set_source / with_source: Option<BoxedValue> = Some(FerroProperty::unset_value()),
        /// The string format.
        string_format / set_string_format / with_string_format: Option<String> = None,
        /// The value to use when the binding produces null; the unset marker
        /// for none.
        target_null_value / set_target_null_value / with_target_null_value: Option<BoxedValue> = Some(FerroProperty::unset_value()),
        /// What triggers updates of the binding source in two-way and
        /// one-way-to-source bindings.
        update_source_trigger / set_update_source_trigger / with_update_source_trigger: UpdateSourceTrigger = UpdateSourceTrigger::Default,
        /// The anchor to use if the target is not an element.
        default_anchor / set_default_anchor / with_default_anchor: Option<WeakRef<FerroObject>> = None,
    }
}

impl CompiledBinding {
    /// Creates a binding without a path (the parameterless constructor).
    pub fn empty() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Creates a binding with the given path.
    pub fn new(path: CompiledBindingPath) -> Rc<Self> {
        Self::empty().with_path(Some(path))
    }

    /// The chaining form of [`set_converter`](Self::set_converter), for a
    /// converter.
    pub fn with_converter(self: Rc<Self>, converter: Rc<dyn IValueConverter>) -> Rc<Self> {
        self.with_converter_value(Some(converter))
    }
}

impl CompiledBinding {
    /// Decides whether the binding can be instantiated as a typed binding
    /// expression, and returns the path element that creates it.
    fn typed_element_for(
        &self,
        target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
    ) -> Option<Rc<dyn TypedPropertyElement>> {
        // A path with a single typed property element is needed.
        let typed = self.path.borrow().as_ref()?.single_typed_element()?.clone();

        // A data context binding is needed.
        if !is_unset(&self.source.borrow()) {
            return None;
        }

        // It cannot have a converter, fallback value, string format, target
        // null value or a non-default update source trigger.
        if self.converter.borrow().is_some()
            || !is_unset(&self.fallback_value.borrow())
            || self.string_format.borrow().is_some()
            || !is_unset(&self.target_null_value.borrow())
            || !matches!(
                self.update_source_trigger(),
                UpdateSourceTrigger::Default | UpdateSourceTrigger::PropertyChanged
            )
            || self.delay() != 0
        {
            return None;
        }

        // The value must be directly assignable to the target property.
        let target_property = target_property?;
        let target_type = property_value_type(target_property);
        if !crate::data::core::ValueTypes::is_assignable(typed.value_type(), target_type) {
            return None;
        }

        // The typed expression listens for data context changes of a styled
        // element.
        if !target.is::<StyledElement>() {
            return None;
        }

        // Data context bindings read their source from the parent of the
        // target, which the typed expression does not support.
        if target_property.id() == StyledElement::data_context_property().id() {
            return None;
        }

        // The typed expression does not support data validation.
        if target_property.get_metadata_for(target).enable_data_validation() == Some(true) {
            return None;
        }

        // For modes that write back to the source, the source property must
        // be settable and the target value must be assignable back to the
        // source property type.
        let (mode, _) =
            resolve_defaults_from_metadata(self.mode(), self.update_source_trigger(), target, Some(target_property));
        if matches!(mode, BindingMode::TwoWay | BindingMode::OneWayToSource)
            && (!typed.can_set() || !crate::data::core::ValueTypes::is_assignable(target_type, typed.value_type()))
        {
            return None;
        }

        Some(typed)
    }
}

impl BindingBase for CompiledBinding {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn create_instance(
        &self,
        target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        if let Some(typed) = self.typed_element_for(target, target_property) {
            // The update source trigger is constrained to property-changed
            // by the check, so only the mode needs to be resolved here.
            let (mode, _) =
                resolve_defaults_from_metadata(self.mode(), self.update_source_trigger(), target, target_property);
            return typed.create_expression(mode, self.priority());
        }

        let enable_data_validation =
            target_property.is_some_and(|p| p.get_metadata_for(target).enable_data_validation() == Some(true));
        let mut nodes: Vec<Rc<dyn ExpressionNode>> = Vec::new();
        let path = self.path();
        let is_rooted = path.as_ref().is_some_and(|p| p.build_expression(&mut nodes));
        let explicit_source = explicit_source(self.source());

        // If the binding isn't rooted (doesn't have a source or start with an
        // element that selects one) a data context source node is needed.
        if explicit_source.is_none() && !is_rooted {
            nodes.insert(0, create_data_context_node(target_property));
        }

        // If the first node is a source node then allow it to select the
        // source; otherwise use the binding source if specified, falling back
        // to the target.
        let target_ref = target.to_ref();
        let default_anchor = self.default_anchor().as_ref().and_then(WeakRef::upgrade);
        let anchor = anchor.or(default_anchor.as_ref());
        let source = match nodes.first().and_then(|n| n.as_source_node()) {
            Some(source_node) => source_node.select_source(explicit_source.as_ref(), &target_ref, anchor),
            None => match &explicit_source {
                Some(source) => source.clone(),
                None => Some(Rc::new(target_ref.clone()) as BoxedValue),
            },
        };

        let (mode, trigger) =
            resolve_defaults_from_metadata(self.mode(), self.update_source_trigger(), target, target_property);

        BindingExpression::new(
            source,
            nodes,
            BindingExpressionOptions {
                delay: self.delay(),
                fallback_value: self.fallback_value(),
                converter: self.converter(),
                converter_parameter: self.converter_parameter(),
                enable_data_validation,
                mode,
                priority: self.priority(),
                string_format: self.string_format(),
                target_null_value: self.target_null_value(),
                target_property,
                target_type_converter: Some(TargetTypeConverter::get_default_converter()),
                update_source_trigger: trigger,
            },
        )
    }
}

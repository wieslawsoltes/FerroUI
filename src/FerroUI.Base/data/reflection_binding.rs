use crate::controls::{INameScope, NameScopeRef};
use crate::data::compiled_binding::{
    binding_properties, create_data_context_node, explicit_source, resolve_defaults_from_metadata,
};
use crate::data::converters::IValueConverter;
use crate::data::core::expression_nodes::{ExpressionNode, NamedElementNode};
use crate::data::core::parsers::{BindingExpressionGrammar, ExpressionNodeFactory, TypeResolver};
use crate::data::core::{BindingExpression, BindingExpressionOptions, ExpressionParseException, TargetTypeConverter};
use crate::data::{
    BindingBase, BindingExpressionBase, BindingMode, BindingPriority, RelativeSource, UpdateSourceTrigger,
};
use crate::{BoxedValue, FerroObject, FerroProperty, Ref, WeakRef};
use std::rc::{Rc, Weak};

binding_properties! {
    /// A binding whose path is text, resolved at run time against the
    /// metadata that the objects along the path declared: registered
    /// properties for objects of the class hierarchy, declared model metadata
    /// for everything else.
    ///
    /// A binding is a mutable reference object: it is created with
    /// [`ReflectionBinding::empty`] or [`ReflectionBinding::new`], configured
    /// property by property (or with the chaining `with_*` forms) and shared
    /// as `Rc<ReflectionBinding>`, which converts to `Rc<dyn BindingBase>`.
    /// Handles compare by identity. The properties are read when the binding
    /// is instantiated on a target: changing them afterwards affects later
    /// instances only.
    pub struct ReflectionBinding {
        /// The converter to use.
        converter / set_converter / with_converter_value: Option<Rc<dyn IValueConverter>> = None,
        /// A parameter to pass to the converter.
        converter_parameter / set_converter_parameter / with_converter_parameter: Option<BoxedValue> = None,
        /// The amount of time, in milliseconds, to wait before updating the
        /// binding source after the value on the target changes.
        delay / set_delay / with_delay: i32 = 0,
        /// The name of the element to use as the binding source.
        element_name / set_element_name / with_element_name: Option<String> = None,
        /// The value to use when the binding is unable to produce a value;
        /// the unset marker for none.
        fallback_value / set_fallback_value / with_fallback_value: Option<BoxedValue> = Some(FerroProperty::unset_value()),
        /// The binding mode.
        mode / set_mode / with_mode: BindingMode = BindingMode::Default,
        /// The binding path.
        path / set_path / with_path: String = String::new(),
        /// The binding priority.
        priority / set_priority / with_priority: BindingPriority = BindingPriority::LocalValue,
        /// The relative source for the binding.
        relative_source / set_relative_source / with_relative_source: Option<Rc<RelativeSource>> = None,
        /// The source for the binding; the unset marker (the default) for
        /// none.
        source / set_source / with_source: Option<BoxedValue> = Some(FerroProperty::unset_value()),
        /// The string format.
        string_format / set_string_format / with_string_format: Option<String> = None,
        /// The value to use when the binding produces null; the unset marker
        /// for none.
        target_null_value / set_target_null_value / with_target_null_value: Option<BoxedValue> = Some(FerroProperty::unset_value()),
        /// What triggers updates of the binding source in two-way and
        /// one-way-to-source bindings.
        update_source_trigger / set_update_source_trigger / with_update_source_trigger: UpdateSourceTrigger = UpdateSourceTrigger::Default,
        /// Resolves the type names in the path (casts, attached properties,
        /// ancestor types).
        type_resolver / set_type_resolver / with_type_resolver: Option<TypeResolver> = None,
        /// The anchor to use if the target is not an element.
        default_anchor / set_default_anchor / with_default_anchor: Option<WeakRef<FerroObject>> = None,
        /// The name scope in which named elements are looked up.
        name_scope / set_name_scope / with_name_scope: Option<Weak<dyn INameScope>> = None,
    }
}

impl ReflectionBinding {
    /// Creates a binding without a path (the parameterless constructor).
    pub fn empty() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Creates a binding with the given path.
    pub fn new(path: &str) -> Rc<Self> {
        Rc::new(Self::construct(path))
    }

    /// The class data of a binding with the given path, for a type that
    /// derives from this one.
    pub fn construct(path: &str) -> Self {
        let result = Self::default();
        result.set_path(path.to_string());
        result
    }

    /// The chaining form of [`set_converter`](Self::set_converter), for a
    /// converter.
    pub fn with_converter(self: Rc<Self>, converter: Rc<dyn IValueConverter>) -> Rc<Self> {
        self.with_converter_value(Some(converter))
    }

    fn get_name_scope(&self) -> Option<NameScopeRef> {
        self.name_scope().as_ref().and_then(Weak::upgrade).map(NameScopeRef)
    }

    fn create_source_node(&self, target_property: Option<&'static FerroProperty>) -> Option<Rc<dyn ExpressionNode>> {
        let element_name = self.element_name();
        if let Some(element_name) = element_name.as_deref().filter(|n| !n.is_empty()) {
            let name_scope =
                self.get_name_scope().expect("Cannot create ElementName binding when NameScope is null");
            return Some(NamedElementNode::new(Some(&name_scope), element_name));
        }
        if let Some(relative_source) = self.relative_source() {
            return ExpressionNodeFactory::create_relative_source(&relative_source);
        }
        Some(create_data_context_node(target_property))
    }

    /// Instantiates the binding, reporting a malformed path as an error
    /// instead of panicking.
    pub fn try_create_instance(
        &self,
        target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Result<Rc<BindingExpression>, ExpressionParseException> {
        let mut nodes: Vec<Rc<dyn ExpressionNode>> = Vec::new();
        let mut is_rooted = false;
        let enable_data_validation =
            target_property.is_some_and(|p| p.get_metadata_for(target).enable_data_validation() == Some(true));

        // Build the expression nodes from the binding path.
        let path = self.path();
        let explicit_source = explicit_source(self.source());
        if !path.is_empty() {
            let (ast, _source_mode) = BindingExpressionGrammar::parse(&path)?;
            let name_scope = self.get_name_scope();
            is_rooted = ExpressionNodeFactory::create_from_ast(
                &ast,
                self.type_resolver().as_ref(),
                name_scope.as_ref(),
                &mut nodes,
            )?;
        }

        // If the binding isn't rooted (doesn't have a source or start with
        // `$parent`, `$self`, `#elementName` etc.) a source node is needed.
        // Its kind depends on the element name and relative source of the
        // binding and defaults to a data context node.
        if explicit_source.is_none() && !is_rooted {
            if let Some(source_node) = self.create_source_node(target_property) {
                nodes.insert(0, source_node);
            }
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

        Ok(BindingExpression::new(
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
                target_type_converter: Some(TargetTypeConverter::get_reflection_converter()),
                update_source_trigger: trigger,
            },
        ))
    }
}

impl BindingBase for ReflectionBinding {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    /// Panics if the path is malformed (a programmer error, as in the
    /// managed implementation); use
    /// [`try_create_instance`](ReflectionBinding::try_create_instance) for
    /// paths that come from user input.
    fn create_instance(
        &self,
        target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        match self.try_create_instance(target, target_property, anchor) {
            Ok(expression) => expression,
            Err(e) => panic!("{e}"),
        }
    }
}

//! Port of `CompilerExtensions/Transformers/FerroXamlIlResourceTransformer.cs`.

use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

use xamlx::ast::{
    visit_node, IXamlAstNode, IXamlAstValueNode, IXamlPropertySetter,
    PropertySetterBinderParameters, XamlAstConstructableObjectNode, XamlAstExtensions,
    XamlAstNodeExtensions, XamlAstTextNode, XamlAstXmlDirective, XamlDeferredContentNode,
    XamlPropertyAssignmentNode,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer, XamlTransformHelpers};
use xamlx::type_system::{IXamlCustomAttribute, IXamlMethod, IXamlType};
use xamlx::XamlNamespaces;

use super::{FerroXamlIlWellKnownTypes, FerroXamlIlWellKnownTypesExtensions};
use crate::compiler_extensions::visitors::NameScopeRegistrationVisitor;

/// Transforms ResourceDictionary and IResourceDictionary property assignments
/// to use Add method calls with deferred content where applicable.
/// Additionally, handles x:Shared on assignments and injects XamlSourceInfo.
pub struct FerroXamlResourceTransformer {
    create_source_info: Cell<bool>,
}

impl Default for FerroXamlResourceTransformer {
    fn default() -> Self {
        Self {
            create_source_info: Cell::new(true),
        }
    }
}

impl FerroXamlResourceTransformer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets a value indicating whether source information should be generated and injected
    /// into the compiled XAML output.
    pub fn create_source_info(&self) -> bool {
        self.create_source_info.get()
    }

    pub fn set_create_source_info(&self, value: bool) {
        self.create_source_info.set(value);
    }

    fn resolve_adder_and_value(
        context: &AstTransformationContext,
        types: &FerroXamlIlWellKnownTypes,
        node: &Rc<dyn IXamlAstNode>,
        value_node: Rc<dyn IXamlAstValueNode>,
    ) -> XamlResult<(Rc<dyn IXamlMethod>, Rc<dyn IXamlAstValueNode>)> {
        if Self::should_be_deferred(&value_node)? {
            let adder = match Self::try_get_shared_value(context, node, &value_node)? {
                Some(is_shared) if !is_shared => {
                    types.resource_dictionary_not_shared_deferred_add.clone()
                }
                _ => types.resource_dictionary_deferred_add.clone(),
            };
            let deferred_node = XamlDeferredContentNode::new(
                value_node,
                Some(types.xaml_il_types.object.clone()),
                context.configuration(),
            )?;
            Ok((adder, deferred_node))
        } else {
            let adder = XamlTransformHelpers::find_possible_adders(
                context,
                &types.i_resource_dictionary,
            )?
            .into_iter()
            .next()
            .ok_or_else(|| {
                XamlError::transform_exception(
                    "No suitable Add method found for IResourceDictionary.",
                    Some(&**node),
                )
            })?;
            Ok((adder, value_node))
        }
    }

    /// `TryGetSharedValue(valueNode, out value)`: `Some(value)` is `true`.
    ///
    /// As upstream, the parsed value is not handed out: `value` is always `false` when the
    /// directive is valid, so any valid `x:Shared` directive (`True` or `False`) selects the
    /// not-shared adder.
    fn try_get_shared_value(
        context: &AstTransformationContext,
        node: &Rc<dyn IXamlAstNode>,
        value_node: &Rc<dyn IXamlAstValueNode>,
    ) -> XamlResult<Option<bool>> {
        let value = false;
        if let Some(co) = value_node.cast::<XamlAstConstructableObjectNode>() {
            // Try find x:Share directive
            let shared_directive = co
                .children
                .borrow()
                .iter()
                .find_map(|d| {
                    d.cast::<XamlAstXmlDirective>().filter(|d| {
                        d.namespace.borrow().as_deref() == Some(XamlNamespaces::XAML2006)
                            && *d.name.borrow() == "Shared"
                    })
                });
            if let Some(shared_directive) = shared_directive {
                let text = {
                    let values = shared_directive.values.borrow();
                    if values.len() == 1 {
                        Some(values[0].cast::<XamlAstTextNode>())
                    } else {
                        None
                    }
                };
                match text {
                    Some(Some(text)) => {
                        if parse_boolean(&text.text()).is_some() {
                            // If the parser succeeds, remove the x:Share directive
                            co.children
                                .borrow_mut()
                                .retain(|c| !c.same_node(&shared_directive));
                            return Ok(Some(value));
                        } else {
                            context.report_transform_error_node(
                                "Invalid argument type for x:Shared directive.",
                                node.clone(),
                            )?;
                        }
                    }
                    // One value that is not a text node: upstream reports the same error as
                    // for a wrong number of values.
                    Some(None) | None => {
                        context.report_transform_error_node(
                            "Invalid number of arguments for x:Shared directive.",
                            node.clone(),
                        )?;
                    }
                }
            }
        }
        Ok(None)
    }

    fn should_be_deferred(node: &Rc<dyn IXamlAstValueNode>) -> XamlResult<bool> {
        let clr_type = node.type_().get_clr_type()?;

        // XAML compiler is currently strict about value types, allowing them to be created only through converters.
        // At the moment it should be safe to not defer structs.
        if clr_type.is_value_type() {
            return Ok(false);
        }

        // Never defer strings.
        if clr_type.is("System", "String") {
            return Ok(false);
        }

        // Do not defer resources, if it has any x:Name registration, as it cannot be delayed.
        // This visitor will count x:Name registrations, ignoring nested NestedScopeMetadataNode scopes.
        // We set target scope level to 0, assuming that this resource node is a scope of itself.
        let mut name_registrations_visitor = NameScopeRegistrationVisitor::new(0, 0);
        visit_node(&node.as_node(), &mut name_registrations_visitor)?;
        if !name_registrations_visitor.is_empty() {
            return Ok(false);
        }

        Ok(true)
    }
}

/// `bool.TryParse`: `True`/`False` in any casing, surrounded by optional white space or
/// null characters.
fn parse_boolean(text: &str) -> Option<bool> {
    let trimmed = text.trim_matches(|c: char| c.is_whitespace() || c == '\0');
    if trimmed.eq_ignore_ascii_case("true") {
        Some(true)
    } else if trimmed.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

impl IXamlAstTransformer for FerroXamlResourceTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let Some(pa) = node.cast::<XamlPropertyAssignmentNode>() else {
            return Ok(node);
        };
        if pa.values.borrow().len() != 2 {
            return Ok(node);
        }

        let types = context.try_get_ferro_types()?;
        let document = context.current_document();

        if pa
            .property
            .declaring_type()
            .equals(&*types.resource_dictionary)
            && pa.property.name() == "Content"
        {
            let value = pa.values.borrow()[1].clone();
            let (adder, value) = Self::resolve_adder_and_value(context, &types, &node, value)?;

            pa.values.borrow_mut()[1] = value.clone();
            let setter: Rc<dyn IXamlPropertySetter> = ResourceAdderSetter::new(
                adder,
                self.create_source_info.get(),
                types.clone(),
                value.line(),
                value.position(),
                document,
            )?;
            *pa.possible_setters.borrow_mut() = vec![setter];
        } else if pa.property.name() == "Resources" {
            let getter = pa
                .property
                .getter()
                .filter(|getter| getter.return_type().equals(&*types.i_resource_dictionary));
            if let Some(getter) = getter {
                let value = pa.values.borrow()[1].clone();
                let (adder, value) =
                    Self::resolve_adder_and_value(context, &types, &node, value)?;

                pa.values.borrow_mut()[1] = value.clone();
                let setter: Rc<dyn IXamlPropertySetter> = ResourceAdderSetter::with_getter(
                    getter,
                    adder,
                    self.create_source_info.get(),
                    types.clone(),
                    value.line(),
                    value.position(),
                    document,
                )?;
                *pa.possible_setters.borrow_mut() = vec![setter];
            }
        }

        Ok(node)
    }
}

/// The upstream nested class `FerroXamlResourceTransformer.AdderSetter`: adds a keyed
/// resource to a resource dictionary and optionally records where it was declared.
///
/// Target type and parameters come from the adder (`Add(key, value)`,
/// `AddDeferred(key, deferredContent)` or `AddNotSharedDeferred(key, deferredContent)`):
/// parameters are `[key, value]`. Without a getter the target is the dictionary itself and
/// the setter allows multiple values; with a getter the target is the object that owns the
/// `Resources` property (the getter's declaring type), multiple values and attribute syntax
/// are not allowed. Null is allowed when the value parameter accepts it. The custom
/// attributes are the adder's.
///
/// Two setters are equal when they are the same object, or when both have a getter and the
/// getters and the adders are equal.
///
/// # What a back end has to do (upstream IL)
///
/// The setter is an optimized one. `info` below is `new XamlSourceInfo(line, position,
/// document)` built with `types.xaml_source_info_constructor` from
/// [`ResourceAdderSetter::line`], [`ResourceAdderSetter::position`] and
/// [`ResourceAdderSetter::document`] (`null` when `None`); it is attached with the static
/// method `types.xaml_source_info_dictionary_setter`:
/// `XamlSourceInfo.SetXamlSourceInfo(IResourceDictionary target, object key, XamlSourceInfo
/// info)`.
///
/// * `EmitWithArguments` (stack: target object; `arguments` are `[key, value]`; leaves
///   nothing):
///   1. when [`ResourceAdderSetter::getter`] is set, call it on the target, replacing the
///      target with the resources dictionary;
///   2. when [`ResourceAdderSetter::emit_source_info`] is set, duplicate the dictionary
///      reference (it is needed again after the add);
///   3. emit each argument converted to the matching entry of `parameters`; right after the
///      first one (the key), when `emit_source_info` is set, store it in a temporary and
///      load it back;
///   4. call [`ResourceAdderSetter::adder`] on the dictionary and discard a non-void result;
///   5. when `emit_source_info` is set: the duplicated dictionary is on the stack, load the
///      key from the temporary, build `info` and call the dictionary setter.
///
///   That is `var d = target[.Resources]; var k = key; d.Add(k, value);
///   XamlSourceInfo.SetXamlSourceInfo(d, k, info);`.
/// * `Emit` (stack: target object, key, value; leaves nothing): when there is neither a
///   getter nor source info, just call the adder. Otherwise store the arguments in
///   temporaries (last first; the key is copied to a second temporary when
///   `emit_source_info` is set), call the getter on the target when there is one, duplicate
///   the dictionary reference, load the arguments back, call the adder, and when
///   `emit_source_info` is set load the key copy, build `info` and call the dictionary
///   setter. Upstream duplicates the dictionary reference whenever this path is taken, also
///   when only the getter is set (and nothing consumes the copy afterwards).
pub struct ResourceAdderSetter {
    pub getter: Option<Rc<dyn IXamlMethod>>,
    pub adder: Rc<dyn IXamlMethod>,
    pub emit_source_info: bool,
    pub types: Rc<FerroXamlIlWellKnownTypes>,
    pub document: Option<String>,
    pub line: i32,
    pub position: i32,
    pub target_type: Rc<dyn IXamlType>,
    pub binder_parameters: PropertySetterBinderParameters,
    pub parameters: Vec<Rc<dyn IXamlType>>,
}

impl ResourceAdderSetter {
    fn create(
        getter: Option<Rc<dyn IXamlMethod>>,
        adder: Rc<dyn IXamlMethod>,
        emit_source_info: bool,
        ferro_types: Rc<FerroXamlIlWellKnownTypes>,
        line: i32,
        position: i32,
        document: Option<String>,
    ) -> XamlResult<Self> {
        let target_type = adder.this_or_first_parameter()?;
        let parameters: Vec<Rc<dyn IXamlType>> =
            adder.parameters_with_this().into_iter().skip(1).collect();

        let allow_null = parameters
            .last()
            .ok_or_else(|| {
                XamlError::invalid_operation("Sequence contains no elements")
            })?
            .accepts_null();
        Ok(Self {
            getter,
            adder,
            emit_source_info,
            types: ferro_types,
            document,
            line,
            position,
            target_type,
            binder_parameters: PropertySetterBinderParameters {
                allow_multiple: Cell::new(true),
                allow_x_null: Cell::new(allow_null),
                allow_runtime_null: Cell::new(allow_null),
                allow_attribute_syntax: Cell::new(true),
            },
            parameters,
        })
    }

    /// Creates an adder-only setter. Target is assumed to be already on the stack before emit.
    /// For example:
    /// `var resourceDictionary = ...`
    /// `resourceDictionary.Add(key, value);`
    /// `resourceDictionary.Add(key2, value2);`
    pub fn new(
        adder: Rc<dyn IXamlMethod>,
        emit_source_info: bool,
        ferro_types: Rc<FerroXamlIlWellKnownTypes>,
        line: i32,
        position: i32,
        document: Option<String>,
    ) -> XamlResult<Rc<Self>> {
        Ok(Rc::new(Self::create(
            None,
            adder,
            emit_source_info,
            ferro_types,
            line,
            position,
            document,
        )?))
    }

    /// Explicit target getter - target will be obtained by calling the getter first.
    pub fn with_getter(
        getter: Rc<dyn IXamlMethod>,
        adder: Rc<dyn IXamlMethod>,
        emit_source_info: bool,
        ferro_types: Rc<FerroXamlIlWellKnownTypes>,
        line: i32,
        position: i32,
        document: Option<String>,
    ) -> XamlResult<Rc<Self>> {
        let mut rv = Self::create(
            None,
            adder,
            emit_source_info,
            ferro_types,
            line,
            position,
            document,
        )?;
        rv.target_type = getter.declaring_type();
        rv.getter = Some(getter);
        rv.binder_parameters.allow_multiple.set(false);
        rv.binder_parameters.allow_attribute_syntax.set(false);
        Ok(Rc::new(rv))
    }
}

impl IXamlPropertySetter for ResourceAdderSetter {
    fn target_type(&self) -> Rc<dyn IXamlType> {
        self.target_type.clone()
    }
    fn binder_parameters(&self) -> &PropertySetterBinderParameters {
        &self.binder_parameters
    }
    fn parameters(&self) -> Vec<Rc<dyn IXamlType>> {
        self.parameters.clone()
    }
    fn custom_attributes(&self) -> Vec<Rc<dyn IXamlCustomAttribute>> {
        self.adder.custom_attributes()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn type_name(&self) -> &'static str {
        "AdderSetter"
    }
    fn equals(&self, other: &dyn IXamlPropertySetter) -> bool {
        let Some(other) = other.as_any().downcast_ref::<ResourceAdderSetter>() else {
            return false;
        };
        if std::ptr::eq(self, other) {
            return true;
        }

        match (&self.getter, &other.getter) {
            (Some(getter), Some(other_getter)) => {
                getter.equals(&**other_getter) && self.adder.equals(&*other.adder)
            }
            _ => false,
        }
    }
}

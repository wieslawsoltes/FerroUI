//! Port of `CompilerExtensions/FerroXamlIlLanguage.cs`.
//!
//! This file is used by the build tool. It refers to framework types by name only: no
//! dependency on the framework's object model is allowed here.

use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;

use xamlx::ast::{
    IXamlAstValueNode, XamlAstNodeExtensions, XamlAstTextNode,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::extensions::query_node_interface;
use xamlx::transform::{
    AstTransformationContext, IXamlCustomAttributeResolver, XamlLanguageTypeMappings,
    XamlValueConverter,
};
use xamlx::type_system::{
    FindMethodMethodSignature, IXamlCustomAttribute, IXamlMethod, IXamlProperty, IXamlType,
    IXamlTypeSystem, XamlValue, XamlVisibility,
};

use super::transformers::{
    FerroXamlIlTargetTypeMetadataNode, FerroXamlIlWellKnownTypesExtensions, ScopeTypes,
};
use super::{FerroXamlIlLanguageParseIntrinsics, XamlIlFerroPropertyHelper};

pub struct FerroXamlIlLanguage;

impl FerroXamlIlLanguage {
    /// The name of the runtime context field holding the current name scope.
    pub const CONTEXT_NAME_SCOPE_FIELD_NAME: &'static str = "FerroNameScope";

    /// `Configure(typeSystem)`: the language type mappings and the emit mappings.
    pub fn configure(
        type_system: &Rc<dyn IXamlTypeSystem>,
    ) -> XamlResult<(XamlLanguageTypeMappings, FerroXamlIlLanguageEmitMappings)> {
        let get = |name: &str| type_system.get_type(name);

        let runtime_helpers = get("FerroUI.Markup.Xaml.XamlIl.Runtime.XamlIlRuntimeHelpers")?;
        let mut rv = XamlLanguageTypeMappings::new(&**type_system)?;
        rv.support_initialize = Some(get("System.ComponentModel.ISupportInitialize")?);
        rv.xmlns_attributes
            .push(get("FerroUI.Metadata.XmlnsDefinitionAttribute")?);
        rv.content_attributes
            .push(get("FerroUI.Metadata.ContentAttribute")?);
        rv.whitespace_significant_collection_attributes
            .push(get("FerroUI.Metadata.WhitespaceSignificantCollectionAttribute")?);
        rv.trim_surrounding_whitespace_attributes
            .push(get("FerroUI.Metadata.TrimSurroundingWhitespaceAttribute")?);
        rv.provide_value_target = Some(get("FerroUI.Markup.Xaml.IProvideValueTarget")?);
        rv.root_object_provider = Some(get("FerroUI.Markup.Xaml.IRootObjectProvider")?);
        rv.root_object_provider_intermediate_root_property_name =
            Some("IntermediateRootObject".to_string());
        rv.uri_context_provider = Some(get("FerroUI.Markup.Xaml.IUriContext")?);
        rv.parent_stack_provider = Some(get(
            "FerroUI.Markup.Xaml.XamlIl.Runtime.IFerroXamlIlParentStackProvider",
        )?);

        rv.xml_namespace_info_provider = Some(get(
            "FerroUI.Markup.Xaml.XamlIl.Runtime.IFerroXamlIlXmlNamespaceInfoProvider",
        )?);
        rv.deferred_content_property_attributes
            .push(get("FerroUI.Metadata.TemplateContentAttribute")?);
        rv.deferred_content_executor_customization_default_type_parameter =
            Some(get("FerroUI.Controls.Control")?);
        rv.deferred_content_executor_customization_type_parameter_deferred_content_attribute_property_names =
            vec!["TemplateResultType".to_string()];
        rv.deferred_content_executor_customization =
            runtime_helpers.find_method(|m| m.name() == "DeferredTransformationFactoryV3");
        rv.usable_during_initialization_attributes
            .push(get("FerroUI.Metadata.UsableDuringInitializationAttribute")?);
        rv.inner_service_provider_factory_method =
            runtime_helpers.find_method(|m| m.name() == "CreateInnerServiceProviderV1");
        rv.i_add_child = Some(get("FerroUI.Metadata.IAddChild")?);
        rv.i_add_child_of_t = Some(get("FerroUI.Metadata.IAddChild`1")?);
        rv.custom_attribute_resolver = Some(Rc::new(AttributeResolver::new(type_system, &rv)?));

        let name_scope_type = get("FerroUI.Controls.INameScope")?;
        let eager_parent_stack_provider_interface_type = get(
            "FerroUI.Markup.Xaml.XamlIl.Runtime.IFerroXamlIlEagerParentStackProvider",
        )?;

        let emit = FerroXamlIlLanguageEmitMappings {
            type_system: type_system.clone(),
            runtime_helpers,
            name_scope_type,
            eager_parent_stack_provider_interface_type,
        };
        Ok((rv, emit))
    }

    /// `EmitNameScopeField`: the description of the name scope field of the runtime context.
    fn emit_name_scope_field(
        mappings: &XamlLanguageTypeMappings,
        type_system: &Rc<dyn IXamlTypeSystem>,
        name_scope_type: &Rc<dyn IXamlType>,
    ) -> XamlResult<FerroXamlIlContextNameScopeField> {
        let well_known_types = type_system.well_known_types();
        Ok(FerroXamlIlContextNameScopeField {
            field_type: name_scope_type.clone(),
            name: Self::CONTEXT_NAME_SCOPE_FIELD_NAME,
            visibility: XamlVisibility::Public,
            is_static: false,
            service_provider_get_service_method: mappings
                .service_provider()?
                .get_method_by_signature(&FindMethodMethodSignature::new(
                    "GetService",
                    well_known_types.object.clone(),
                    vec![well_known_types.type_.clone()],
                ))?,
        })
    }

    /// `EmitEagerParentStackProvider`: the description of the eager parent stack provider
    /// implementation of the runtime context.
    fn emit_eager_parent_stack_provider(
        mappings: &XamlLanguageTypeMappings,
        type_system: &Rc<dyn IXamlTypeSystem>,
        runtime_helpers: &Rc<dyn IXamlType>,
        interface_type: &Rc<dyn IXamlType>,
    ) -> XamlResult<FerroXamlIlContextEagerParentStackProvider> {
        let well_known_types = type_system.well_known_types();
        let implement_interface_property_getter = |property_name: &str| {
            let getter_name = format!("get_{property_name}");
            interface_type.get_method(|m| m.name() == getter_name)
        };

        // IReadOnlyList<object> DirectParentsStack => (IReadOnlyList<object>)ParentsStack;
        let direct_parents_stack_getter = implement_interface_property_getter("DirectParentsStack")?;

        let service_provider_get_service_method = mappings
            .service_provider()?
            .get_method_by_signature(&FindMethodMethodSignature::new(
                "GetService",
                well_known_types.object.clone(),
                vec![well_known_types.type_.clone()],
            ))?;

        let parent_stack_provider = mappings.parent_stack_provider.clone().ok_or_else(|| {
            XamlError::internal(
                "NullReferenceException",
                "XamlLanguageTypeMappings.ParentStackProvider is not set",
            )
        })?;
        let mut signature = FindMethodMethodSignature::new(
            "AsEagerParentStackProvider",
            interface_type.clone(),
            vec![parent_stack_provider.clone()],
        );
        signature.is_static = true;
        let as_eager_parent_stack_provider_method =
            runtime_helpers.get_method_by_signature(&signature)?;

        // IFerroXamlIlEagerParentStackProvider? ParentProvider
        // => XamlIlRuntimeHelpers.AsEagerParentStackProvider(_serviceProvider.GetService(typeof(IFerroXamlIlParentStackProvider)));
        let parent_provider_getter = implement_interface_property_getter("ParentProvider")?;

        Ok(FerroXamlIlContextEagerParentStackProvider {
            interface_type: interface_type.clone(),
            direct_parents_stack_getter,
            parent_provider_getter,
            parent_stack_provider,
            service_provider_get_service_method,
            as_eager_parent_stack_provider_method,
        })
    }

    /// `CustomValueConverter(context, node, customAttributes, type, out result)`:
    /// `Ok(Some(result))` is `true`, `Ok(None)` is `false`.
    pub fn custom_value_converter(
        context: &AstTransformationContext,
        node: &Rc<dyn IXamlAstValueNode>,
        custom_attributes: Option<&[Rc<dyn IXamlCustomAttribute>]>,
        type_: &Rc<dyn IXamlType>,
    ) -> XamlResult<Option<Rc<dyn IXamlAstValueNode>>> {
        if let Some(options_node) =
            query_node_interface::<dyn IOptionsMarkupExtensionNode>(&node.as_node())
        {
            if let Some(new_options_node) = options_node.convert_to_return_type(context, type_)? {
                return Ok(Some(new_options_node));
            }
        }

        let Some(text_node) = node.cast::<XamlAstTextNode>() else {
            return Ok(None);
        };

        let text = text_node.text();
        let types = context.try_get_ferro_types()?;

        if let Some(result) =
            FerroXamlIlLanguageParseIntrinsics::try_convert(context, node, &text, type_, &types)?
        {
            return Ok(Some(result));
        }

        if type_.is("FerroUI", "FerroProperty") {
            let attr_type = &types.inherit_data_type_from_attribute;
            let scope_kind = match custom_attributes
                .and_then(|attributes| attributes.iter().find(|a| a.type_().equals(&**attr_type)))
                .and_then(|a| a.parameters().into_iter().next())
            {
                Some(XamlValue::Int32(1)) => Some(ScopeTypes::Style),
                Some(XamlValue::Int32(2)) => Some(ScopeTypes::ControlTemplate),
                _ => None,
            };

            let scope = context
                .parent_nodes()
                .into_iter()
                .filter_map(|n| n.cast::<FerroXamlIlTargetTypeMetadataNode>())
                .find(|s| match scope_kind {
                    Some(scope_kind) => s.scope_type == scope_kind,
                    None => true,
                });
            let Some(scope) = scope else {
                // `Enum.IsDefined(scopeKind ?? default)`: the default value (0) is not a member.
                let scope_kind_str = match scope_kind {
                    Some(scope_kind) => scope_kind.name(),
                    None => "parent",
                };
                return Err(XamlError::load_exception(
                    format!("Unable to find the {scope_kind_str} scope for FerroProperty lookup"),
                    Some(&**node),
                ));
            };

            let result: Rc<dyn IXamlAstValueNode> =
                XamlIlFerroPropertyHelper::create_node(context, &text, scope.target_type(), &**node)?;
            return Ok(Some(result));
        }

        Ok(None)
    }

    /// [`FerroXamlIlLanguage::custom_value_converter`] as the configuration's value converter.
    pub fn value_converter() -> XamlValueConverter {
        Rc::new(Self::custom_value_converter)
    }
}

/// What the custom value converter needs from the `OptionsMarkupExtensionNode` of the option
/// markup extension transformer (`OnPlatform`, `OnFormFactor`): the node converts all its
/// branches to the requested type.
///
/// The node answers this interface from its `IXamlAstNode::query_interface`
/// (`xaml_query_interface!(self, slot, dyn IOptionsMarkupExtensionNode)`).
pub trait IOptionsMarkupExtensionNode: 'static {
    /// `ConvertToReturnType(context, type, out res)`: the node with every branch (and the default
    /// value) converted to `type_`, or `None` when one of them cannot be converted.
    fn convert_to_return_type(
        &self,
        context: &AstTransformationContext,
        type_: &Rc<dyn IXamlType>,
    ) -> XamlResult<Option<Rc<dyn IXamlAstValueNode>>>;
}

/// The emit side of the language (upstream `XamlLanguageEmitMappings<IXamlILEmitter,
/// XamlILNodeEmitResult>`), as data.
///
/// Upstream sets two members, both of which generate IL:
///
/// * `ProvideValueTargetPropertyEmitter = XamlIlFerroPropertyHelper.EmitProvideValueTarget`:
///   a back end calls [`XamlIlFerroPropertyHelper::try_get_provide_value_target`] and loads
///   what it describes.
/// * `ContextTypeBuilderCallback`: while the back end defines the runtime context type it calls
///   [`FerroXamlIlLanguageEmitMappings::context_type_builder_callback`] and adds the members
///   the returned [`FerroXamlIlContextDefinition`] describes.
pub struct FerroXamlIlLanguageEmitMappings {
    type_system: Rc<dyn IXamlTypeSystem>,
    runtime_helpers: Rc<dyn IXamlType>,
    name_scope_type: Rc<dyn IXamlType>,
    eager_parent_stack_provider_interface_type: Rc<dyn IXamlType>,
}

impl FerroXamlIlLanguageEmitMappings {
    /// `ContextTypeBuilderCallback(definition)`: the framework-specific members of the runtime
    /// context type. `mappings` are the type mappings returned by
    /// [`FerroXamlIlLanguage::configure`] next to this value.
    pub fn context_type_builder_callback(
        &self,
        mappings: &XamlLanguageTypeMappings,
    ) -> XamlResult<FerroXamlIlContextDefinition> {
        Ok(FerroXamlIlContextDefinition {
            name_scope_field: FerroXamlIlLanguage::emit_name_scope_field(
                mappings,
                &self.type_system,
                &self.name_scope_type,
            )?,
            eager_parent_stack_provider: FerroXamlIlLanguage::emit_eager_parent_stack_provider(
                mappings,
                &self.type_system,
                &self.runtime_helpers,
                &self.eager_parent_stack_provider_interface_type,
            )?,
        })
    }
}

/// The members the language adds to the runtime context type of a back end.
pub struct FerroXamlIlContextDefinition {
    pub name_scope_field: FerroXamlIlContextNameScopeField,
    pub eager_parent_stack_provider: FerroXamlIlContextEagerParentStackProvider,
}

/// The name scope field of the runtime context.
///
/// # What a back end has to do (upstream IL)
///
/// 1. Define an instance field named `name` (`FerroNameScope`) of type `field_type`
///    (`INameScope`) with `visibility` (public) on the context type.
/// 2. In the context constructor, whose first argument is the parent `IServiceProvider`:
///    load `this`, load constructor argument 1 (the parent service provider), load the type
///    token of `field_type` (`typeof(INameScope)`), call
///    `service_provider_get_service_method` (`IServiceProvider.GetService(Type)`) and store the
///    result in the field. The result is stored as returned (no cast is emitted); it is null
///    when the parent provider has no name scope.
///
/// Name registration nodes read the field through
/// [`FerroXamlIlLanguage::CONTEXT_NAME_SCOPE_FIELD_NAME`].
pub struct FerroXamlIlContextNameScopeField {
    pub field_type: Rc<dyn IXamlType>,
    pub name: &'static str,
    pub visibility: XamlVisibility,
    pub is_static: bool,
    pub service_provider_get_service_method: Rc<dyn IXamlMethod>,
}

/// The eager parent stack provider implemented by the runtime context.
///
/// # What a back end has to do (upstream IL)
///
/// 1. Make the context type implement `interface_type`
///    (`IFerroXamlIlEagerParentStackProvider`).
/// 2. Implement `DirectParentsStack` (getter `direct_parents_stack_getter`) as a private,
///    explicit interface implementation that loads the context's parent list field
///    (`ParentsStack`, which upstream requires to exist) and casts it to the getter's return
///    type (`IReadOnlyList<object>`).
/// 3. Implement `ParentProvider` (getter `parent_provider_getter`) as a private, explicit
///    interface implementation that loads the context's parent service provider field, loads
///    the type token of `parent_stack_provider` (`typeof(IFerroXamlIlParentStackProvider)`),
///    calls `service_provider_get_service_method` (`IServiceProvider.GetService(Type)`), passes
///    the result to the static `as_eager_parent_stack_provider_method`
///    (`XamlIlRuntimeHelpers.AsEagerParentStackProvider`) and returns what that returns.
/// 4. Define the two properties on the context type with those getters and no setters.
pub struct FerroXamlIlContextEagerParentStackProvider {
    pub interface_type: Rc<dyn IXamlType>,
    pub direct_parents_stack_getter: Rc<dyn IXamlMethod>,
    pub parent_provider_getter: Rc<dyn IXamlMethod>,
    pub parent_stack_provider: Rc<dyn IXamlType>,
    pub service_provider_get_service_method: Rc<dyn IXamlMethod>,
    pub as_eager_parent_stack_provider_method: Rc<dyn IXamlMethod>,
}

struct AttributeResolver {
    type_converter_attribute: Rc<dyn IXamlType>,
    converters: Vec<(Rc<dyn IXamlType>, Rc<dyn IXamlType>)>,
    ferro_list: Rc<dyn IXamlType>,
    ferro_list_converter: Rc<dyn IXamlType>,
}

impl AttributeResolver {
    fn new(
        type_system: &Rc<dyn IXamlTypeSystem>,
        mappings: &XamlLanguageTypeMappings,
    ) -> XamlResult<Self> {
        let get = |name: &str| type_system.get_type(name);
        let type_converter_attribute = mappings
            .type_converter_attributes
            .first()
            .cloned()
            .ok_or_else(|| XamlError::invalid_operation("Sequence contains no elements"))?;

        let well_known_types = type_system.well_known_types();
        let mut converters: Vec<(Rc<dyn IXamlType>, Rc<dyn IXamlType>)> = Vec::new();
        let mut add_type = |type_: Rc<dyn IXamlType>, conv: Rc<dyn IXamlType>| {
            converters.push((type_, conv));
        };

        add_type(
            get("FerroUI.Media.IImage")?,
            get("FerroUI.Markup.Xaml.Converters.BitmapTypeConverter")?,
        );
        add_type(
            get("FerroUI.Media.Imaging.Bitmap")?,
            get("FerroUI.Markup.Xaml.Converters.BitmapTypeConverter")?,
        );
        add_type(
            get("FerroUI.Media.IImageBrushSource")?,
            get("FerroUI.Markup.Xaml.Converters.BitmapTypeConverter")?,
        );
        // Not in the upstream list: the contract of bitmap-valued properties
        // (`NativeMenuItem.Icon`) takes text through the same converter. A type system that
        // does not know the contract (a reduced test model) simply has no mapping for it.
        if let Some(bitmap_contract) = type_system.find_type("FerroUI.Media.Imaging.IBitmap") {
            add_type(bitmap_contract, get("FerroUI.Markup.Xaml.Converters.BitmapTypeConverter")?);
        }
        add_type(
            well_known_types
                .i_list_of_t
                .make_generic_type(&[get("FerroUI.Point")?])?,
            get("FerroUI.Markup.Xaml.Converters.PointsListTypeConverter")?,
        );
        add_type(
            get("FerroUI.Controls.WindowIcon")?,
            get("FerroUI.Markup.Xaml.Converters.IconTypeConverter")?,
        );
        add_type(
            get("System.Globalization.CultureInfo")?,
            get("System.ComponentModel.CultureInfoConverter")?,
        );
        add_type(
            well_known_types.uri.clone(),
            get("FerroUI.Markup.Xaml.Converters.FerroUriTypeConverter")?,
        );
        add_type(
            get("System.TimeSpan")?,
            get("FerroUI.Markup.Xaml.Converters.TimeSpanTypeConverter")?,
        );
        add_type(
            get("FerroUI.Media.FontFamily")?,
            get("FerroUI.Markup.Xaml.Converters.FontFamilyTypeConverter")?,
        );
        let ferro_list = get("FerroUI.Collections.FerroList`1")?;
        let ferro_list_converter = get("FerroUI.Collections.FerroListConverter`1")?;

        Ok(Self {
            type_converter_attribute,
            converters,
            ferro_list,
            ferro_list_converter,
        })
    }

    fn lookup_converter(&self, type_: &dyn IXamlType) -> Option<Rc<dyn IXamlType>> {
        for (key, value) in &self.converters {
            if key.equals(type_) {
                return Some(value.clone());
            }
        }
        if type_
            .generic_type_definition()
            .is_some_and(|d| d.equals(&*self.ferro_list))
        {
            let argument = type_.generic_arguments().into_iter().next()?;
            // Upstream lets a failure to construct the converter type propagate; with a
            // one-parameter converter definition and one argument it cannot fail.
            return self.ferro_list_converter.make_generic_type(&[argument]).ok();
        }
        None
    }
}

struct ConstructedAttribute {
    type_: Rc<dyn IXamlType>,
    parameters: Vec<XamlValue>,
    properties: HashMap<String, XamlValue>,
}

impl ConstructedAttribute {
    fn new(
        type_: Rc<dyn IXamlType>,
        parameters: Option<Vec<XamlValue>>,
        properties: Option<HashMap<String, XamlValue>>,
    ) -> Self {
        Self {
            type_,
            parameters: parameters.unwrap_or_default(),
            properties: properties.unwrap_or_default(),
        }
    }
}

impl IXamlCustomAttribute for ConstructedAttribute {
    fn type_(&self) -> Rc<dyn IXamlType> {
        self.type_.clone()
    }
    fn parameters(&self) -> Vec<XamlValue> {
        self.parameters.clone()
    }
    fn properties(&self) -> HashMap<String, XamlValue> {
        self.properties.clone()
    }
    fn equals(&self, _other: &dyn IXamlCustomAttribute) -> bool {
        false
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IXamlCustomAttributeResolver for AttributeResolver {
    fn get_custom_attribute_for_type(
        &self,
        type_: &dyn IXamlType,
        attribute_type: &dyn IXamlType,
    ) -> Option<Rc<dyn IXamlCustomAttribute>> {
        if attribute_type.equals(&*self.type_converter_attribute) {
            if let Some(conv) = self.lookup_converter(type_) {
                return Some(Rc::new(ConstructedAttribute::new(
                    self.type_converter_attribute.clone(),
                    Some(vec![XamlValue::Type(conv)]),
                    None,
                )));
            }
        }

        None
    }

    fn get_custom_attribute_for_property(
        &self,
        _property: &dyn IXamlProperty,
        _attribute_type: &dyn IXamlType,
    ) -> Option<Rc<dyn IXamlCustomAttribute>> {
        None
    }
}

#[cfg(test)]
#[path = "ferro_xaml_il_language_tests.rs"]
mod tests;

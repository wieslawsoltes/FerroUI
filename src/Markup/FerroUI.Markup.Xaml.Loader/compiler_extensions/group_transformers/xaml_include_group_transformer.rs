//! Port of `CompilerExtensions/GroupTransformers/XamlIncludeGroupTransformer.cs`.
//!
//! # What the transformer needs from a back end
//!
//! Upstream reaches the compiled form of the included document through IL type builders
//! (`XamlDocumentTypeBuilderProvider`). Here the same information arrives as type system
//! handles, so the transformer stays independent of the back end:
//!
//! * a document of the current compilation exposes
//!   [`IXamlDocumentResource::build_method`](crate::compiler_extensions::IXamlDocumentResource::build_method)
//!   (the static `T Build(IServiceProvider)` method the back end generates for a document that
//!   can be instantiated on its own; provided through
//!   [`IXamlDocumentTypeBuilderProvider`](crate::compiler_extensions::IXamlDocumentTypeBuilderProvider))
//!   or [`IXamlDocumentResource::class_type`](crate::compiler_extensions::IXamlDocumentResource::class_type)
//!   (the `x:Class` type);
//! * a document compiled into another assembly is found through the type system: that
//!   assembly's type [`COMPILED_RESOURCES_TYPE_NAME`] has one public static method per compiled
//!   document named `Build:` followed by the rooted path of the document (`Build:/Dir/File.xaml`),
//!   or the assembly has a type whose full name is the document path with dots
//!   (`Assembly.Dir.File`). A type system over compiled metadata synthesizes that type from the
//!   list of compiled document URIs the other crate exports.

use std::rc::Rc;

use ferroui_base::utilities::{Uri, UriKind};
use xamlx::ast::{
    IXamlAstManipulationNode, IXamlAstNode, IXamlAstNodeNeedsParentStack, IXamlAstTypeReference,
    IXamlAstValueNode, IXamlLineInfo, XamlAstClrTypeReference, XamlAstExtensions,
    XamlAstNewClrObjectNode, XamlAstNode, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlConstantNode, XamlManipulationGroupNode, XamlObjectInitializationNode,
    XamlPropertyAssignmentNode, XamlStaticOrTargetedReturnMethodCallNode,
    XamlValueWithManipulationNode,
};
use xamlx::diagnostics::XamlDiagnosticSeverity;
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::transformers::{ConstructableObjectTransformer, NewObjectTransformer};
use xamlx::transform::IXamlAstTransformer;
use xamlx::type_system::{IXamlMethod, IXamlType, XamlValue};
use xamlx::{xaml_ast_node_members, xaml_line_info_impl};

use super::xaml_merge_resource_group_transformer::invariant_ignore_case_equals;
use super::{AstGroupTransformationContext, IXamlAstGroupTransformer};
use crate::compiler_extensions::transformers::{
    FerroXamlIlConstructorServiceProviderTransformer, FerroXamlIlWellKnownTypesExtensions,
};
use crate::compiler_extensions::{FerroXamlDiagnosticCodes, XamlAstNewClrObjectHelper};

/// The full name of the type that lists the compiled documents of an assembly.
pub const COMPILED_RESOURCES_TYPE_NAME: &str = "CompiledFerroXaml.!FerroResources";

/// Lets an include that cannot be linked at compile time stay a run-time include
/// (a configuration extra: `configuration.get_or_create_extra::<XamlRuntimeIncludeFallback>()`).
///
/// Upstream every assembly with markup is compiled, so an include of a document that is
/// neither part of the current compilation nor listed by the compiled-resources type of its
/// assembly is an error. In the port an assembly may have no compiled markup at all (its
/// documents are assets loaded by the run-time loader). The run-time compiler therefore
/// installs a predicate here that says whether the document with an absolute URI can be
/// loaded at run time (the asset exists); for such a document
///
/// * [`FerroXamlIncludeTransformer`] leaves a `StyleInclude` / `ResourceInclude` as the
///   object with its `Source`, whose `Loaded` loads the document on first use;
/// * [`XamlMergeResourceGroupTransformer`](super::XamlMergeResourceGroupTransformer) leaves a
///   `MergeResourceInclude` in the merged dictionaries (the class is a `ResourceInclude`
///   and loads at run time as one).
///
/// The fallback applies only where upstream reports that the assembly or its
/// compiled-resources type is missing: a document that is missing from an assembly that
/// does have compiled markup stays an error, and so does everything else. Without a
/// predicate (the default, and what a build-time compilation uses) nothing changes.
#[derive(Default)]
pub struct XamlRuntimeIncludeFallback {
    can_load: std::cell::RefCell<Option<Rc<dyn Fn(&str) -> bool>>>,
}

impl XamlRuntimeIncludeFallback {
    /// Installs (or with `None` removes) the predicate: whether the document with the
    /// absolute URI can be loaded at run time.
    pub fn set(&self, can_load: Option<Rc<dyn Fn(&str) -> bool>>) {
        *self.can_load.borrow_mut() = can_load;
    }

    /// Whether the fallback is enabled.
    pub fn is_enabled(&self) -> bool {
        self.can_load.borrow().is_some()
    }

    /// Whether the include of `absolute_uri` is left to run time.
    pub fn can_load(&self, absolute_uri: &str) -> bool {
        let can_load = self.can_load.borrow().clone();
        can_load.is_some_and(|can_load| can_load(absolute_uri))
    }

    /// Whether the configuration of `context` has the fallback enabled.
    pub(super) fn is_enabled_for(context: &AstGroupTransformationContext) -> bool {
        context.configuration().get_extra::<XamlRuntimeIncludeFallback>().is_ok_and(|fallback| fallback.is_enabled())
    }

    /// The fallback of the configuration of `context`.
    pub(super) fn applies(context: &AstGroupTransformationContext, absolute_uri: &str) -> bool {
        context
            .configuration()
            .get_extra::<XamlRuntimeIncludeFallback>()
            .is_ok_and(|fallback| fallback.can_load(absolute_uri))
    }
}

/// Links `StyleInclude` and `ResourceInclude` elements whose `Source` is a compile-time
/// constant to the compiled form of the included document: the include object is replaced
/// with a call of the document's build method or with an instance of its class.
pub struct FerroXamlIncludeTransformer;

fn line_info(node: &Option<Rc<dyn IXamlAstNode>>) -> Option<&dyn IXamlLineInfo> {
    node.as_deref().map(|n| n as &dyn IXamlLineInfo)
}

/// `Path.GetFileNameWithoutExtension`.
fn get_file_name_without_extension(path: &str) -> &str {
    let file_name = match path.rfind('/') {
        Some(separator) => &path[separator + 1..],
        None => path,
    };
    match file_name.rfind('.') {
        Some(dot) => &file_name[..dot],
        None => file_name,
    }
}

/// `Uri.UnescapeDataString`: decodes percent-encoded sequences that form valid UTF-8 and
/// leaves everything else as it is.
fn unescape_data_string(s: &str) -> String {
    fn hex(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }

    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }

        // Collect a run of %XX escapes and decode it as UTF-8 where it is valid.
        let run_start = i;
        let mut decoded: Vec<u8> = Vec::new();
        while i + 2 < bytes.len() && bytes[i] == b'%' {
            match (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                (Some(high), Some(low)) => {
                    decoded.push(high * 16 + low);
                    i += 3;
                }
                _ => break,
            }
        }
        if decoded.is_empty() {
            out.push(bytes[i]);
            i += 1;
            continue;
        }

        let mut rest: &[u8] = &decoded;
        let mut consumed = run_start;
        while !rest.is_empty() {
            match std::str::from_utf8(rest) {
                Ok(valid) => {
                    out.extend_from_slice(valid.as_bytes());
                    rest = &[];
                }
                Err(e) => {
                    let valid_up_to = e.valid_up_to();
                    out.extend_from_slice(&rest[..valid_up_to]);
                    consumed += valid_up_to * 3;
                    // Keep the escape of the invalid byte as written.
                    out.extend_from_slice(&bytes[consumed..consumed + 3]);
                    consumed += 3;
                    rest = &rest[valid_up_to + 1..];
                }
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn uri_format_exception(e: impl std::fmt::Display) -> XamlError {
    XamlError::internal("UriFormatException", e.to_string())
}

impl IXamlAstGroupTransformer for FerroXamlIncludeTransformer {
    fn transform(
        &self,
        context: &AstGroupTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        // Filter object initialization nodes like:
        // > XamlValueWithManipulationNode
        // > > XamlAstNewClrObjectNode // StyleInclude or ResourceInclude, can be nested in another XamlValueWithManipulationNode
        // > > XamlObjectInitializationNode
        let Some(value_node) = node.as_value_with_manipulation_node() else {
            return Ok(node);
        };
        let Some(initialization_node) = value_node
            .manipulation()
            .and_then(|m| m.cast::<XamlObjectInitializationNode>())
        else {
            return Ok(node);
        };
        let Some(object_node) =
            XamlAstNewClrObjectHelper::unwrap_value::<XamlAstNewClrObjectNode>(value_node)
        else {
            return Ok(node);
        };
        let ferro_types = context.try_get_ferro_types()?;
        let object_type = object_node.type_.borrow().clone().get_clr_type()?;
        if !object_type.equals(&*ferro_types.style_include)
            && !object_type.equals(&*ferro_types.resource_include)
        {
            return Ok(node);
        }

        let node_type_name = object_type.name();
        let expected_loaded_type = object_type
            .get_all_properties()
            .into_iter()
            .find(|p| p.name() == "Loaded")
            .map(|p| p.property_type());
        let Some(expected_loaded_type) = expected_loaded_type else {
            return Err(XamlError::invalid_operation(format!(
                "\"{node_type_name}\".Loaded property is expected to be defined"
            )));
        };

        let mut additional_properties: Vec<Rc<dyn IXamlAstManipulationNode>> = Vec::new();

        let manipulation = initialization_node.manipulation();
        let source_property = match manipulation
            .cast::<XamlPropertyAssignmentNode>()
            .filter(|p| p.property.name() == "Source")
        {
            Some(source_property) => source_property,
            None => {
                let manipulation_group = manipulation.cast::<XamlManipulationGroupNode>();
                let source_property2 = manipulation_group.as_ref().and_then(|group| {
                    group
                        .children
                        .borrow()
                        .iter()
                        .filter_map(|c| c.cast::<XamlPropertyAssignmentNode>())
                        .find(|p| p.property.name() == "Source")
                });
                match (manipulation_group, source_property2) {
                    (Some(manipulation_group), Some(source_property2)) => {
                        // We need to copy some additional properties from ResourceInclude to ResourceDictionary except the Source one.
                        // If there is any missing properties, then XAML compiler will throw an error in the emitter code.
                        additional_properties = manipulation_group
                            .children
                            .borrow()
                            .iter()
                            .filter(|c| !c.same_node(&source_property2))
                            .cloned()
                            .collect();
                        source_property2
                    }
                    _ => {
                        context.report_transform_error_node(
                            &format!("Source property must be set on the \"{node_type_name}\" node."),
                            node.clone(),
                        )?;
                        return Ok(node);
                    }
                }
            }
        };

        let (asset_path_uri, source_uri_node) =
            Self::resolve_source_from_xaml_include(context, &node_type_name, &source_property, false)?;
        let Some(asset_path_uri) = asset_path_uri else {
            return Ok(node);
        };
        let source_uri_node: Rc<dyn IXamlAstNode> = source_uri_node.unwrap_or_else(|| node.clone());

        let asset_path = asset_path_uri.replace("ferres://", "");
        let assembly_name_separator = asset_path.find('/').ok_or_else(|| {
            XamlError::internal(
                "ArgumentOutOfRangeException",
                "Length cannot be less than zero. (Parameter 'length')",
            )
        })?;
        let assembly = asset_path[..assembly_name_separator].to_string();
        let dotted_path = asset_path.replace('/', ".");
        let full_type_name = get_file_name_without_extension(&dotted_path);

        // Search file in the current assembly among other XAML resources.
        let target_document = context
            .documents()
            .iter()
            .find(|d| invariant_ignore_case_equals(d.uri().as_deref(), &asset_path_uri))
            .cloned();
        if let Some(target_document) = target_document {
            if let Some(build_method) = target_document.build_method()? {
                return Self::from_method(
                    context,
                    build_method,
                    &source_uri_node,
                    &expected_loaded_type,
                    node,
                    &asset_path_uri,
                    &assembly,
                    additional_properties,
                );
            }

            if let Some(class_type) = target_document.class_type() {
                return Self::from_type(
                    context,
                    class_type,
                    &source_uri_node,
                    &expected_loaded_type,
                    node,
                    &asset_path_uri,
                    &assembly,
                    additional_properties,
                );
            }

            context.report_transform_error_node(
                &format!(
                    "Unable to resolve XAML resource \"{asset_path_uri}\" in the current assembly."
                ),
                source_uri_node,
            )?;
            return Ok(node);
        }

        // If resource wasn't found in the current assembly, search in the others.
        let Some(asset_assembly) = context.configuration().type_system.find_assembly(&assembly)
        else {
            if XamlRuntimeIncludeFallback::applies(context, &asset_path_uri) {
                return Ok(node);
            }
            if XamlRuntimeIncludeFallback::is_enabled_for(context) {
                return Self::missing_run_time_document(context, &asset_path_uri, &assembly, source_uri_node, node);
            }
            context.report_transform_error_node(
                &format!(
                    "Assembly \"{assembly}\" was not found from the \"{asset_path_uri}\" source."
                ),
                source_uri_node,
            )?;
            return Ok(node);
        };

        let Some(ava_res_type) = asset_assembly.find_type(COMPILED_RESOURCES_TYPE_NAME) else {
            // The assembly has no compiled markup: its documents are loaded at run time.
            if XamlRuntimeIncludeFallback::applies(context, &asset_path_uri) {
                return Ok(node);
            }
            if XamlRuntimeIncludeFallback::is_enabled_for(context) {
                return Self::missing_run_time_document(context, &asset_path_uri, &assembly, source_uri_node, node);
            }
            context.report_transform_error_node(
                &format!("Unable to resolve \"!FerroResources\" type on \"{assembly}\" assembly."),
                source_uri_node,
            )?;
            return Ok(node);
        };

        let relative_name = format!("Build:{}", &asset_path[assembly_name_separator..]);
        let build_method = ava_res_type.find_method(|m| m.name() == relative_name && m.is_public());
        if let Some(build_method) = build_method {
            return Self::from_method(
                context,
                build_method,
                &source_uri_node,
                &expected_loaded_type,
                node,
                &asset_path_uri,
                &assembly,
                additional_properties,
            );
        } else if let Some(type_) = asset_assembly.find_type(full_type_name) {
            return Self::from_type(
                context,
                type_,
                &source_uri_node,
                &expected_loaded_type,
                node,
                &asset_path_uri,
                &assembly,
                additional_properties,
            );
        }

        context.report_transform_error_node(
            &format!(
                "Unable to resolve XAML resource \"{asset_path_uri}\" in the \"{assembly}\" assembly. Make sure this file exists and is public."
            ),
            source_uri_node,
        )?;
        Ok(node)
    }
}

impl FerroXamlIncludeTransformer {
    /// The error for an include the run-time fallback is enabled for
    /// ([`XamlRuntimeIncludeFallback`]) whose document cannot be loaded at run time either:
    /// the message upstream has for a document that is missing from its assembly, which
    /// names the document.
    fn missing_run_time_document(
        context: &AstGroupTransformationContext,
        asset_path_uri: &str,
        assembly: &str,
        source_uri_node: Rc<dyn IXamlAstNode>,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        context.report_transform_error_node(
            &format!(
                "Unable to resolve XAML resource \"{asset_path_uri}\" in the \"{assembly}\" assembly. Make sure this file exists and is public."
            ),
            source_uri_node,
        )?;
        Ok(node)
    }

    fn from_type(
        context: &AstGroupTransformationContext,
        type_: Rc<dyn IXamlType>,
        li: &Rc<dyn IXamlAstNode>,
        expected_loaded_type: &Rc<dyn IXamlType>,
        fallback_node: Rc<dyn IXamlAstNode>,
        asset_path_uri: &str,
        assembly: &str,
        manipulation_nodes: Vec<Rc<dyn IXamlAstManipulationNode>>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if !expected_loaded_type.is_assignable_from(&*type_) {
            context.report_transform_error_node(
                &format!(
                    "Resource \"{asset_path_uri}\" is defined as \"{}\" type in the \"{assembly}\" assembly, but expected \"{}\".",
                    type_.to_type_string(),
                    expected_loaded_type.to_type_string()
                ),
                li.clone(),
            )?;
            return Ok(fallback_node);
        }

        let new_obj_node =
            XamlAstObjectNode::new(&**li, XamlAstClrTypeReference::new(&**li, type_, false));
        new_obj_node
            .children
            .borrow_mut()
            .extend(manipulation_nodes.iter().map(|m| m.as_node()));
        let new_obj_node: Rc<dyn IXamlAstNode> = new_obj_node;
        let new_obj_node =
            FerroXamlIlConstructorServiceProviderTransformer.transform(context, new_obj_node)?;
        let new_obj_node = ConstructableObjectTransformer.transform(context, new_obj_node)?;
        NewObjectTransformer.transform(context, new_obj_node)
    }

    fn from_method(
        context: &AstGroupTransformationContext,
        method: Rc<dyn IXamlMethod>,
        li: &Rc<dyn IXamlAstNode>,
        expected_loaded_type: &Rc<dyn IXamlType>,
        fallback_node: Rc<dyn IXamlAstNode>,
        asset_path_uri: &str,
        assembly: &str,
        manipulation_nodes: Vec<Rc<dyn IXamlAstManipulationNode>>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let return_type = method.return_type();
        if !expected_loaded_type.is_assignable_from(&*return_type) {
            context.report_transform_error_node(
                &format!(
                    "Resource \"{asset_path_uri}\" is defined as \"{}\" type in the \"{assembly}\" assembly, but expected \"{}\".",
                    return_type.to_type_string(),
                    expected_loaded_type.to_type_string()
                ),
                li.clone(),
            )?;
            return Ok(fallback_node);
        }

        let sp = context.configuration().type_mappings.service_provider()?;
        let arguments: Vec<Rc<dyn IXamlAstValueNode>> = vec![NewServiceProviderNode::new(sp, &**li)];
        Ok(XamlValueWithManipulationNode::new(
            &**li,
            XamlStaticOrTargetedReturnMethodCallNode::new(&**li, method, Some(arguments)),
            Some(XamlManipulationGroupNode::new(
                &**li,
                Some(manipulation_nodes),
            )),
        ))
    }

    /// `ResolveSourceFromXamlInclude(context, nodeTypeName, sourceProperty,
    /// strictSourceValueType)`: the absolute, unescaped URI of the included document and the
    /// node the source came from. The URI is `None` when the source cannot be resolved at
    /// compile time (a diagnostic has been reported: an error when
    /// `strict_source_value_type`, a warning otherwise).
    pub fn resolve_source_from_xaml_include(
        context: &AstGroupTransformationContext,
        node_type_name: &str,
        source_property: &Rc<XamlPropertyAssignmentNode>,
        strict_source_value_type: bool,
    ) -> XamlResult<(Option<String>, Option<Rc<dyn IXamlAstNode>>)> {
        let on_invalid_source = |node: &Option<Rc<dyn IXamlAstNode>>| -> XamlResult<()> {
            context.report_diagnostic_at(
                FerroXamlDiagnosticCodes::TRANSFORM_ERROR,
                if strict_source_value_type {
                    XamlDiagnosticSeverity::Error
                } else {
                    XamlDiagnosticSeverity::Warning
                },
                &format!(
                    "\"{node_type_name}.Source\" supports only \"ferres://\" absolute or relative uri. This {node_type_name} will be resolved in runtime instead."
                ),
                line_info(node),
                XamlDiagnosticSeverity::None,
            )
        };

        // We expect that FerroXamlIlLanguageParseIntrinsics has already parsed the Uri and created node like: `new Uri(assetPath, uriKind)`.
        let values = source_property.values.borrow().clone();
        if values.len() != 1 {
            on_invalid_source(&Some(source_property.as_node()))?;
            return Ok((None, None));
        }

        // `new Uri` can be wrapped in manipulation node if source info or another manipulation was applied.
        let source_uri_node_wrapped = values[0].clone();
        let source_uri_node = match source_uri_node_wrapped.cast::<XamlAstNewClrObjectNode>() {
            Some(new_obj) => Some(new_obj),
            None => source_uri_node_wrapped
                .as_value_with_manipulation_node()
                .and_then(XamlAstNewClrObjectHelper::unwrap_value::<XamlAstNewClrObjectNode>),
        };

        // Validate Uri type and constant arguments.
        let mut validated: Option<(Rc<XamlAstNewClrObjectNode>, String, i32)> = None;
        if let Some(source_uri_node) = &source_uri_node {
            let source_type = source_uri_node.type_.borrow().clone().get_clr_type()?;
            if source_type.equals(&*context.try_get_ferro_types()?.uri) {
                let arguments = source_uri_node.arguments.borrow();
                let original_asset_path = arguments
                    .first()
                    .and_then(|a| a.cast::<XamlConstantNode>())
                    .and_then(|c| match &c.constant {
                        XamlValue::String(s) => Some(s.clone()),
                        _ => None,
                    });
                let uri_kind = arguments
                    .get(1)
                    .and_then(|a| a.cast::<XamlConstantNode>())
                    .and_then(|c| match &c.constant {
                        XamlValue::Int32(kind) => Some(*kind),
                        _ => None,
                    });
                if let (Some(original_asset_path), Some(uri_kind)) = (original_asset_path, uri_kind)
                {
                    validated = Some((source_uri_node.clone(), original_asset_path, uri_kind));
                }
            }
        }
        let Some((source_uri_node, original_asset_path, uri_kind)) = validated else {
            // Source value can be set with markup extension instead of the Uri object node, we don't support it here yet.
            let any_prop_value: Option<Rc<dyn IXamlAstNode>> =
                values.first().map(|value| value.as_node());
            on_invalid_source(&any_prop_value)?;
            return Ok((None, any_prop_value));
        };

        let uri_kind = match uri_kind {
            1 => UriKind::Absolute,
            2 => UriKind::Relative,
            _ => UriKind::RelativeOrAbsolute,
        };
        let mut uri_path = Uri::new(&original_asset_path, uri_kind).map_err(uri_format_exception)?;
        if !uri_path.is_absolute_uri() {
            let base_url = context
                .current_document()
                .and_then(|d| d.uri())
                .ok_or_else(|| XamlError::invalid_operation("CurrentDocument URI is null."))?;
            uri_path = Uri::combine(
                &Uri::new(&base_url, UriKind::Absolute).map_err(uri_format_exception)?,
                &uri_path,
            );
        } else if !uri_path.scheme().eq_ignore_ascii_case("ferres") {
            let source_uri_node: Option<Rc<dyn IXamlAstNode>> = Some(source_uri_node);
            on_invalid_source(&source_uri_node)?;
            return Ok((None, source_uri_node));
        }

        Ok((
            Some(unescape_data_string(uri_path.absolute_uri())),
            Some(source_uri_node),
        ))
    }
}

/// A value node of the service provider type that stands for "a new root service provider".
/// It needs the parent stack.
///
/// # What a back end has to do (upstream IL)
///
/// `Emit` (leaves one value): load the local that holds the runtime context of the method
/// being generated and call the static runtime helper
/// `XamlIlRuntimeHelpers.CreateRootServiceProviderV3(IServiceProvider)` (looked up by name on
/// `types.runtime_helpers`) with it; the helper's result, typed as the node's type, is the
/// service provider passed to the included document's build method.
pub struct NewServiceProviderNode {
    base: XamlAstNode,
    type_: Rc<dyn IXamlAstTypeReference>,
}

impl NewServiceProviderNode {
    /// The name of the runtime helper method the upstream IL calls.
    pub const CREATE_ROOT_SERVICE_PROVIDER_METHOD_NAME: &'static str = "CreateRootServiceProviderV3";

    pub fn new(type_: Rc<dyn IXamlType>, line_info: &dyn IXamlLineInfo) -> Rc<Self> {
        Rc::new(Self {
            base: XamlAstNode::new(line_info),
            type_: XamlAstClrTypeReference::new(line_info, type_, false),
        })
    }
}

xaml_line_info_impl!(NewServiceProviderNode, base);

impl IXamlAstNode for NewServiceProviderNode {
    xaml_ast_node_members!("NewServiceProviderNode", value, needs_parent_stack);
}

impl IXamlAstValueNode for NewServiceProviderNode {
    fn type_(&self) -> Rc<dyn IXamlAstTypeReference> {
        self.type_.clone()
    }
}

impl IXamlAstNodeNeedsParentStack for NewServiceProviderNode {
    fn needs_parent_stack(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod helper_tests {
    use super::{get_file_name_without_extension, unescape_data_string};

    #[test]
    fn unescapes_percent_encoded_text() {
        assert_eq!(unescape_data_string("a%20b"), "a b");
        assert_eq!(unescape_data_string("%C3%A9t%C3%A9"), "\u{e9}t\u{e9}");
        assert_eq!(unescape_data_string("100%"), "100%");
        assert_eq!(unescape_data_string("%zz%41"), "%zzA");
        assert_eq!(unescape_data_string("%FF%41"), "%FFA");
        assert_eq!(unescape_data_string("plain"), "plain");
    }

    #[test]
    fn file_name_without_extension() {
        assert_eq!(get_file_name_without_extension("Asm.Dir.File.xaml"), "Asm.Dir.File");
        assert_eq!(get_file_name_without_extension("File"), "File");
    }
}

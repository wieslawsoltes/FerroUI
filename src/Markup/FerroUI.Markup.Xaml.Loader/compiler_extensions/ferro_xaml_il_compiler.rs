//! Port of `CompilerExtensions/FerroXamlIlCompiler.cs`: the assembly of the transformation
//! pipeline of the framework's XAML language.
//!
//! Upstream the class derives from the IL compiler of XamlX and also drives IL generation.
//! The port keeps everything that is independent of a back end: the transformer pipeline with
//! exactly the upstream order, the group transformers, document parsing with root type
//! substitution, and the configuration properties. Code generation is the business of the two
//! back ends (the run-time interpreter and the Rust source emitter); the contract they have to
//! fulfil is described below.
//!
//! # Back-end contract
//!
//! ## Input
//!
//! After [`FerroXamlIlCompiler::transform`] (and, for a set of documents,
//! [`FerroXamlIlCompiler::transform_group`]) the root of a document is a
//! `XamlValueWithManipulationNode`; [`FerroXamlIlCompiler::get_transformed_root`] splits it
//! into the value that creates the root object, the manipulation that populates it and the
//! root type.
//!
//! ## `Populate` (upstream `CompilePopulate`, method name [`FerroXamlIlCompiler::POPULATE_NAME`])
//!
//! `void Populate(IServiceProvider sp, T target)`:
//!
//! 1. create the runtime context for the document from `sp` (parent service provider), the
//!    target as root object, the document's base URI and its XML namespace table; the context
//!    carries the framework members described by
//!    [`FerroXamlIlContextDefinition`](super::FerroXamlIlContextDefinition) (name scope field,
//!    eager parent stack provider);
//! 2. push the target on the parent stack and evaluate the root manipulation with the target
//!    as the object being initialized;
//! 3. pop the parent stack.
//!
//! ## `Build` (upstream `CompileBuild`, method name [`FerroXamlIlCompiler::BUILD_NAME`])
//!
//! `T Build(IServiceProvider sp)`, generated only for documents that can be instantiated on
//! their own: evaluate the root value node (an object creation; when it has exactly one
//! argument of the service provider type the runtime context has to be created first, and the
//! argument types must equal the constructor parameter types, otherwise the document cannot
//! have a build method), then call `Populate(sp, instance)` and return the instance.
//!
//! ## Node emitters
//!
//! A back end has to handle every node type that can occur in a transformed document:
//!
//! * the XamlX nodes handled upstream by the default IL emitters (`NewObjectEmitter`,
//!   `TextNodeEmitter`, `MethodCallEmitter`, `PropertyAssignmentEmitter`,
//!   `PropertyValueManipulationEmitter`, `ManipulationGroupEmitter`,
//!   `ValueWithManipulationsEmitter`, `MarkupExtensionEmitter`,
//!   `ObjectInitializationNodeEmitter`) and the self-emitting XamlX nodes (intrinsics, locals,
//!   deferred content, runtime casts, ...);
//! * the two emitters this class registers upstream:
//!   `FerroNameScopeRegistrationXamlIlNodeEmitter` for
//!   [`FerroNameScopeRegistrationXamlIlNode`](super::transformers::FerroNameScopeRegistrationXamlIlNode)
//!   and `FerroXamlIlRootObjectScope.Emitter` for
//!   [`HandleRootObjectScopeNode`](super::transformers::HandleRootObjectScopeNode); their
//!   semantics are documented on the node types;
//! * every node, property setter and pseudo method listed in the table of
//!   [`crate::compiler_extensions`].
//!
//! ## Documents that refer to each other
//!
//! Upstream hands the group transformers a `XamlDocumentTypeBuilderProvider` (IL type and
//! method builders) per document. Here a back end implements
//! [`IXamlDocumentTypeBuilderProvider`](super::IXamlDocumentTypeBuilderProvider): handles to
//! the populate method and, when the document is instantiable, the build method it will
//! generate for the document (one output module per document). The include group transformer
//! puts those handles into method call nodes.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use xamlx::ast::{
    IXamlAstManipulationNode, IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode,
    XamlAstClrTypeReference, XamlAstExtensions, XamlAstNamePropertyReference,
    XamlAstNodeExtensions, XamlAstObjectNode, XamlAstTextNode, XamlAstXamlPropertyValueNode,
    XamlAstXmlDirective, XamlAstXmlTypeReference, XamlDocument, XamlRootObjectNode,
};
use xamlx::compiler::XamlImperativeCompiler;
use xamlx::emit::{IXamlEmitResult, XamlLanguageEmitMappings};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::parsers::XDocumentXamlParser;
use xamlx::transform::transformers::{
    ContentConvertTransformer, ConvertPropertyValuesToAssignmentsTransformer,
    DeferredContentTransformer, NewObjectTransformer, PropertyReferenceResolver,
    TypeReferenceResolver,
};
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer, TransformerConfiguration};
use xamlx::type_system::IXamlType;
use xamlx::XamlNamespaces;

use super::group_transformers::{
    AstGroupTransformationContext, FerroXamlIncludeTransformer, IXamlAstGroupTransformer,
    XamlMergeResourceGroupTransformer,
};
use super::transformers::*;
use super::IXamlDocumentResource;

/// A transformer shared between the pipeline and the compiler, which changes its settings.
struct SharedTransformer<T: IXamlAstTransformer>(Rc<T>);

impl<T: IXamlAstTransformer> IXamlAstTransformer for SharedTransformer<T> {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        self.0.transform(context, node)
    }

    fn transformer_name(&self) -> String {
        self.0.transformer_name()
    }
}

/// The emit result type the default transformer list of XamlX is instantiated with; no code
/// is generated through it.
struct NoEmitResult;

impl IXamlEmitResult for NoEmitResult {
    fn return_type(&self) -> Option<Rc<dyn IXamlType>> {
        None
    }
    fn valid(&self) -> bool {
        true
    }
}

/// `typeof(T).Name`, as [`IXamlAstTransformer::transformer_name`] reports it.
fn transformer_type_name<T>() -> String {
    let full = std::any::type_name::<T>();
    full.rsplit("::").next().unwrap_or(full).to_string()
}

/// `Transformers.FindIndex(x => x is T)`.
fn find_index<T>(transformers: &[Box<dyn IXamlAstTransformer>]) -> Option<usize> {
    let name = transformer_type_name::<T>();
    transformers.iter().position(|x| x.transformer_name() == name)
}

fn missing_transformer<T>() -> XamlError {
    XamlError::internal(
        "ArgumentOutOfRangeException",
        format!(
            "The transformer {} is not part of the pipeline",
            transformer_type_name::<T>()
        ),
    )
}

/// `InsertAfter<T>(params IXamlAstTransformer[] t)`. As upstream, the transformers go to the
/// front of the list when there is no `T` (`FindIndex` returns -1).
fn insert_after<T>(
    transformers: &mut Vec<Box<dyn IXamlAstTransformer>>,
    t: Vec<Box<dyn IXamlAstTransformer>>,
) {
    let index = find_index::<T>(transformers).map_or(0, |index| index + 1);
    transformers.splice(index..index, t);
}

/// `InsertBefore<T>(params IXamlAstTransformer[] t)`. Fails when there is no `T` (upstream:
/// `InsertRange(-1, ...)` throws).
fn insert_before<T>(
    transformers: &mut Vec<Box<dyn IXamlAstTransformer>>,
    t: Vec<Box<dyn IXamlAstTransformer>>,
) -> XamlResult<()> {
    let index = find_index::<T>(transformers).ok_or_else(missing_transformer::<T>)?;
    transformers.splice(index..index, t);
    Ok(())
}

/// The root of a transformed document, split into what a back end generates code from.
pub struct FerroXamlIlTransformedRoot {
    /// The value node that creates the root object (for `Build`).
    pub root_instance: Rc<dyn IXamlAstValueNode>,
    /// The manipulation that populates a root object (for `Populate`).
    pub manipulation: Option<Rc<dyn IXamlAstManipulationNode>>,
    /// The type of the root object.
    pub root_type: Rc<dyn IXamlType>,
}

/// `FerroXamlIlCompiler`: the transformation pipeline of the framework's XAML language. See the
/// module documentation for what is left to a back end.
pub struct FerroXamlIlCompiler {
    configuration: Rc<TransformerConfiguration>,
    /// `Transformers`: the per-document pipeline, in order.
    pub transformers: Vec<Box<dyn IXamlAstTransformer>>,
    /// `SimplificationTransformers`: run once over the whole tree after `transformers`.
    pub simplification_transformers: Vec<Box<dyn IXamlAstTransformer>>,
    /// `GroupTransformers`: run over all documents of a compilation, in order.
    pub group_transformers: Vec<Box<dyn IXamlAstGroupTransformer>>,
    design_transformer: Rc<FerroXamlIlDesignPropertiesTransformer>,
    binding_transformer: Rc<FerroBindingExtensionTransformer>,
    add_source_info_transformer: Rc<FerroXamlIlAddSourceInfoTransformer>,
    resource_transformer: Rc<FerroXamlResourceTransformer>,
}

impl FerroXamlIlCompiler {
    /// The name of the generated method that populates an existing root object.
    pub const POPULATE_NAME: &'static str = "__FerroXamlIlPopulate";
    /// The name of the generated method that creates and populates the root object.
    pub const BUILD_NAME: &'static str = "__FerroXamlIlBuild";
    /// The name of the generated type that carries the XML namespace table of a document.
    pub const NAMESPACE_INFO_NAME: &'static str = "__FerroXamlIlNsInfo";

    /// Assembles the pipeline over `configuration` (the transformer configuration built from
    /// `FerroXamlIlLanguage::configure`).
    ///
    /// The list starts with the default transformers of XamlX (`XamlCompiler` followed by the
    /// three of `XamlImperativeCompiler`); the framework's transformers are then inserted with
    /// the same calls, in the same order, as upstream.
    pub fn new(configuration: Rc<TransformerConfiguration>) -> XamlResult<Self> {
        let defaults = XamlImperativeCompiler::<(), NoEmitResult>::new(
            configuration.clone(),
            Rc::new(XamlLanguageEmitMappings::default()),
            true,
        );
        let mut transformers = defaults.base.transformers;
        let simplification_transformers = defaults.base.simplification_transformers;

        let design_transformer = Rc::new(FerroXamlIlDesignPropertiesTransformer::new());
        let binding_transformer = Rc::new(FerroBindingExtensionTransformer::new());
        let add_source_info_transformer = Rc::new(FerroXamlIlAddSourceInfoTransformer::new());
        let resource_transformer = Rc::new(FerroXamlResourceTransformer::new());

        // Before everything else

        transformers.insert(0, Box::new(XNameTransformer));
        transformers.insert(1, Box::new(IgnoredDirectivesTransformer));
        transformers.insert(2, Box::new(SharedTransformer(design_transformer.clone())));
        transformers.insert(3, Box::new(SharedTransformer(binding_transformer.clone())));

        // Targeted
        insert_before::<PropertyReferenceResolver>(
            &mut transformers,
            vec![
                Box::new(FerroXamlIlResolveClassesPropertiesTransformer),
                Box::new(FerroXamlIlTransformInstanceAttachedProperties),
                Box::new(FerroXamlIlTransformSyntheticCompiledBindingMembers),
            ],
        )?;
        insert_after::<PropertyReferenceResolver>(
            &mut transformers,
            vec![
                Box::new(FerroXamlIlFerroPropertyResolver),
                Box::new(FerroXamlIlReorderClassesPropertiesTransformer),
                Box::new(FerroXamlIlClassesTransformer),
            ],
        );

        insert_before::<ContentConvertTransformer>(
            &mut transformers,
            vec![
                Box::new(FerroXamlIlControlThemeTransformer),
                Box::new(FerroXamlIlSelectorTransformer),
                Box::new(FerroXamlIlQueryTransformer),
                Box::new(FerroXamlIlDuplicateSettersChecker),
                Box::new(FerroXamlIlControlTemplateTargetTypeMetadataTransformer),
                Box::new(FerroXamlIlBindingPathParser),
                Box::new(FerroXamlIlSetterTargetTypeMetadataTransformer),
                Box::new(FerroXamlIlSetterTransformer),
                Box::new(FerroXamlIlStyleValidatorTransformer),
                Box::new(FerroXamlIlConstructorServiceProviderTransformer),
                Box::new(FerroXamlIlTransitionsTypeMetadataTransformer),
                Box::new(FerroXamlIlResolveByNameMarkupExtensionReplacer),
                Box::new(FerroXamlIlThemeVariantProviderTransformer),
                Box::new(FerroXamlIlDataTemplateWarningsTransformer),
            ],
        )?;
        insert_before::<ConvertPropertyValuesToAssignmentsTransformer>(
            &mut transformers,
            vec![Box::new(FerroXamlIlOptionMarkupExtensionTransformer)],
        )?;

        insert_after::<TypeReferenceResolver>(
            &mut transformers,
            vec![Box::new(XDataTypeTransformer)],
        );

        // After everything else
        insert_before::<NewObjectTransformer>(
            &mut transformers,
            vec![
                Box::new(AddNameScopeRegistration),
                Box::new(FerroXamlIlControlTemplatePartsChecker),
                Box::new(FerroXamlIlDataContextTypeTransformer),
                Box::new(FerroXamlIlBindingPathTransformer),
                Box::new(FerroXamlIlCompiledBindingsMetadataRemover),
            ],
        )?;
        {
            // InsertBeforeMany(new [] { typeof(DeferredContentTransformer),
            //     typeof(FerroXamlIlCompiledBindingsMetadataRemover) }, _resourceTransformer = ...)
            let deferred_content = find_index::<DeferredContentTransformer>(&transformers)
                .ok_or_else(missing_transformer::<DeferredContentTransformer>)?;
            let metadata_remover =
                find_index::<FerroXamlIlCompiledBindingsMetadataRemover>(&transformers)
                    .ok_or_else(missing_transformer::<FerroXamlIlCompiledBindingsMetadataRemover>)?;
            let index = deferred_content.min(metadata_remover);
            transformers.insert(
                index,
                Box::new(SharedTransformer(resource_transformer.clone())),
            );
        }
        insert_before::<FerroXamlIlTransformInstanceAttachedProperties>(
            &mut transformers,
            vec![Box::new(FerroXamlIlTransformRoutedEvent)],
        )?;

        transformers.push(Box::new(FerroXamlIlControlTemplatePriorityTransformer));
        transformers.push(Box::new(FerroXamlIlMetadataRemover));
        transformers.push(Box::new(
            FerroXamlIlEnsureResourceDictionaryCapacityTransformer::new(),
        ));
        transformers.push(Box::new(FerroXamlIlRootObjectScope));
        transformers.push(Box::new(SharedTransformer(
            add_source_info_transformer.clone(),
        )));

        // Upstream: Emitters.Add(new FerroNameScopeRegistrationXamlIlNodeEmitter());
        //           Emitters.Add(new FerroXamlIlRootObjectScope.Emitter());
        // See "Node emitters" in the module documentation.

        let group_transformers: Vec<Box<dyn IXamlAstGroupTransformer>> = vec![
            Box::new(XamlMergeResourceGroupTransformer),
            Box::new(FerroXamlIncludeTransformer),
        ];

        Ok(Self {
            configuration,
            transformers,
            simplification_transformers,
            group_transformers,
            design_transformer,
            binding_transformer,
            add_source_info_transformer,
            resource_transformer,
        })
    }

    /// The transformer configuration the pipeline runs over.
    pub fn configuration(&self) -> &Rc<TransformerConfiguration> {
        &self.configuration
    }

    /// The names of the transformers of the per-document pipeline, in order.
    pub fn transformer_names(&self) -> Vec<String> {
        self.transformers
            .iter()
            .map(|t| t.transformer_name())
            .collect()
    }

    pub fn create_source_info(&self) -> bool {
        self.add_source_info_transformer.create_source_info()
            || self.resource_transformer.create_source_info()
    }

    pub fn set_create_source_info(&self, value: bool) {
        self.resource_transformer.set_create_source_info(value);
        self.add_source_info_transformer.set_create_source_info(value);
    }

    pub fn is_design_mode(&self) -> bool {
        self.design_transformer.is_design_mode()
    }

    pub fn set_is_design_mode(&self, value: bool) {
        self.design_transformer.set_is_design_mode(value);
    }

    pub fn default_compile_bindings(&self) -> bool {
        self.binding_transformer.compile_bindings_by_default.get()
    }

    pub fn set_default_compile_bindings(&self, value: bool) {
        self.binding_transformer.compile_bindings_by_default.set(value);
    }

    /// `CreateTransformationContext(doc)`.
    pub fn create_transformation_context(&self, doc: &XamlDocument) -> AstTransformationContext {
        AstTransformationContext::new(self.configuration.clone(), Some(doc))
    }

    /// `Transform(doc)`: runs the per-document pipeline and replaces the root of the document
    /// with the transformed tree.
    pub fn transform(&self, doc: &mut XamlDocument) -> XamlResult<()> {
        let ctx = self.create_transformation_context(doc);

        let mut root: Rc<dyn IXamlAstNode> = doc.root()?;
        let root_object = root.cast::<XamlAstObjectNode>().ok_or_else(|| {
            XamlError::invalid_cast(format!(
                "Unable to cast object of type '{}' to type 'XamlAstObjectNode'.",
                root.type_name()
            ))
        })?;
        ctx.set_root_object(XamlRootObjectNode::new(&root_object));
        for transformer in &self.transformers {
            ctx.visit_children(&*ctx.root_object()?, &**transformer)?;
            root = ctx.visit(&root, &**transformer)?;
        }

        for simplifier in &self.simplification_transformers {
            root = ctx.visit(&root, &**simplifier)?;
        }

        doc.set_root(root);
        Ok(())
    }

    /// `TransformGroup(documents)`: runs the group transformers over every (already
    /// transformed) document of a compilation.
    pub fn transform_group(&self, documents: &[Rc<dyn IXamlDocumentResource>]) -> XamlResult<()> {
        let ctx = AstGroupTransformationContext::new(documents.to_vec(), self.configuration.clone());
        for transformer in &self.group_transformers {
            for doc in documents {
                let root = doc.xaml_document().borrow().root()?;
                ctx.set_current_document(Some(doc.clone()));
                let root_object = root.cast::<dyn IXamlAstValueNode>().ok_or_else(|| {
                    XamlError::invalid_cast(format!(
                        "Unable to cast object of type '{}' to type 'IXamlAstValueNode'.",
                        root.type_name()
                    ))
                })?;
                ctx.set_root_object(root_object.clone());
                ctx.visit_children_group(&*root_object, &**transformer)?;
                let root = ctx.visit_group(&root, &**transformer)?;
                doc.xaml_document().borrow_mut().set_root(root);
            }
        }
        Ok(())
    }

    /// `Parse(xaml, overrideRootType)`: parses the document (design-time `d:` markup is kept)
    /// and fixes the root type: the `x:Class` type when there is one, the type of the root
    /// element otherwise, or `override_root_type` when it derives from that type.
    pub fn parse(
        &self,
        xaml: &str,
        override_root_type: Option<Rc<dyn IXamlType>>,
    ) -> XamlResult<XamlDocument> {
        let mut compatibility_mappings = HashMap::new();
        compatibility_mappings.insert(
            XamlNamespaces::BLEND2008.to_string(),
            XamlNamespaces::BLEND2008.to_string(),
        );
        let parsed = XDocumentXamlParser::parse(xaml, Some(&compatibility_mappings))?;

        let root = parsed.root()?;
        let root_object = root.cast::<XamlAstObjectNode>().ok_or_else(|| {
            XamlError::invalid_cast(format!(
                "Unable to cast object of type '{}' to type 'XamlAstObjectNode'.",
                root.type_name()
            ))
        })?;

        let class_directive = root_object
            .children
            .borrow()
            .iter()
            .filter_map(|c| c.cast::<XamlAstXmlDirective>())
            .find(|x| {
                x.namespace.borrow().as_deref() == Some(XamlNamespaces::XAML2006)
                    && *x.name.borrow() == "Class"
            });

        let mut root_type: Rc<XamlAstClrTypeReference> = match class_directive {
            Some(class_directive) => {
                let class_name = class_directive
                    .values
                    .borrow()
                    .first()
                    .and_then(|v| v.cast::<XamlAstTextNode>())
                    .map(|text| text.text())
                    .ok_or_else(|| {
                        XamlError::invalid_cast(
                            "Unable to cast the value of the x:Class directive to type 'XamlAstTextNode'.",
                        )
                    })?;
                XamlAstClrTypeReference::new(
                    &*class_directive,
                    self.configuration.type_system.get_type(&class_name)?,
                    false,
                )
            }
            None => {
                let xml_type = root_object
                    .type_
                    .borrow()
                    .clone()
                    .cast::<XamlAstXmlTypeReference>()
                    .ok_or_else(|| {
                        XamlError::invalid_cast(
                            "Unable to cast the root type to type 'XamlAstXmlTypeReference'.",
                        )
                    })?;
                TypeReferenceResolver::resolve_type_reference(
                    &self.create_transformation_context(&parsed),
                    &xml_type,
                )?
            }
        };

        if let Some(override_root_type) = override_root_type {
            if !root_type.type_.is_assignable_from(&*override_root_type) {
                return Err(XamlError::load_exception(
                    format!(
                        "Unable to substitute {} with {}",
                        root_type.type_.get_fqn(),
                        override_root_type.get_fqn()
                    ),
                    Some(&*root_object),
                ));
            }
            root_type = XamlAstClrTypeReference::new(&*root_object, override_root_type, false);
        }

        self.override_root_type(&parsed, root_type)?;
        Ok(parsed)
    }

    /// `OverrideRootType(doc, newType)`: replaces the type of the root object and the
    /// references to it in the property references of the root's attributes.
    pub fn override_root_type(
        &self,
        doc: &XamlDocument,
        new_type: Rc<dyn IXamlAstTypeReference>,
    ) -> XamlResult<()> {
        let root = doc.root()?;
        let root = root.cast::<XamlAstObjectNode>().ok_or_else(|| {
            XamlError::invalid_cast(format!(
                "Unable to cast object of type '{}' to type 'XamlAstObjectNode'.",
                root.type_name()
            ))
        })?;
        let old_type = root.type_.borrow().clone();
        if old_type.equals(&*new_type) {
            return Ok(());
        }

        *root.type_.borrow_mut() = new_type.clone();
        let children = root.children.borrow().clone();
        for child in children
            .iter()
            .filter_map(|c| c.cast::<XamlAstXamlPropertyValueNode>())
        {
            if let Some(prop) = child.property().cast::<XamlAstNamePropertyReference>() {
                let declaring_type = prop.declaring_type.borrow().clone();
                if declaring_type.equals(&*old_type) {
                    *prop.declaring_type.borrow_mut() = new_type.clone();
                }
                let target_type = prop.target_type.borrow().clone();
                if target_type.equals(&*old_type) {
                    *prop.target_type.borrow_mut() = new_type.clone();
                }
            }
        }
        Ok(())
    }

    /// The root of a transformed document as a back end consumes it: the node that creates
    /// the root object, the manipulation that populates it and the root type.
    pub fn get_transformed_root(&self, doc: &XamlDocument) -> XamlResult<FerroXamlIlTransformedRoot> {
        let root = doc.root()?;
        let root_grp = root.as_value_with_manipulation_node().ok_or_else(|| {
            XamlError::invalid_cast(format!(
                "Unable to cast object of type '{}' to type 'XamlValueWithManipulationNode'.",
                root.type_name()
            ))
        })?;
        Ok(FerroXamlIlTransformedRoot {
            root_instance: root_grp.value(),
            manipulation: root_grp.manipulation(),
            root_type: IXamlAstValueNode::type_(root_grp).get_clr_type()?,
        })
    }
}

/// Keeps the compiler usable where a `RefCell` of documents is at hand: transforms the
/// document in place.
impl FerroXamlIlCompiler {
    /// [`FerroXamlIlCompiler::transform`] for a shared document (as the group transformers
    /// see it through `IXamlDocumentResource::xaml_document`).
    pub fn transform_shared(&self, doc: &RefCell<XamlDocument>) -> XamlResult<()> {
        // The document is not borrowed while the transformers run.
        let mut working = std::mem::take(&mut *doc.borrow_mut());
        let result = self.transform(&mut working);
        *doc.borrow_mut() = working;
        result
    }
}

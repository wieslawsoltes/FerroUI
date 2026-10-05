//! Port of `Compiler/XamlImperativeCompiler.cs`.
//!
//! The backend-independent orchestration is ported; the abstract members a backend has to
//! implement (`CompilePopulate`, `CompileBuild`, `CreateRuntimeContext`) form the
//! [`IXamlImperativeCompilerBackend`] trait.

use std::rc::Rc;

use crate::ast::{IXamlAstManipulationNode, IXamlAstValueNode, XamlAstExtensions, XamlDocument};
use crate::emit::{IXamlEmitResult, XamlLanguageEmitMappings, XamlRuntimeContext};
use crate::exceptions::{XamlError, XamlResult};
use crate::transform::transformers::{
    DeferredContentTransformer, NewObjectTransformer, TopDownInitializationTransformer,
};
use crate::transform::TransformerConfiguration;
use crate::type_system::{
    IFileSource, IXamlMethod, IXamlMethodBuilder, IXamlType, IXamlTypeBuilder, XamlVisibility,
};

use super::XamlCompiler;

pub trait IXamlImperativeCompilerBackend<
    TBackendEmitter: 'static,
    TEmitResult: IXamlEmitResult + 'static,
>
{
    fn compile_populate(
        &self,
        file_source: Option<Rc<dyn IFileSource>>,
        manipulation: Rc<dyn IXamlAstManipulationNode>,
        declaring_type: Rc<dyn IXamlTypeBuilder<TBackendEmitter>>,
        code_gen: TBackendEmitter,
        context: Rc<XamlRuntimeContext<TBackendEmitter, TEmitResult>>,
    ) -> XamlResult<()>;

    fn compile_build(
        &self,
        file_source: Option<Rc<dyn IFileSource>>,
        root_instance: Rc<dyn IXamlAstValueNode>,
        declaring_type: Rc<dyn IXamlTypeBuilder<TBackendEmitter>>,
        code_gen: TBackendEmitter,
        context: Rc<XamlRuntimeContext<TBackendEmitter, TEmitResult>>,
        compiled_populate: Rc<dyn IXamlMethod>,
    ) -> XamlResult<()>;

    fn create_runtime_context(
        &self,
        doc: &XamlDocument,
        context_type: Rc<dyn IXamlType>,
        namespace_info_builder: Option<Rc<dyn IXamlTypeBuilder<TBackendEmitter>>>,
        base_uri: Option<&str>,
        root_type: Rc<dyn IXamlType>,
    ) -> XamlResult<Rc<XamlRuntimeContext<TBackendEmitter, TEmitResult>>>;
}

pub struct XamlImperativeCompiler<TBackendEmitter: 'static, TEmitResult: IXamlEmitResult + 'static>
{
    pub base: XamlCompiler<TBackendEmitter, TEmitResult>,
}

fn root_group(
    doc: &XamlDocument,
) -> XamlResult<(
    Rc<dyn IXamlAstValueNode>,
    Option<Rc<dyn IXamlAstManipulationNode>>,
    Rc<dyn IXamlType>,
)> {
    let root = doc.root()?;
    let root_grp = root.as_value_with_manipulation_node().ok_or_else(|| {
        XamlError::invalid_cast(format!(
            "Unable to cast object of type '{}' to type 'XamlValueWithManipulationNode'.",
            root.type_name()
        ))
    })?;
    let root_type = crate::ast::IXamlAstValueNode::type_(root_grp).get_clr_type()?;
    Ok((root_grp.value(), root_grp.manipulation(), root_type))
}

impl<TBackendEmitter: 'static, TEmitResult: IXamlEmitResult + 'static>
    XamlImperativeCompiler<TBackendEmitter, TEmitResult>
{
    pub fn new(
        configuration: Rc<TransformerConfiguration>,
        emit_mappings: Rc<XamlLanguageEmitMappings<TBackendEmitter, TEmitResult>>,
        fill_with_defaults: bool,
    ) -> Self {
        let mut base = XamlCompiler::new(configuration, emit_mappings, fill_with_defaults);
        if fill_with_defaults {
            base.transformers.push(Box::new(NewObjectTransformer));
            base.transformers.push(Box::new(DeferredContentTransformer));
            base.transformers
                .push(Box::new(TopDownInitializationTransformer));
        }
        Self { base }
    }

    pub fn transform(&self, doc: &mut XamlDocument) -> XamlResult<()> {
        self.base.transform(doc)
    }

    /// `void Populate(IServiceProvider sp, T target);`
    pub fn define_populate_method(
        &self,
        type_builder: &Rc<dyn IXamlTypeBuilder<TBackendEmitter>>,
        doc: &XamlDocument,
        name: &str,
        visibility: XamlVisibility,
    ) -> XamlResult<Rc<dyn IXamlMethodBuilder<TBackendEmitter>>> {
        let (_, _, root_type) = root_group(doc)?;
        let configuration = &self.base.configuration;
        type_builder.define_method(
            &configuration.well_known_types().void,
            &[configuration.type_mappings.service_provider()?, root_type],
            name,
            visibility,
            true,
            false,
            None,
        )
    }

    /// `T Build(IServiceProvider sp);`
    pub fn define_build_method(
        &self,
        type_builder: &Rc<dyn IXamlTypeBuilder<TBackendEmitter>>,
        doc: &XamlDocument,
        name: &str,
        visibility: XamlVisibility,
    ) -> XamlResult<Rc<dyn IXamlMethodBuilder<TBackendEmitter>>> {
        let (_, _, root_type) = root_group(doc)?;
        let configuration = &self.base.configuration;
        type_builder.define_method(
            &root_type,
            &[configuration.type_mappings.service_provider()?],
            name,
            visibility,
            true,
            false,
            None,
        )
    }

    /// `Compile(doc, typeBuilder, contextType, populateMethodName, createMethodName, namespaceInfoClassName, baseUri, fileSource)`.
    #[allow(clippy::too_many_arguments)]
    pub fn compile(
        &self,
        backend: &dyn IXamlImperativeCompilerBackend<TBackendEmitter, TEmitResult>,
        doc: &XamlDocument,
        type_builder: &Rc<dyn IXamlTypeBuilder<TBackendEmitter>>,
        context_type: Rc<dyn IXamlType>,
        populate_method_name: &str,
        create_method_name: Option<&str>,
        namespace_info_class_name: &str,
        base_uri: Option<&str>,
        file_source: Option<Rc<dyn IFileSource>>,
    ) -> XamlResult<()> {
        let populate_method = self.define_populate_method(
            type_builder,
            doc,
            populate_method_name,
            XamlVisibility::Public,
        )?;
        let build_method = match create_method_name {
            Some(name) => {
                Some(self.define_build_method(type_builder, doc, name, XamlVisibility::Public)?)
            }
            None => None,
        };
        let configuration = &self.base.configuration;
        let namespace_info_builder = match &configuration.type_mappings.xml_namespace_info_provider
        {
            Some(_) => Some(type_builder.define_sub_type(
                &configuration.well_known_types().object,
                namespace_info_class_name,
                XamlVisibility::Private,
            )?),
            None => None,
        };
        self.compile_with_methods(
            backend,
            doc,
            context_type,
            populate_method,
            type_builder.clone(),
            build_method,
            Some(type_builder.clone()),
            namespace_info_builder,
            base_uri,
            file_source,
        )
    }

    /// `Compile(doc, contextType, populateMethod, populateDeclaringType, buildMethod, buildDeclaringType, namespaceInfoBuilder, baseUri, fileSource)`.
    #[allow(clippy::too_many_arguments)]
    pub fn compile_with_methods(
        &self,
        backend: &dyn IXamlImperativeCompilerBackend<TBackendEmitter, TEmitResult>,
        doc: &XamlDocument,
        context_type: Rc<dyn IXamlType>,
        populate_method: Rc<dyn IXamlMethodBuilder<TBackendEmitter>>,
        populate_declaring_type: Rc<dyn IXamlTypeBuilder<TBackendEmitter>>,
        build_method: Option<Rc<dyn IXamlMethodBuilder<TBackendEmitter>>>,
        build_declaring_type: Option<Rc<dyn IXamlTypeBuilder<TBackendEmitter>>>,
        namespace_info_builder: Option<Rc<dyn IXamlTypeBuilder<TBackendEmitter>>>,
        base_uri: Option<&str>,
        file_source: Option<Rc<dyn IFileSource>>,
    ) -> XamlResult<()> {
        let (root_value, root_manipulation, root_type) = root_group(doc)?;
        let context = backend.create_runtime_context(
            doc,
            context_type,
            namespace_info_builder.clone(),
            base_uri,
            root_type,
        )?;

        let root_manipulation = root_manipulation.ok_or_else(|| {
            XamlError::internal(
                "NullReferenceException",
                "The root node doesn't have a manipulation",
            )
        })?;
        backend.compile_populate(
            file_source.clone(),
            root_manipulation,
            populate_declaring_type,
            populate_method.generator(),
            context.clone(),
        )?;

        if let Some(build_method) = build_method {
            let build_declaring_type = build_declaring_type.ok_or_else(|| {
                XamlError::internal("ArgumentNullException", "buildDeclaringType")
            })?;

            backend.compile_build(
                file_source,
                root_value,
                build_declaring_type,
                build_method.generator(),
                context,
                populate_method,
            )?;
        }

        if let Some(namespace_info_builder) = namespace_info_builder {
            namespace_info_builder.create_type()?;
        }
        Ok(())
    }
}

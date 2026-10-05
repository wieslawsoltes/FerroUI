//! Port of `Compiler/XamlCompiler.cs`.

use std::rc::Rc;

use crate::ast::{
    IXamlAstNode, XamlAstNodeExtensions, XamlAstObjectNode, XamlDocument, XamlRootObjectNode,
};
use crate::emit::{
    IXamlEmitResult, IXamlEmitter, XamlEmitContext, XamlLanguageEmitMappings, XamlRuntimeContext,
};
use crate::exceptions::{XamlError, XamlResult};
use crate::transform::transformers::*;
use crate::transform::{AstTransformationContext, IXamlAstTransformer, TransformerConfiguration};
use crate::type_system::{IFileSource, IXamlTypeBuilder};

/// The state and the backend-independent behavior of the upstream abstract `XamlCompiler` class.
/// The abstract member (`InitCodeGen`) lives in [`IXamlCompilerCodeGen`], which a backend
/// compiler implements.
pub struct XamlCompiler<TBackendEmitter: 'static, TEmitResult: IXamlEmitResult + 'static> {
    pub configuration: Rc<TransformerConfiguration>,
    pub emit_mappings: Rc<XamlLanguageEmitMappings<TBackendEmitter, TEmitResult>>,
    pub transformers: Vec<Box<dyn IXamlAstTransformer>>,
    pub simplification_transformers: Vec<Box<dyn IXamlAstTransformer>>,
    pub emitters: Vec<Rc<dyn IXamlEmitter<TBackendEmitter, TEmitResult>>>,
}

impl<TBackendEmitter: 'static, TEmitResult: IXamlEmitResult + 'static>
    XamlCompiler<TBackendEmitter, TEmitResult>
{
    pub fn new(
        configuration: Rc<TransformerConfiguration>,
        emit_mappings: Rc<XamlLanguageEmitMappings<TBackendEmitter, TEmitResult>>,
        fill_with_defaults: bool,
    ) -> Self {
        let mut rv = Self {
            configuration,
            emit_mappings,
            transformers: Vec::new(),
            simplification_transformers: Vec::new(),
            emitters: Vec::new(),
        };
        if fill_with_defaults {
            rv.transformers = vec![
                Box::new(KnownDirectivesTransformer),
                Box::new(XamlIntrinsicsTransformer),
                Box::new(XArgumentsTransformer),
                Box::new(TypeReferenceResolver),
                Box::new(MarkupExtensionTransformer),
                Box::new(TextNodeMerger),
                Box::new(PropertyReferenceResolver),
                Box::new(ContentConvertTransformer),
                // This should come before actual content property processing
                Box::new(RemoveWhitespaceBetweenPropertyValuesTransformer),
                Box::new(ResolveContentPropertyTransformer),
                Box::new(ResolvePropertyValueAddersTransformer),
                Box::new(ApplyWhitespaceNormalization),
                // Should happen before we split on clr and xaml assignments
                Box::new(ObsoleteWarningsTransformer),
                Box::new(StaticIntrinsicsPostProcessTransformer),
                Box::new(ConvertPropertyValuesToAssignmentsTransformer),
                Box::new(ConstructableObjectTransformer),
            ];
            rv.simplification_transformers = vec![Box::new(FlattenAstTransformer)];
        }
        rv
    }

    pub fn create_transformation_context(&self, doc: &XamlDocument) -> AstTransformationContext {
        AstTransformationContext::new(self.configuration.clone(), Some(doc))
    }

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
}

/// `protected abstract XamlEmitContext<..> InitCodeGen(...)`: the code generation entry point a
/// backend compiler has to provide.
pub trait IXamlCompilerCodeGen<TBackendEmitter: 'static, TEmitResult: IXamlEmitResult + 'static> {
    fn init_code_gen(
        &self,
        file: Option<Rc<dyn IFileSource>>,
        declaring_type: Rc<dyn IXamlTypeBuilder<TBackendEmitter>>,
        code_gen: TBackendEmitter,
        context: Rc<XamlRuntimeContext<TBackendEmitter, TEmitResult>>,
        need_context_local: bool,
    ) -> XamlResult<Rc<dyn XamlEmitContext<TBackendEmitter, TEmitResult>>>;
}

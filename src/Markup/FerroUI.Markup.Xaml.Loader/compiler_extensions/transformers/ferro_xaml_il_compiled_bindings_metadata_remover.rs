//! Port of `CompilerExtensions/Transformers/FerroXamlIlCompiledBindingsMetadataRemover.cs`.

use std::rc::Rc;

use xamlx::ast::{IXamlAstNode, XamlAstNodeExtensions};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};

use super::{
    FerroXamlIlCompileBindingsNode, FerroXamlIlDataContextTypeMetadataNode,
    NestedScopeMetadataNode,
};

/// Unwraps the scope markers the compiled binding transformers need (nested name scopes, data
/// context types, `x:CompileBindings` scopes) once the binding paths are resolved.
pub struct FerroXamlIlCompiledBindingsMetadataRemover;

impl IXamlAstTransformer for FerroXamlIlCompiledBindingsMetadataRemover {
    fn transform(
        &self,
        _context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let mut node = node;
        loop {
            if let Some(nested_scope) = node.cast::<NestedScopeMetadataNode>() {
                node = nested_scope.value();
            } else if let Some(data_context_type) =
                node.cast::<FerroXamlIlDataContextTypeMetadataNode>()
            {
                node = data_context_type.value();
            } else if let Some(compile_bindings) = node.cast::<FerroXamlIlCompileBindingsNode>() {
                node = compile_bindings.value();
            } else {
                return Ok(node);
            }
        }
    }
}

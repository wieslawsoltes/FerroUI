//! Port of `CompilerExtensions/Transformers/FerroXamlIlCompiledBindingsMetadataRemover.cs`.

use std::rc::Rc;

use xamlx::ast::{IXamlAstNode, XamlAstConstructableObjectNode, XamlAstNodeExtensions, XamlAstObjectNode};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};

use super::{
    FerroXamlIlCompileBindingsNode, FerroXamlIlDataContextTypeMetadataNode,
    NestedScopeMetadataNode,
};

/// Unwraps the scope markers the compiled binding transformers need (nested name scopes, data
/// context types, `x:CompileBindings` scopes) once the binding paths are resolved, and (not
/// upstream) takes out the `x:DefaultBindMode` directives that gave `{x:Bind}` its default mode
/// (docs/porting/x-bind.md, step X5).
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
                if let Some(object) = node.cast::<XamlAstConstructableObjectNode>() {
                    super::x_bind_directives::remove_default_bind_mode(&object.children);
                } else if let Some(object) = node.cast::<XamlAstObjectNode>() {
                    super::x_bind_directives::remove_default_bind_mode(&object.children);
                }
                return Ok(node);
            }
        }
    }
}

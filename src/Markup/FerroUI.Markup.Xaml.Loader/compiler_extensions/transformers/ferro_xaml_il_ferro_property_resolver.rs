//! Port of `CompilerExtensions/Transformers/FerroXamlIlFerroPropertyResolver.cs`.

use std::rc::Rc;

use xamlx::ast::{IXamlAstNode, XamlAstClrProperty, XamlAstNodeExtensions};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};

use super::FerroXamlIlWellKnownTypesExtensions;
use crate::compiler_extensions::XamlIlFerroProperty;

/// Turns a CLR property whose declaring type has a `<Name>Property` field into a
/// [`XamlIlFerroProperty`] (a CLR property that also knows its registered property and gets the
/// binding-aware setters).
pub struct FerroXamlIlFerroPropertyResolver;

impl IXamlAstTransformer for FerroXamlIlFerroPropertyResolver {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(prop) = node.cast::<XamlAstClrProperty>() {
            let n = format!("{}Property", prop.name());
            let field = prop
                .declaring_type()
                .fields()
                .into_iter()
                .find(|f| f.name() == n);
            if let Some(field) = field {
                return Ok(XamlIlFerroProperty::new(
                    &prop,
                    field,
                    &context.try_get_ferro_types()?,
                )?);
            }
        }
        Ok(node)
    }
}

//! Port of `CompilerExtensions/Transformers/FerroXamlIlThemeVariantProviderTransformer.cs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, XamlAstClrProperty, XamlAstExtensions, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstPropertyReferenceExtensions, XamlAstXamlPropertyValueNode, XamlAstXmlDirective,
};
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::XamlNamespaces;

use super::FerroXamlIlWellKnownTypesExtensions;

/// Copies the `x:Key` of a theme variant provider (a resource dictionary) that is an entry of
/// a theme dictionaries collection (`IDictionary<ThemeVariant, IThemeVariantProvider>`) to
/// the provider's `Key` property, so that the dictionary knows which variant it belongs to.
pub struct FerroXamlIlThemeVariantProviderTransformer;

impl IXamlAstTransformer for FerroXamlIlThemeVariantProviderTransformer {
    fn transform(
        &self,
        context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        let av_types = context.try_get_ferro_types()?;
        let type_ = &av_types.i_theme_variant_provider;
        let Some(on) = node.cast::<XamlAstObjectNode>() else {
            return Ok(node);
        };
        let on_type = on.type_.borrow().clone();
        if !type_.is_assignable_from(&*on_type.get_clr_type()?) {
            return Ok(node);
        }

        let key_directive = on
            .children
            .borrow()
            .iter()
            .find(|n| {
                n.cast::<XamlAstXmlDirective>().is_some_and(|d| {
                    d.namespace.borrow().as_deref() == Some(XamlNamespaces::XAML2006)
                        && *d.name.borrow() == "Key"
                })
            })
            .and_then(|n| n.cast::<XamlAstXmlDirective>());
        let Some(key_directive) = key_directive else {
            return Ok(node);
        };

        let theme_dictionaries_coll = av_types.i_dictionary_t.make_generic_type(&[
            av_types.theme_variant.clone(),
            av_types.i_theme_variant_provider.clone(),
        ])?;
        let Some(property_value_node) = context
            .first_parent_node()
            .and_then(|parent| parent.cast::<XamlAstXamlPropertyValueNode>())
        else {
            return Ok(node);
        };
        let Some(getter) = property_value_node.property().get_clr_property()?.getter() else {
            return Ok(node);
        };
        if !theme_dictionaries_coll.is_assignable_from(&*getter.return_type()) {
            return Ok(node);
        }

        let key_prop = type_
            .properties()
            .into_iter()
            .find(|p| p.name() == "Key")
            .ok_or_else(|| {
                XamlError::invalid_operation("Sequence contains no matching element")
            })?;
        let key_values = key_directive.values.borrow().clone();
        let key_assignment = XamlAstXamlPropertyValueNode::with_values(
            &*key_directive,
            XamlAstClrProperty::from_property(&*key_directive, &key_prop, context.configuration())?,
            key_values,
            true,
        );
        on.children.borrow_mut().push(key_assignment);

        Ok(node)
    }
}

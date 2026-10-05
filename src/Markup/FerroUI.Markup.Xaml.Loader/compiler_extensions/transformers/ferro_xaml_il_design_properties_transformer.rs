//! Port of `CompilerExtensions/Transformers/FerroXamlIlDesignPropertiesTransformer.cs`.

use std::cell::Cell;
use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, XamlAstNamePropertyReference, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstXamlPropertyValueNode, XamlAstXmlDirective, XamlAstXmlTypeReference,
};
use xamlx::exceptions::XamlResult;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::XamlNamespaces;

use super::FERRO_XML_NAMESPACE;

/// Handles the design-time directives (`d:DataContext`, `d:DesignWidth`, `d:DesignHeight`,
/// `d:PreviewWith`) and the `Design.*` attached properties: outside of design mode they are
/// removed from the AST, in design mode the directives become assignments of the matching
/// attached property of the `Design` class.
#[derive(Default)]
pub struct FerroXamlIlDesignPropertiesTransformer {
    is_design_mode: Cell<bool>,
}

impl FerroXamlIlDesignPropertiesTransformer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_design_mode(&self) -> bool {
        self.is_design_mode.get()
    }

    pub fn set_is_design_mode(&self, value: bool) {
        self.is_design_mode.set(value);
    }

    /// `DesignDirectives.TryGetValue`.
    fn design_directive(name: &str) -> Option<&'static str> {
        match name {
            "DataContext" => Some("DataContext"),
            "DesignWidth" => Some("Width"),
            "DesignHeight" => Some("Height"),
            "PreviewWith" => Some("PreviewWith"),
            _ => None,
        }
    }
}

impl IXamlAstTransformer for FerroXamlIlDesignPropertiesTransformer {
    fn transform(
        &self,
        _context: &AstTransformationContext,
        node: Rc<dyn IXamlAstNode>,
    ) -> XamlResult<Rc<dyn IXamlAstNode>> {
        if let Some(on) = node.cast::<XamlAstObjectNode>() {
            let is_design_mode = self.is_design_mode.get();
            let mut c = 0;
            loop {
                let Some(ch) = on.children.borrow().get(c).cloned() else {
                    break;
                };

                let directive = ch.cast::<XamlAstXmlDirective>().and_then(|directive| {
                    if directive.namespace.borrow().as_deref() != Some(XamlNamespaces::BLEND2008) {
                        return None;
                    }
                    let map_to = Self::design_directive(&directive.name.borrow())?;
                    Some((directive, map_to))
                });

                if let Some((directive, map_to)) = directive {
                    if !is_design_mode {
                        // Just remove it from AST in non-design mode
                        on.children.borrow_mut().remove(c);
                    } else {
                        // Map to an actual property in `Design` class
                        let replacement = XamlAstXamlPropertyValueNode::with_values(
                            &*ch,
                            XamlAstNamePropertyReference::new(
                                &*ch,
                                XamlAstXmlTypeReference::new(
                                    &*ch,
                                    Some(FERRO_XML_NAMESPACE),
                                    "Design",
                                ),
                                map_to,
                                on.type_.borrow().clone(),
                            ),
                            directive.values.borrow().clone(),
                            true,
                        );
                        on.children.borrow_mut()[c] = replacement;
                        c += 1;
                    }
                    continue;
                }

                // Remove all "Design" attached properties in non-design mode
                let is_design_property = !is_design_mode
                    && ch
                        .cast::<XamlAstXamlPropertyValueNode>()
                        .and_then(|pv| pv.property().cast::<XamlAstNamePropertyReference>())
                        .and_then(|pref| {
                            let declaring_type = pref.declaring_type.borrow().clone();
                            declaring_type.cast::<XamlAstXmlTypeReference>()
                        })
                        .is_some_and(|dref| {
                            dref.xml_namespace.borrow().as_deref() == Some(FERRO_XML_NAMESPACE)
                                && *dref.name.borrow() == "Design"
                        });
                if is_design_property {
                    on.children.borrow_mut().remove(c);
                } else {
                    c += 1;
                }
            }
        }
        Ok(node)
    }
}

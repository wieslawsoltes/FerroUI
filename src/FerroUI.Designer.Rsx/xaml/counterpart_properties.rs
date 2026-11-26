use super::XamlTree;
use crate::tree::{escape_xml, inline_content, ValueChange};
use ferroui_designer_core::{Document, Edit, TextPatch};

impl XamlTree {
    pub(super) fn plan_properties(
        &self,
        document: &Document,
        changes: Vec<ValueChange<'_>>,
    ) -> Result<Vec<TextPatch>, String> {
        let mut attributes = Vec::new();
        let mut patches = Vec::new();
        for change in changes {
            let control = &self.origins[change.index];
            let Some(origin) = control.properties.get(change.name) else {
                if let Some(value) = change.value {
                    attributes.push(Edit::SetAttribute {
                        node: control.node,
                        name: change.name.into(),
                        value: value.into(),
                    });
                }
                continue;
            };
            if let Some(quote) = origin.quote {
                if let Some(value) = change.value {
                    patches.push(TextPatch {
                        span: origin.span,
                        replacement: escape_xml(value, Some(quote)),
                    });
                } else {
                    let attribute = document
                        .ast()
                        .node(control.node)
                        .and_then(|node| {
                            node.attributes
                                .iter()
                                .find(|attr| attr.value_span == origin.span)
                        })
                        .ok_or("The authored XAML property changed")?;
                    attributes.push(Edit::RemoveAttribute {
                        node: control.node,
                        name: attribute.name.clone(),
                    });
                }
                continue;
            }
            // Keep plain text in its original slot. Values with markup or
            // significant whitespace need the attribute form.
            let inline = change.value.and_then(inline_content);
            patches.push(TextPatch {
                span: origin.span,
                replacement: inline
                    .as_deref()
                    .map_or_else(String::new, |value| escape_xml(value, None)),
            });
            if let Some(value) = change.value.filter(|_| inline.is_none()) {
                attributes.push(Edit::SetAttribute {
                    node: control.node,
                    name: change.name.into(),
                    value: value.into(),
                });
            }
        }
        patches.extend(
            document
                .plan_attribute_patches(attributes)
                .map_err(|error| error.to_string())?,
        );
        Ok(patches)
    }
}

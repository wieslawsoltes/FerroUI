use crate::{
    blocks::blocks,
    registered::{Class, RsxCatalog},
};
use ferroui_designer_core::SourceSpan;
use ferroui_designer_language::{LineIndex, Range};
use ferroui_view_syntax::{Attribute, Node};
use serde::Serialize;
use std::collections::BTreeMap;
use syn::{Expr, ext::IdentExt, spanned::Spanned};

#[derive(Default, Serialize)]
pub struct RsxLanguageDocument {
    nodes: Vec<ControlSpan>,
    properties: BTreeMap<String, Vec<Property>>,
}

#[derive(Serialize)]
struct ControlSpan {
    class: String,
    key: String,
    name: Range,
    fields: Vec<FieldSpan>,
}

#[derive(Serialize)]
struct FieldSpan {
    name: String,
    range: Range,
    property: Option<Property>,
    literal: bool,
    shorthand: bool,
    attached: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Property {
    setter: String,
    xaml_name: String,
    value_type: String,
}

/// Analyzes only parsed RSX elements. Branches are readable even when the visual
/// projection omits them. Other Rust syntax belongs to rust-analyzer.
pub fn analyze_rsx(source: &str) -> RsxLanguageDocument {
    RsxCatalog::built_in().analyze(source)
}

impl RsxCatalog {
    pub fn analyze(&self, source: &str) -> RsxLanguageDocument {
        let Ok(blocks) = blocks(source) else {
            return RsxLanguageDocument::default();
        };
        let index = LineIndex::new(source);
        let mut result = RsxLanguageDocument::default();
        for block in blocks {
            if let Ok(nodes) = ferroui_view_syntax::parse(block.group.stream()) {
                result.collect(self, &index, &nodes, 0);
            }
        }
        result
    }
}

impl RsxLanguageDocument {
    fn collect(&mut self, catalog: &RsxCatalog, index: &LineIndex, nodes: &[Node], depth: usize) {
        if depth > crate::tree::MAX_DEPTH {
            return;
        }
        for node in nodes {
            if self.nodes.len() >= 2000 {
                return;
            }
            match node {
                Node::Element(node) => {
                    if let (Some(class), Some(name)) =
                        (catalog.rust(&node.path), range(index, node.path.span()))
                    {
                        let key = class.rust_name().to_owned();
                        self.properties
                            .entry(key.clone())
                            .or_insert_with(|| supported(&class));
                        self.nodes.push(ControlSpan {
                            fields: fields(index, &class, &node.attributes),
                            class: class.name().to_owned(),
                            key,
                            name,
                        });
                    }
                    self.collect(catalog, index, &node.children, depth + 1);
                }
                Node::For(_, _, body, _) => self.collect(catalog, index, body, depth + 1),
                Node::If(_, body, otherwise, _) => {
                    self.collect(catalog, index, body, depth + 1);
                    if let Some(otherwise) = otherwise {
                        self.collect(catalog, index, otherwise, depth + 1);
                    }
                }
                Node::Expr(..) | Node::Text(..) => (),
            }
        }
    }
}

fn fields(index: &LineIndex, class: &Class, attributes: &[Attribute]) -> Vec<FieldSpan> {
    let mut fields: Vec<_> = attributes
        .iter()
        .filter_map(|attr| field(index, class, attr))
        .collect();
    let mut count = BTreeMap::new();
    for property in fields.iter().filter_map(|field| field.property.as_ref()) {
        *count.entry(property.xaml_name.clone()).or_insert(0) += 1;
    }
    for field in &mut fields {
        if field
            .property
            .as_ref()
            .is_some_and(|property| count[&property.xaml_name] > 1)
        {
            field.literal = false;
        }
    }
    fields
}

fn field(index: &LineIndex, class: &Class, attr: &Attribute) -> Option<FieldSpan> {
    let (owner, name, value) = match attr {
        Attribute::Value { owner, name, value, .. } => (owner.as_ref(), name, Some(value.as_ref())),
        Attribute::Call { name, arguments } => (
            None,
            name,
            arguments.first().filter(|_| arguments.len() == 1),
        ),
    };
    let owner = owner.map(|owner| owner.unraw().to_string());
    let setter = name.unraw().to_string();
    let mapped = class.xaml_property(owner.as_deref(), &setter);
    let attached = owner.is_some() || mapped.as_ref().is_some_and(|name| name.contains('.'));
    let property = mapped.map(|name| property(class, name));
    if matches!(attr, Attribute::Call { .. }) && property.is_none() {
        return None;
    }
    let literal = property.as_ref().is_some_and(|property| {
        class
            .literal(&property.xaml_name)
            .and_then(|codec| codec.read_rust(value?))
            .is_some()
    });
    let name_range = range(index, name.span())?;
    Some(FieldSpan {
        name: setter,
        range: name_range,
        property,
        literal,
        shorthand: value.is_some_and(|value| {
            matches!(value, Expr::Path(_)) && range(index, value.span()) == Some(name_range)
        }),
        attached,
    })
}

fn supported(class: &Class) -> Vec<Property> {
    class
        .property_names()
        .into_iter()
        .map(|name| property(class, name))
        .collect()
}

fn property(class: &Class, name: String) -> Property {
    Property {
        setter: class.setter(&name),
        value_type: class.property_type(&name),
        xaml_name: name,
    }
}

fn range(index: &LineIndex, span: proc_macro2::Span) -> Option<Range> {
    let span = span.byte_range();
    index.range(SourceSpan::new(span.start, span.end)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_multiple_invocations_without_claiming_custom_or_rust_fields() {
        let source = r#"let text = "rsx! { Button { content: 4 } }";
let first = rsx! { Button { content: "🦀", widt } };
let second = rsx! { StackPanel { for item in rows { TextBlock { text: item.label } } other::Button { width: 22.0 } } };
let third = rsx! { TextBox { text: "draft" } };"#;
        let result = analyze_rsx(source);
        let classes: Vec<_> = result
            .nodes
            .iter()
            .map(|node| node.class.as_str())
            .collect();
        assert_eq!(classes, ["Button", "StackPanel", "TextBlock", "TextBox"]);
        assert!(result.nodes[0].fields[0].literal);
        assert!(result.nodes[0].fields[1].shorthand);
        assert_eq!(result.nodes[0].fields[1].name, "widt");
        assert!(!result.nodes[2].fields[0].literal);
        assert!(
            analyze_rsx("fn example() { /* rsx! { Button {} } */ }")
                .nodes
                .is_empty()
        );
    }
}

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
                            fields: node
                                .attributes
                                .iter()
                                .filter_map(|attr| field(index, &class, attr))
                                .collect(),
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

fn field(index: &LineIndex, class: &Class, attr: &Attribute) -> Option<FieldSpan> {
    let Attribute::Value {
        owner, name, value, ..
    } = attr
    else {
        return None;
    };
    let owner = owner.as_ref().map(|owner| owner.unraw().to_string());
    let setter = name.unraw().to_string();
    let property = class
        .xaml_property(owner.as_deref(), &setter)
        .map(|name| property(class, name));
    let literal = property.as_ref().is_some_and(|property| {
        class
            .literal(&property.xaml_name)
            .is_some_and(|codec| codec.read_rust(value).is_some())
    });
    let name_range = range(index, name.span())?;
    Some(FieldSpan {
        name: setter,
        range: name_range,
        property,
        literal,
        shorthand: matches!(value.as_ref(), Expr::Path(_))
            && range(index, value.span()) == Some(name_range),
        attached: owner.is_some(),
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

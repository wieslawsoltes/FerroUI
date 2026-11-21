//! Project-aware type contracts for source-backed style editors.
use super::{setter_properties::setter_target, style_selectors};
use crate::{binding, PropertyInfo, Schema, TypeInfo};
use ferroui_designer_core::{Document, NodeId};

impl Schema {
    /// Resolve an explicit XAML type in this node's inherited namespaces.
    pub fn authored_type(
        &self,
        document: &Document,
        node: NodeId,
        reference: &str,
    ) -> Option<&TypeInfo> {
        binding::type_reference(self, document, node, reference)
    }

    /// Resolve the common target of a style's branches, or the target of a ControlTheme.
    pub fn style_target(&self, document: &Document, definition: NodeId) -> Option<&TypeInfo> {
        setter_target(self, document, definition)
    }

    /// Resolve a writable setter member, including qualified attached properties.
    pub fn style_setter(
        &self,
        document: &Document,
        definition: NodeId,
        property: &str,
    ) -> Option<PropertyInfo> {
        let target = self.style_target(document, definition)?;
        let namespaces = document.ast().namespaces(definition);
        let namespace = if let Some((prefix, _)) = property.split_once(':') {
            Some(namespaces.get(prefix)?.as_str())
        } else if property.contains('.') {
            namespaces.get("").map(String::as_str)
        } else {
            Some(target.namespace.as_str())
        };
        self.property_in_context(&target.name, Some(&target.namespace), property, namespace)
            .filter(|info| !info.event && !info.read_only && info.style_settable != Some(false))
    }

    /// Validate a replacement or a new selector in the authored namespace and Cargo type scope.
    pub fn validate_style_selector(
        &self,
        document: &Document,
        node: NodeId,
        selector: &str,
        nested: bool,
    ) -> Result<(), String> {
        style_selectors::validate_selector(self, document, node, selector, nested)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_style_targets_filter_direct_members_and_resolve_nested_setters() {
        let mut schema = Schema::ferroui();
        let namespace = "using:PaletteReview";
        schema.register(TypeInfo {
            name: "Card".into(),
            namespace: namespace.into(),
            base: Some("Border".into()),
            properties: [
                (
                    "Accent".into(),
                    PropertyInfo {
                        name: "Accent".into(),
                        type_name: "f64".into(),
                        style_settable: Some(true),
                        ..Default::default()
                    },
                ),
                (
                    "DirectName".into(),
                    PropertyInfo {
                        name: "DirectName".into(),
                        type_name: "String".into(),
                        style_settable: Some(false),
                        ..Default::default()
                    },
                ),
            ]
            .into(),
            ..Default::default()
        });
        let source = Document::new(r#"<Styles xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:app="using:PaletteReview">
  <Style Selector="app|Card" x:Name="Outer"><Style Selector="^:pointerover" x:Name="Nested"/></Style>
</Styles>"#).unwrap();
        let nested = source.ast().find_name("Nested").unwrap();
        assert_eq!(schema.style_target(&source, nested).unwrap().name, "Card");
        assert_eq!(
            schema
                .style_setter(&source, nested, "Accent")
                .unwrap()
                .type_name,
            "f64"
        );
        assert!(schema.style_setter(&source, nested, "Width").is_some());
        assert!(schema.style_setter(&source, nested, "Grid.Row").is_some());
        assert!(schema.style_setter(&source, nested, "DirectName").is_none());
        assert!(schema
            .validate_style_selector(&source, nested, "^:pointerover", true)
            .is_ok());
        assert!(schema
            .validate_style_selector(&source, nested, "app|Missing", false)
            .is_err());
    }
}

//! Property editors for native and Cargo-registered style targets.
use crate::property_literals::{property_on_type, setter_target_type};
use crate::style_composition::{setter_metadata, setter_property};
use crate::{
    resolve_property_editor_metadata, PropertyEditorContext, PropertyEditorKind,
    PropertyEditorMetadata,
};
use ferroui_base::FerroProperty;
use ferroui_designer_core::{Document, MarkupValue, Node, NodeId};
use ferroui_designer_language::{PropertyInfo, Schema};
use serde_json::Value;
use std::rc::Rc;

pub(crate) struct StyleField {
    property: Option<&'static FerroProperty>,
    schema_property: Option<PropertyInfo>,
    context: PropertyEditorContext,
    metadata: PropertyEditorMetadata,
}

impl StyleField {
    pub(crate) fn resolve(
        schema: &Schema,
        document: &Document,
        node: NodeId,
        name: &str,
    ) -> Option<Self> {
        let native = setter_property(document, node, name);
        let schema_property = schema.style_setter(document, node, name);
        if let Some(native) = native {
            let (context, metadata) = setter_metadata(node, name, native);
            return Some(Self {
                property: Some(native),
                schema_property,
                context,
                metadata,
            });
        }
        // The native registry is authoritative for known direct and read-only properties.
        if setter_target_type(document, node)
            .and_then(|target| property_on_type(document, node, target, name))
            .is_some()
        {
            return None;
        }
        let info = schema_property.as_ref()?;
        let context = PropertyEditorContext {
            property: name.into(),
            targets: vec![node],
            value: None,
            mixed: false,
            runtime_type: Some(info.type_name.clone()),
            apply: Rc::new(|_| {}),
        };
        let mut metadata = resolve_property_editor_metadata(&context, None);
        if !info.values.is_empty() && matches!(metadata.kind, PropertyEditorKind::Text) {
            metadata.kind = PropertyEditorKind::Enumeration {
                values: info.values.clone(),
                flags: false,
            };
        }
        Some(Self {
            property: None,
            schema_property,
            context,
            metadata,
        })
    }

    pub(crate) fn describe(&self) -> Value {
        crate::style_report::setter_field(&self.metadata, self.type_name())
    }

    pub(crate) fn prepare<'a>(&self, text: &'a str) -> Result<&'a str, String> {
        if matches!(
            MarkupValue::parse(text).map_err(|error| error.to_string())?,
            MarkupValue::Literal(_)
        ) {
            return Ok(self
                .metadata
                .prepare_literal(&self.context, text)?
                .unwrap_or(text));
        }
        Ok(text)
    }

    pub(crate) fn accepts_resource(&self, schema: &Schema, node: &Node) -> bool {
        if let Some(native) = self.property {
            if let Some(compatible) = crate::editors::resource_is_compatible(native, node) {
                return compatible;
            }
        }
        let actual = schema.type_info(&node.name, node.namespace_uri.as_deref());
        let modeled = self
            .schema_property
            .as_ref()
            .and_then(|property| property.model_type.as_ref())
            .and_then(|ty| schema.type_info(&ty.name, Some(&ty.namespace)));
        let expected = type_leaf(self.type_name());
        if let Some(modeled) = modeled {
            return actual.is_some_and(|actual| schema.type_assignable_to(actual, modeled));
        }
        let target = schema.type_info(
            if expected == "IBrush" {
                "Brush"
            } else {
                expected
            },
            None,
        );
        actual
            .zip(target)
            .is_some_and(|(actual, target)| schema.type_assignable_to(actual, target))
            || matches!(self.metadata.kind, PropertyEditorKind::Brush)
                && schema
                    .type_info("Color", None)
                    .or_else(|| schema.type_info("Color", Some("using:FerroUI.Media")))
                    .zip(actual)
                    .is_some_and(|(color, actual)| schema.type_assignable_to(actual, color))
    }

    fn type_name(&self) -> &str {
        self.context.runtime_type.as_deref().unwrap_or("")
    }
}

fn type_leaf(mut name: &str) -> &str {
    name = name.trim();
    while let Some((wrapper, inner)) = name.split_once('<') {
        if !matches!(
            wrapper.rsplit("::").next(),
            Some("Option" | "Ref" | "Rc" | "Arc" | "Box")
        ) {
            break;
        }
        let Some(inner) = inner.strip_suffix('>') else {
            break;
        };
        name = inner.trim();
    }
    name.strip_prefix("dyn ")
        .unwrap_or(name)
        .rsplit("::")
        .next()
        .unwrap_or(name)
}

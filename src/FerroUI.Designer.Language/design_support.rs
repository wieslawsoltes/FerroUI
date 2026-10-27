//! Namespace-exact metadata for the designer's preview projection. This remains portable:
//! native control properties still come from the generated framework schema.
use crate::{ExtensionInfo, PropertyInfo, Schema};
use ferroui_designer_core::{Attribute, Document, MarkupValue, Node};

pub(crate) const DESIGN_NAMESPACE: &str = "http://schemas.microsoft.com/expression/blend/2008";
pub(crate) const COMPATIBILITY_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/markup-compatibility/2006";

pub(crate) fn runtime_property(name: &str) -> &str {
    match name {
        "DesignWidth" => "Width",
        "DesignHeight" => "Height",
        name => name,
    }
}

pub(crate) fn property(
    schema: &Schema,
    owner: &str,
    namespace: Option<&str>,
    name: &str,
) -> Option<PropertyInfo> {
    let namespace = namespace.map(|namespace| {
        if namespace == DESIGN_NAMESPACE {
            crate::schema::FRAMEWORK_NAMESPACE
        } else {
            namespace
        }
    });
    let name = runtime_property(name);
    if matches!(name, "PreviewWith" | "DesignStyle") {
        return schema.properties("Design", None).get(name).cloned();
    }
    let member_namespace = name.rsplit_once('.').map_or(namespace, |(declaring, _)| {
        if declaring == owner.rsplit(':').next().unwrap_or(owner) {
            namespace
        } else {
            Some(crate::schema::FRAMEWORK_NAMESPACE)
        }
    });
    schema.property_in_context(owner, namespace, name, member_namespace)
}

pub(crate) fn property_element(
    schema: &Schema,
    document: &Document,
    node: &Node,
) -> Option<PropertyInfo> {
    let parent = document.ast().node(node.parent?)?;
    let (owner, member) = node.local_name().split_once('.')?;
    let member = if owner == parent.local_name() || owner == "Design" {
        member
    } else {
        node.local_name()
    };
    property(
        schema,
        &parent.name,
        parent.namespace_uri.as_deref(),
        member,
    )
}

pub(crate) fn ignorable_property() -> PropertyInfo {
    PropertyInfo {
        name: "Ignorable".into(),
        type_name: "String".into(),
        documentation: "Space-separated XML namespace prefixes that a runtime may ignore. Design-time preview members use the Blend namespace.".into(),
        ..Default::default()
    }
}

pub(crate) fn extension(name: &str, namespace: Option<&str>) -> Option<&'static ExtensionInfo> {
    if namespace != Some(DESIGN_NAMESPACE)
        || !matches!(
            name.rsplit(':').next(),
            Some("SampleData" | "SampleDataExtension")
        )
    {
        return None;
    }
    static INFO: std::sync::OnceLock<ExtensionInfo> = std::sync::OnceLock::new();
    Some(INFO.get_or_init(|| ExtensionInfo {
        name: "SampleData".into(),
        parameters: vec!["ItemCount".into()],
        documentation: "Generate preview-only data for design-time ItemsSource or DataContext. ItemCount is an integer from 0 to 200 (default 5); fields follow template bindings.".into(),
    }))
}

/// Validate only the exact designer extension. Application extensions with the same local
/// name retain their normal schema behavior; they are never mistaken for preview data.
pub(crate) fn sample_diagnostic(
    document: &Document,
    node: &Node,
    attribute: &Attribute,
) -> Option<String> {
    let MarkupValue::Extension(value) = attribute.markup().ok()? else {
        return None;
    };
    let (prefix, _) = value.name.split_once(':')?;
    let namespaces = document.ast().namespaces(node.id);
    extension(&value.name, namespaces.get(prefix).map(String::as_str))?;
    let target = if attribute.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE) {
        attribute.name.rsplit(':').next()?
    } else if !attribute.name.contains(':') {
        attribute.name.strip_prefix("Design.").unwrap_or("")
    } else {
        ""
    };
    if !matches!(target, "ItemsSource" | "DataContext") {
        return Some("SampleData is supported on design-time ItemsSource and DataContext.".into());
    }
    for argument in &value.arguments {
        if argument.name.as_deref() != Some("ItemCount") {
            return Some("Design SampleData accepts only ItemCount.".into());
        }
        let MarkupValue::Literal(count) = &argument.value else {
            return Some("Sample ItemCount must be an integer from 0 to 200.".into());
        };
        if count.parse::<usize>().ok().is_none_or(|count| count > 200) {
            return Some("Sample ItemCount must be an integer from 0 to 200.".into());
        }
    }
    None
}

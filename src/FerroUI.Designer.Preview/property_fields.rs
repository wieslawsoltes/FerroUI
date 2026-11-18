use crate::document::{DesignDocument, schema};
use crate::edits::editable_attribute;
use ferroui_designer_core::{Node, NodeKind, SourceSpan};
use ferroui_designer_language::{PropertyInfo, Range};
use ferroui_designer_tools::{PropertyDescriptor, PropertyKind, PropertyRegistry};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PropertyField {
    name: String,
    group: String,
    kind: &'static str,
    options: Vec<String>,
    authored: Option<String>,
    effective: Option<String>,
    origin: Option<String>,
    range: Option<Range>,
    editable: bool,
    can_reset: bool,
}

impl PropertyField {
    pub(crate) fn runtime(name: &str, origin: &str, effective: String) -> Self {
        Self {
            name: name.into(),
            group: "Runtime".into(),
            kind: "text",
            options: Vec::new(),
            authored: None,
            effective: Some(effective),
            origin: Some(origin.into()),
            range: None,
            editable: false,
            can_reset: false,
        }
    }
}

pub(crate) fn registry() -> &'static PropertyRegistry {
    static REGISTRY: OnceLock<PropertyRegistry> = OnceLock::new();
    REGISTRY.get_or_init(PropertyRegistry::built_in)
}

pub(crate) fn property_fields(
    design: &DesignDocument<'_>,
    node: &Node,
    effective: Vec<ferroui_inspect::PropertyInfo>,
) -> Vec<PropertyField> {
    if node.kind != NodeKind::Element {
        return Vec::new();
    }
    let effective: BTreeMap<_, _> = effective
        .into_iter()
        .map(|info| (info.name.clone(), info))
        .collect();
    let mut schema_fields = schema().properties(node.local_name(), node.namespace_uri.as_deref());
    let descriptors: BTreeMap<_, _> = registry()
        .descriptors(&design.xaml, node.id)
        .unwrap_or_default()
        .into_iter()
        .map(|descriptor| (descriptor.name.clone(), descriptor))
        .collect();
    for name in descriptors.keys().filter(|name| name.contains('.')) {
        if let Some(info) = member(node, name) {
            schema_fields.insert(name.clone(), info);
        }
    }
    for attribute in node.attributes.iter().filter(|attr| !attr.is_namespace()) {
        schema_fields
            .entry(attribute.name.clone())
            .or_insert_with(|| member(node, &attribute.name).unwrap_or_default());
    }
    let element_values = object_fields(design, node);
    for name in element_values.keys() {
        schema_fields.entry(name.clone()).or_default();
    }
    let rsx = design.rsx_node(node.id).map(|(_, node)| node);
    schema_fields
        .into_iter()
        .filter_map(|(name, info)| {
            if info.event
                || (name == "Name"
                    && node.xaml_name().is_some()
                    && node.attribute_info("Name").is_none())
            {
                return None;
            }
            let attribute = node.attribute_info(&name);
            let expression = rsx.and_then(|node| node.expressions.get(&name));
            let object = element_values.get(&name);
            let (kind, options) = editor_kind(&name, &info, descriptors.get(&name));
            let addable = can_add(design, node, &name, &info, kind);
            if attribute.is_none()
                && object.is_none()
                && expression.is_none()
                && !addable
                && !common_readable(&name, &info)
            {
                return None;
            }
            let literal = editable_attribute(design, node, &name);
            let editable = object.is_none() && expression.is_none() && (literal || addable);
            let runtime = effective.get(&name);
            let origin = if expression.is_some() {
                Some("Rust expression".into())
            } else if object.is_some() {
                Some("Property element".into())
            } else if attribute.is_some() && !literal {
                Some("Source value".into())
            } else {
                runtime.map(|info| info.priority_name())
            };
            let span = expression
                .copied()
                .or_else(|| rsx.and_then(|node| node.properties.get(&name).copied()))
                .or_else(|| attribute.map(|attr| attr.value_span))
                .or_else(|| object.copied());
            let authored = expression
                .and_then(|span| design.source.get(span.range()).map(str::to_owned))
                .or_else(|| attribute.map(|attr| attr.value.clone()));
            Some(PropertyField {
                group: category(&name, descriptors.get(&name)),
                name,
                kind,
                options,
                authored,
                effective: runtime.map(|info| ferroui_inspect::display_value(Some(&info.value))),
                range: span.and_then(|span| design.range(span).ok()),
                origin,
                editable,
                can_reset: editable && literal,
            })
        })
        .collect()
}

pub(crate) fn can_add_property(design: &DesignDocument<'_>, node: &Node, name: &str) -> bool {
    let Some(info) = member(node, name) else {
        return false;
    };
    let descriptor = if name.contains('.') {
        let Some(descriptor) = registry()
            .descriptors(&design.xaml, node.id)
            .unwrap_or_default()
            .into_iter()
            .find(|field| field.name == name)
        else {
            return false;
        };
        Some(descriptor)
    } else {
        None
    };
    let (kind, _) = editor_kind(name, &info, descriptor.as_ref());
    !object_fields(design, node).contains_key(name) && can_add(design, node, name, &info, kind)
}

fn can_add(
    design: &DesignDocument<'_>,
    node: &Node,
    name: &str,
    info: &PropertyInfo,
    kind: &str,
) -> bool {
    node.kind == NodeKind::Element
        && !info.name.is_empty()
        && !info.event
        && !info.read_only
        && kind != "object"
        && node.attribute_info(name).is_none()
        && !(name == "Name" && node.xaml_name().is_some())
        && !(name == "Content"
            && node
                .children
                .iter()
                .filter_map(|id| design.xaml.ast().node(*id))
                .any(|child| {
                    matches!(child.kind, NodeKind::Element | NodeKind::CData)
                        || child.kind == NodeKind::Text
                            && child
                                .text
                                .as_ref()
                                .is_some_and(|text| !text.trim().is_empty())
                }))
        && design.rsx_node(node.id).is_none_or(|(_, node)| {
            node.supports_literal(name) && !node.expressions.contains_key(name)
        })
}

fn member(node: &Node, name: &str) -> Option<PropertyInfo> {
    schema().property(node.local_name(), node.namespace_uri.as_deref(), name)
}

fn object_fields(design: &DesignDocument<'_>, node: &Node) -> BTreeMap<String, SourceSpan> {
    node.children
        .iter()
        .filter_map(|child| {
            let child = design.xaml.ast().node(*child)?;
            if child.kind != NodeKind::PropertyElement {
                return None;
            }
            let (owner, name) = child.local_name().split_once('.')?;
            let name = if owner == node.local_name() || member(node, name).is_some() {
                name
            } else {
                child.local_name()
            };
            Some((name.to_owned(), child.name_span))
        })
        .collect()
}

fn editor_kind(
    name: &str,
    info: &PropertyInfo,
    descriptor: Option<&PropertyDescriptor>,
) -> (&'static str, Vec<String>) {
    let plain = |kind| (kind, Vec::new());
    match descriptor.map(|descriptor| &descriptor.kind) {
        Some(PropertyKind::Boolean) => plain("boolean"),
        Some(PropertyKind::Number { .. } | PropertyKind::Integer { .. }) => plain("number"),
        Some(PropertyKind::Enum(values)) => ("enum", values.clone()),
        Some(PropertyKind::Color) => plain("color"),
        Some(PropertyKind::Thickness) => plain("thickness"),
        Some(PropertyKind::GridDefinitions | PropertyKind::GridLength) => plain("text"),
        _ => match info.type_name.as_str() {
            "bool" | "Option<bool>" => plain("boolean"),
            "f64" | "f32" | "i32" | "u32" | "i64" | "u64" | "usize" => plain("number"),
            "Color" | "Option<Rc<dyn IBrush>>" => plain("color"),
            "Thickness" | "CornerRadius" => plain("thickness"),
            "String" | "Option<String>" | "FontFamily" | "FontWeight" | "GridLength"
            | "BoxShadows" => plain("text"),
            "Option<BoxedValue>" if matches!(name, "Content" | "Header") => plain("text"),
            "Option<Rc<dyn IImage>>" if name == "Source" => plain("text"),
            _ if !info.values.is_empty() => ("enum", info.values.clone()),
            _ => plain("object"),
        },
    }
}

fn category(name: &str, descriptor: Option<&PropertyDescriptor>) -> String {
    if matches!(
        name,
        "Text"
            | "FontFamily"
            | "FontSize"
            | "FontWeight"
            | "FontStyle"
            | "TextAlignment"
            | "TextWrapping"
            | "Foreground"
            | "LetterSpacing"
            | "LineHeight"
    ) {
        return "Typography".into();
    }
    if matches!(
        name,
        "Background"
            | "BorderBrush"
            | "BorderThickness"
            | "CornerRadius"
            | "BoxShadow"
            | "Fill"
            | "Stroke"
            | "StrokeThickness"
    ) {
        return "Appearance".into();
    }
    descriptor
        .map_or("Control", |descriptor| descriptor.category.as_str())
        .into()
}

fn common_readable(name: &str, info: &PropertyInfo) -> bool {
    !info.name.is_empty()
        && matches!(
            name,
            "Width"
                | "Height"
                | "Margin"
                | "Padding"
                | "HorizontalAlignment"
                | "VerticalAlignment"
                | "Background"
                | "BorderBrush"
                | "Fill"
                | "Stroke"
                | "Opacity"
                | "Text"
                | "Content"
                | "FontSize"
                | "Foreground"
                | "Spacing"
        )
}

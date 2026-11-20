//! Local color resources and theme dictionaries. Computed values and external providers
//! remain source-backed. The native host validates literal colors before planning edits.
use crate::components::{
    ComponentAction, ComponentResourceContext, authoring, component_property_attribute, element,
    external_dictionary, framework, literal_key, plan_component_action, property_elements,
    references, resolve_component_resource_in_context, resource_owner, valid_document, validated,
};
use crate::{Result, ToolError, require};
use ferroui_designer_core::{Document, Edit, MarkupValue, Node, NodeId, NodeKind, SourceSpan};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PaletteLayer {
    Shared,
    Default,
    Light,
    Dark,
}

impl PaletteLayer {
    pub const ALL: [Self; 4] = [Self::Shared, Self::Default, Self::Light, Self::Dark];

    pub fn title(self) -> &'static str {
        match self {
            Self::Shared => "Shared",
            Self::Default => "Default",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PaletteColorKind {
    Color,
    SolidColorBrush,
}

#[derive(Clone, Debug)]
pub struct PaletteColor {
    pub node: NodeId,
    pub key: String,
    pub kind: PaletteColorKind,
    pub value: Option<String>,
    pub span: SourceSpan,
    pub editable: bool,
}

#[derive(Clone, Debug)]
pub struct PaletteLayerModel {
    pub layer: PaletteLayer,
    pub declaration: Option<NodeId>,
    pub unavailable: Option<String>,
    pub colors: Vec<PaletteColor>,
}

/// Describe only declarations owned by this scope. A scope's display should distinguish
/// absent colors from resolved fallbacks using the separate resource resolver.
pub fn palette_layers(document: &Document, owner: NodeId) -> Result<Vec<PaletteLayerModel>> {
    valid_document(document)?;
    PaletteLayer::ALL
        .into_iter()
        .map(|layer| {
            let plan = destination(document, owner, layer);
            let declaration = match &plan {
                Ok(Destination::Container(node)) => Some(*node),
                _ => None,
            };
            let colors = declaration.map_or_else(
                || Ok(Vec::new()),
                |node| {
                    document
                        .ast()
                        .element_children(node)
                        .into_iter()
                        .filter_map(|node| match palette_color(document, node) {
                            Ok(value) => value.map(Ok),
                            Err(error) => Some(Err(error)),
                        })
                        .collect()
                },
            )?;
            Ok(PaletteLayerModel {
                layer,
                declaration,
                unavailable: plan.err().map(|error| error.0),
                colors,
            })
        })
        .collect()
}

pub fn palette_color(document: &Document, node: NodeId) -> Result<Option<PaletteColor>> {
    let declaration = element(document, node)?;
    let Some(key) = literal_key(declaration).filter(|_| resource_owner(document, node).is_some())
    else {
        return Ok(None);
    };
    let Some(kind) = color_kind(declaration) else {
        return Ok(None);
    };
    let (value, span, editable) = color_value(document, declaration, kind)?;
    Ok(Some(PaletteColor {
        node,
        key,
        kind,
        value,
        span,
        editable,
    }))
}

pub fn resolve_palette_color(
    document: &Document,
    owner: NodeId,
    key: &str,
    theme: PaletteLayer,
) -> Option<PaletteColor> {
    let context = ComponentResourceContext {
        theme: (theme != PaletteLayer::Shared).then(|| theme.title().into()),
        inherited_themes: Vec::new(),
    };
    let node = resolve_component_resource_in_context(document, owner, key, &context)?;
    palette_color(document, node).ok().flatten()
}

pub fn plan_palette_color(document: &Document, node: NodeId, color: &str) -> Result<Vec<Edit>> {
    valid_document(document)?;
    validate_color_literal(color)?;
    let entry = palette_color(document, node)?
        .ok_or_else(|| ToolError("Choose an authored palette color".into()))?;
    require(
        entry.editable,
        "Reveal the existing color expression in source",
    )?;
    let declaration = element(document, node)?;
    if entry.kind == PaletteColorKind::SolidColorBrush
        && property_elements(document, declaration, "Color")?.is_empty()
    {
        return plan_component_action(
            document,
            &ComponentAction::SetProperty {
                node,
                property: "Color".into(),
                value: Some(color.into()),
            },
        );
    }
    if entry.value.is_some() {
        return validated(
            document,
            vec![Edit::Text {
                span: entry.span,
                replacement: color.into(),
            }],
        );
    }
    plan_component_action(
        document,
        &ComponentAction::SetResourceValue {
            resource: node,
            value: color.into(),
        },
    )
}

pub fn plan_add_palette_color(
    document: &Document,
    owner: NodeId,
    layer: PaletteLayer,
    key: &str,
    kind: PaletteColorKind,
    color: &str,
) -> Result<Vec<Edit>> {
    valid_document(document)?;
    validate_color_literal(color)?;
    let to = destination(document, owner, layer)?;
    let at = match &to {
        Destination::Container(node) | Destination::NewLayer { parent: node, .. } => *node,
        Destination::NewDictionary {
            resource_property, ..
        } => resource_property.unwrap_or(owner),
    };
    let (prefix, xaml, namespaces) = authoring::creation_namespaces(document, at);
    let fragment = match kind {
        PaletteColorKind::Color => format!("<{prefix}:Color{namespaces}>{color}</{prefix}:Color>"),
        PaletteColorKind::SolidColorBrush => {
            format!("<{prefix}:SolidColorBrush{namespaces} Color=\"{color}\" />")
        }
    };
    match to {
        Destination::Container(node) => plan_component_action(
            document,
            &ComponentAction::CreateResource {
                owner: node,
                key: key.into(),
                source: fragment,
            },
        ),
        Destination::NewLayer {
            parent,
            property_exists,
            dictionary,
        } => {
            authoring::validate_key(key)?;
            let color = authoring::keyed_fragment(document, parent, &fragment, key)?;
            let variant = variant_markup(&prefix, &xaml, &namespaces, layer, &color);
            let property = format!("{dictionary}.ThemeDictionaries");
            let source = if property_exists {
                variant
            } else {
                format!("<{property}>\n{variant}\n</{property}>")
            };
            append(document, parent, source)
        }
        Destination::NewDictionary { resource_property } if layer == PaletteLayer::Shared => {
            plan_component_action(
                document,
                &ComponentAction::CreateResource {
                    owner: resource_property.unwrap_or(owner),
                    key: key.into(),
                    source: fragment,
                },
            )
        }
        Destination::NewDictionary { resource_property } => {
            authoring::validate_key(key)?;
            let color = authoring::keyed_fragment(document, at, &fragment, key)?;
            let variant = variant_markup(&prefix, &xaml, &namespaces, layer, &color);
            let property = format!("{prefix}:ResourceDictionary.ThemeDictionaries");
            let themes = format!("<{property}>\n{variant}\n</{property}>");
            let wrapper = format!("{prefix}:ResourceDictionary");
            if let Some(resources) = resource_property {
                let node = element(document, resources)?;
                if let Some(close) = node.closing_span {
                    let inner = SourceSpan::new(node.opening_span.end, close.start);
                    // Keep all authored entries, comments, and whitespace byte for byte.
                    let body = &document.source()[inner.range()];
                    let eol = if body.contains("\r\n") { "\r\n" } else { "\n" };
                    let themes = themes.replace('\n', eol);
                    return validated(
                        document,
                        vec![Edit::Text {
                            span: inner,
                            replacement: format!(
                                "<{wrapper}{namespaces}>{body}{themes}{eol}</{wrapper}>"
                            ),
                        }],
                    );
                }
                return append(
                    document,
                    resources,
                    format!("<{wrapper}{namespaces}>\n{themes}\n</{wrapper}>"),
                );
            }
            let name = format!("{}.Resources", element(document, owner)?.name);
            let source =
                format!("<{name}>\n<{wrapper}{namespaces}>\n{themes}\n</{wrapper}>\n</{name}>");
            validated(
                document,
                vec![Edit::InsertChild {
                    parent: owner,
                    index: 0,
                    source,
                }],
            )
        }
    }
}

enum Destination {
    Container(NodeId),
    NewLayer {
        parent: NodeId,
        property_exists: bool,
        dictionary: String,
    },
    NewDictionary {
        resource_property: Option<NodeId>,
    },
}

fn destination(document: &Document, owner: NodeId, layer: PaletteLayer) -> Result<Destination> {
    element(document, owner)?;
    require(
        component_property_attribute(document, owner, "Resources")?.is_none(),
        "Reveal the existing Resources assignment in source",
    )?;
    let Some(container) = authoring::resource_container(document, owner)? else {
        return Ok(Destination::NewDictionary {
            resource_property: None,
        });
    };
    if layer == PaletteLayer::Shared {
        return Ok(Destination::Container(container));
    }
    let dictionary = document
        .ast()
        .node(container)
        .ok_or_else(|| ToolError("The resource scope changed".into()))?;
    if dictionary.kind == NodeKind::PropertyElement {
        return Ok(Destination::NewDictionary {
            resource_property: Some(container),
        });
    }
    let blocks = property_elements(document, dictionary, "ThemeDictionaries")?;
    require(
        blocks.len() <= 1
            && component_property_attribute(document, container, "ThemeDictionaries")?.is_none(),
        "Reveal the existing ThemeDictionaries assignment in source",
    )?;
    if let Some(block) = blocks.first() {
        let variants = document.ast().element_children(block.id);
        let mut children = variants
            .into_iter()
            .filter_map(|id| document.ast().node(id))
            .map(|node| (node, references::theme_key(document, node)))
            .collect::<Vec<_>>();
        require(
            children.iter().all(|(_, key)| key.is_some()),
            "An unresolved theme key may overlap this layer; inspect the dictionary in source",
        )?;
        children.retain(|(_, key)| key.as_deref() == Some(layer.title()));
        require(
            children.len() <= 1,
            "More than one resource dictionary owns this theme layer",
        )?;
        if let Some((node, _)) = children.first() {
            require(
                framework(node)
                    && node.local_name() == "ResourceDictionary"
                    && !external_dictionary(document, node),
                "Open the external theme dictionary to edit its colors",
            )?;
            return Ok(Destination::Container(node.id));
        }
    }
    Ok(Destination::NewLayer {
        parent: blocks.first().map_or(container, |node| node.id),
        property_exists: !blocks.is_empty(),
        dictionary: dictionary.name.clone(),
    })
}

fn color_kind(node: &Node) -> Option<PaletteColorKind> {
    if !framework(node) {
        return None;
    }
    match node.local_name() {
        "Color" => Some(PaletteColorKind::Color),
        "SolidColorBrush" => Some(PaletteColorKind::SolidColorBrush),
        _ => None,
    }
}

fn color_value(
    document: &Document,
    node: &Node,
    kind: PaletteColorKind,
) -> Result<(Option<String>, SourceSpan, bool)> {
    let property = if kind == PaletteColorKind::SolidColorBrush {
        let elements = property_elements(document, node, "Color")?;
        let attribute = component_property_attribute(document, node.id, "Color")?;
        if let Some(attribute) = attribute {
            let editable =
                elements.is_empty() && matches!(attribute.markup(), Ok(MarkupValue::Literal(_)));
            return Ok((
                Some(attribute.value.clone()),
                attribute.value_span,
                editable,
            ));
        }
        match elements.as_slice() {
            [] => return Ok((None, node.name_span, true)),
            [property] => *property,
            _ => return Ok((None, node.name_span, false)),
        }
    } else {
        node
    };
    let texts = property
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|node| {
            node.kind == NodeKind::Text
                && node
                    .text
                    .as_deref()
                    .is_some_and(|text| !text.trim().is_empty())
        })
        .collect::<Vec<_>>();
    if !document.ast().element_children(property.id).is_empty()
        || property
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .any(|node| node.kind == NodeKind::CData)
    {
        return Ok((None, property.name_span, false));
    }
    match texts.as_slice() {
        [] => Ok((None, property.name_span, kind == PaletteColorKind::Color)),
        [text] => {
            let raw = &document.source()[text.span.range()];
            let start = text.span.start + raw.len() - raw.trim_start().len();
            let end = text.span.start + raw.trim_end().len();
            let value = text.text.as_deref().unwrap_or("").trim();
            Ok((
                Some(value.into()),
                SourceSpan::new(start, end),
                matches!(MarkupValue::parse(value), Ok(MarkupValue::Literal(_))),
            ))
        }
        _ => Ok((None, property.name_span, false)),
    }
}

fn append(document: &Document, parent: NodeId, source: String) -> Result<Vec<Edit>> {
    validated(
        document,
        vec![Edit::InsertChild {
            parent,
            index: document.ast().element_children(parent).len(),
            source,
        }],
    )
}

fn variant_markup(
    prefix: &str,
    xaml: &str,
    namespaces: &str,
    layer: PaletteLayer,
    child: &str,
) -> String {
    format!(
        "<{prefix}:ResourceDictionary{namespaces} {xaml}:Key=\"{}\">\n{child}\n</{prefix}:ResourceDictionary>",
        layer.title()
    )
}

fn validate_color_literal(color: &str) -> Result<()> {
    // The transport takes a canonical native value, so every generated XML fragment
    // contains only literal ARGB bytes. The native UI can accept names and short forms.
    require(
        color.len() == 9
            && color.starts_with('#')
            && color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit),
        "Use a validated #AARRGGBB color",
    )
}

#[cfg(test)]
#[path = "component_palette_tests.rs"]
mod tests;

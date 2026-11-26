//! Style and palette navigation always starts with an accepted authored include.
use crate::included_source::{IncludedSource, IncludedSourceIndex};
use crate::style_composition::supports_palette;
use crate::DesignerSession;
use ferroui_designer_core::{Document, NodeId, SourceSpan};
use ferroui_designer_project::PreviewInclude;
use ferroui_designer_tools::component_palette::{palette_color, palette_layers, PaletteLayer};
use ferroui_designer_tools::{component_property_elements, ComponentCatalog, ComponentKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StyleSourceKind {
    Style,
    ControlTheme,
    Palette,
    Color,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct StyleSource {
    #[serde(flatten)]
    pub location: IncludedSource,
    pub kind: StyleSourceKind,
}

#[derive(Serialize)]
pub(crate) struct StyleSourceChoice {
    pub label: String,
    pub source: StyleSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "page", rename_all = "lowercase")]
pub(crate) enum StyleEditorLocation {
    Styles {
        node: NodeId,
    },
    Palette {
        owner: NodeId,
        layer: PaletteLayer,
        color: Option<NodeId>,
    },
}

#[derive(Deserialize, Serialize)]
pub(crate) struct StyleNavigation {
    pub source: StyleSource,
    pub consumer: String,
}

impl StyleSource {
    pub(crate) fn editor_in(
        &self,
        document: &Document,
        uri: &str,
    ) -> Result<(StyleEditorLocation, SourceSpan), String> {
        let node = self.location.node_in(document, uri)?;
        let editor = match self.kind {
            StyleSourceKind::Style | StyleSourceKind::ControlTheme => {
                let kind = match self.kind {
                    StyleSourceKind::Style => ComponentKind::Style,
                    _ => ComponentKind::ControlTheme,
                };
                if !ComponentCatalog::declarations(document)
                    .map_err(|error| error.to_string())?
                    .iter()
                    .any(|entry| entry.node == node.id && entry.kind == kind)
                {
                    return Err("The included style declaration changed.".into());
                }
                StyleEditorLocation::Styles { node: node.id }
            }
            StyleSourceKind::Palette if supports_palette(document, node) => {
                StyleEditorLocation::Palette {
                    owner: node.id,
                    layer: PaletteLayer::Shared,
                    color: None,
                }
            }
            StyleSourceKind::Color => {
                let color = palette_color(document, node.id)
                    .map_err(|error| error.to_string())?
                    .ok_or("The included color declaration changed.")?;
                let (owner, layer) = palette_color_owner(document, color.node)
                    .ok_or("The included color has no editable palette scope.")?;
                return Ok((
                    StyleEditorLocation::Palette {
                        owner,
                        layer,
                        color: Some(color.node),
                    },
                    color.span,
                ));
            }
            _ => return Err("The included palette scope changed.".into()),
        };
        Ok((editor, node.name_span))
    }
}

impl StyleEditorLocation {
    pub(crate) fn owner(self) -> NodeId {
        match self {
            Self::Styles { node } => node,
            Self::Palette { owner, .. } => owner,
        }
    }
}

impl DesignerSession {
    pub(crate) fn style_source_choices(&self) -> Result<Vec<StyleSourceChoice>, String> {
        let mut choices = Vec::new();
        for include in self.style_include_documents().into_values() {
            let document = &include.document;
            let index = IncludedSourceIndex::new(include);
            for entry in
                ComponentCatalog::declarations(document).map_err(|error| error.to_string())?
            {
                let kind = match entry.kind {
                    ComponentKind::Style => StyleSourceKind::Style,
                    ComponentKind::ControlTheme => StyleSourceKind::ControlTheme,
                    _ => continue,
                };
                let node = document
                    .ast()
                    .node(entry.node)
                    .ok_or("The included declaration changed.")?;
                choices.push(StyleSourceChoice {
                    label: entry.display_name,
                    source: StyleSource {
                        location: index.locate(node)?,
                        kind,
                    },
                });
            }
            let ast = document.ast();
            if let Some(root) = ast.node(ast.root()).filter(|node| {
                supports_palette(document, node)
                    && (node.local_name() == "ResourceDictionary"
                        || component_property_elements(document, node.id, "Resources")
                            .is_ok_and(|elements| !elements.is_empty()))
            }) {
                choices.push(StyleSourceChoice {
                    label: root.xaml_name().unwrap_or(root.local_name()).into(),
                    source: StyleSource {
                        location: index.locate(root)?,
                        kind: StyleSourceKind::Palette,
                    },
                });
            }
        }
        Ok(choices)
    }

    pub(crate) fn style_include_documents(&self) -> BTreeMap<&str, &PreviewInclude> {
        self.accepted_preview_includes()
            .into_iter()
            .flat_map(|includes| includes.documents())
            .map(|include| (include.uri.as_str(), include))
            .collect()
    }

    pub(crate) fn open_style_source(&self, source: StyleSource) -> Result<StyleNavigation, String> {
        let include = self
            .accepted_preview_includes()
            .and_then(|includes| {
                includes
                    .documents()
                    .find(|include| include.uri == source.location.uri)
            })
            .ok_or("Choose a declaration from the current preview's included sources.")?;
        source.editor_in(&include.document, &include.uri)?;
        Ok(StyleNavigation {
            source,
            consumer: self.document_uri().into(),
        })
    }
}

fn palette_color_owner(document: &Document, color: NodeId) -> Option<(NodeId, PaletteLayer)> {
    let mut current = document.ast().node(color)?.parent;
    let mut shared = None;
    while let Some(node) = current.and_then(|id| document.ast().node(id)) {
        if supports_palette(document, node) {
            if let Some(layer) = palette_layers(document, node.id)
                .ok()?
                .into_iter()
                .find(|layer| layer.colors.iter().any(|entry| entry.node == color))
            {
                if layer.layer != PaletteLayer::Shared {
                    return Some((node.id, layer.layer));
                }
                shared.get_or_insert((node.id, layer.layer));
            }
        }
        current = node.parent;
    }
    shared
}

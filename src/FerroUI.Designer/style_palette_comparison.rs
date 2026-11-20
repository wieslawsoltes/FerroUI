//! Light and Dark comparisons follow the same graph as the accepted preview.
use crate::included_source::IncludedSourceIndex;
use crate::style_composition::argb;
use crate::style_sources::{StyleSource, StyleSourceKind};
use crate::DesignerSession;
use ferroui_designer_core::{NodeId, NodeKind};
use ferroui_designer_tools::component_palette::{palette_color, PaletteLayer, PaletteLayerModel};
use ferroui_designer_tools::{component_resource_location, component_resource_resolution};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize)]
pub(crate) struct ComparedColor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<NodeId>,
    pub value: Option<String>,
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<StyleSource>,
}

#[derive(Serialize)]
pub(crate) struct PaletteComparison {
    pub key: String,
    pub light: Option<ComparedColor>,
    pub dark: Option<ComparedColor>,
}

impl DesignerSession {
    pub(crate) fn compare_palette(
        &self,
        owner: NodeId,
        layers: &[PaletteLayerModel],
    ) -> Result<Vec<PaletteComparison>, String> {
        let local = layers
            .iter()
            .flat_map(|layer| layer.colors.iter().map(|color| color.key.clone()))
            .collect::<BTreeSet<_>>();
        let includes = self.style_include_documents();
        let mut keys = local.clone();
        for include in includes.values() {
            let document = &include.document;
            keys.extend(
                document
                    .ast()
                    .elements()
                    .filter(|node| matches!(node.local_name(), "Color" | "SolidColorBrush"))
                    .filter_map(|node| {
                        palette_color(document, node.id)
                            .ok()
                            .flatten()
                            .map(|entry| entry.key)
                    }),
            );
        }
        let document = self.document();
        let mut indexes = BTreeMap::new();
        let mut resolve = |key: &str,
                           layer: PaletteLayer|
         -> Result<Option<ComparedColor>, String> {
            let context = layer.resource_context();
            let (uri, node) = if includes.is_empty() {
                let local = component_resource_resolution(document, owner, key, Some(&context))
                    .map_err(|error| error.to_string())?;
                let Some(node) = local.resolved() else {
                    return Ok(None);
                };
                (None, node)
            } else {
                let location = component_resource_location(
                    document,
                    self.document_uri(),
                    owner,
                    key,
                    Some(&context),
                    &self.preview.resource_sources,
                )
                .map_err(|error| error.to_string())?;
                let Some(location) = location else {
                    return Ok(None);
                };
                (location.uri, location.node)
            };
            let include = uri.and_then(|uri| includes.get(uri).copied());
            let source_doc = include.map_or(document, |include| &include.document);
            let Some(color) = palette_color(source_doc, node).map_err(|error| error.to_string())?
            else {
                return Ok(None);
            };
            let ast = source_doc.ast();
            let declaration = ast.node(node).ok_or("The palette declaration changed.")?;
            let source = include
                .map(|include| {
                    let index = indexes
                        .entry(include.uri.as_str())
                        .or_insert_with(|| IncludedSourceIndex::new(include));
                    Ok::<_, String>(StyleSource {
                        location: index.locate(declaration)?,
                        kind: StyleSourceKind::Color,
                    })
                })
                .transpose()?;
            let scope = uri.map(str::to_owned).or_else(|| {
                let parent = ast.node(declaration.parent?)?;
                let owner = if parent.kind == NodeKind::PropertyElement {
                    ast.node(parent.parent?)?
                } else {
                    parent
                };
                Some(owner.xaml_name().unwrap_or(owner.local_name()).into())
            });
            Ok(Some(ComparedColor {
                node: include.is_none().then_some(node),
                value: color.value.as_deref().and_then(|value| argb(value).ok()),
                scope,
                source,
            }))
        };
        let mut rows = Vec::new();
        for key in keys {
            let light = resolve(&key, PaletteLayer::Light)?;
            let dark = resolve(&key, PaletteLayer::Dark)?;
            if light.is_some() || dark.is_some() || local.contains(&key) {
                rows.push(PaletteComparison { key, light, dark });
            }
        }
        Ok(rows)
    }
}

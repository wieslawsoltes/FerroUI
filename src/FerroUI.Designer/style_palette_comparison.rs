//! Palette comparison follows the accepted resource graph and native variant chain.
use crate::included_source::IncludedSourceIndex;
use crate::runtime_include_sources::IncludeLookup;
use crate::scoped_palette::ScopedPalette;
use crate::style_composition::argb;
use crate::style_sources::{StyleSource, StyleSourceKind};
use crate::PreviewTheme;
use ferroui_designer_core::{NodeId, NodeKind};
use ferroui_designer_tools::component_palette::{palette_color, PaletteLayer, PaletteLayerModel};
use ferroui_designer_tools::{
    component_resource_location, component_resource_resolution, ComponentResourceContext,
};
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
pub(crate) struct PaletteComparisonRow {
    pub key: String,
    pub light: Option<ComparedColor>,
    pub dark: Option<ComparedColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high_contrast: Option<ComparedColor>,
}

pub(crate) struct PaletteComparison {
    pub rows: Vec<PaletteComparisonRow>,
    pub high_contrast: bool,
}

pub(crate) fn contrast_context() -> Option<ComponentResourceContext> {
    let variant = PreviewTheme::HighContrast.resolve().ok()??;
    crate::preview_theme::resource_context(Some(variant))
}

impl ScopedPalette<'_> {
    pub(crate) fn compare_palette(
        &self,
        owner: NodeId,
        layers: &[PaletteLayerModel],
    ) -> Result<PaletteComparison, String> {
        let local = layers
            .iter()
            .flat_map(|layer| layer.colors.iter().map(|color| color.key.clone()))
            .collect::<BTreeSet<_>>();
        let includes: BTreeMap<_, _> =
            self.includes.into_iter().flat_map(|includes| includes.documents())
                .map(|include| (include.uri.as_str(), include)).collect();
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
        let document = self.document;
        let sources = self.includes.map(IncludeLookup);
        let mut indexes = BTreeMap::new();
        let mut resolve = |key: &str,
                           context: &ComponentResourceContext|
         -> Result<Option<ComparedColor>, String> {
            let (uri, node) = if let Some(sources) = &sources {
                let location = component_resource_location(
                    document,
                    self.address,
                    owner,
                    key,
                    Some(context),
                    sources,
                )
                .map_err(|error| error.to_string())?;
                let Some(location) = location else {
                    return Ok(None);
                };
                (location.uri, location.node)
            } else {
                let local = component_resource_resolution(document, owner, key, Some(context))
                    .map_err(|error| error.to_string())?;
                let Some(node) = local.resolved() else {
                    return Ok(None);
                };
                (None, node)
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
        let light_context = PaletteLayer::Light.resource_context();
        let dark_context = PaletteLayer::Dark.resource_context();
        for key in keys {
            let light = resolve(&key, &light_context)?;
            let dark = resolve(&key, &dark_context)?;
            let high_contrast = self.contrast.as_ref()
                .map(|context| resolve(&key, context))
                .transpose()?
                .flatten();
            if light.is_some() || dark.is_some() || high_contrast.is_some() || local.contains(&key) {
                rows.push(PaletteComparisonRow {
                    key,
                    light,
                    dark,
                    high_contrast,
                });
            }
        }
        Ok(PaletteComparison { rows, high_contrast: self.contrast.is_some() })
    }
}

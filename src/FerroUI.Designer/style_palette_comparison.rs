//! Palette comparison follows the accepted resource graph and native variant chain.
use crate::PreviewTheme;
use crate::included_source::IncludedSourceIndex;
use crate::runtime_include_sources::IncludeLookup;
use crate::scoped_palette::ScopedPalette;
use crate::style_composition::argb;
use crate::style_sources::{StyleSource, StyleSourceKind};
use ferroui_designer_core::{NodeId, NodeKind, SourceSpan};
use ferroui_designer_language::{LineIndex, Range};
use ferroui_designer_tools::component_palette::{
    PaletteLayer, PaletteLayerModel, palette_color, palette_resource_keys,
};
use ferroui_designer_tools::{ComponentResourceContext, ResourceValue};
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
    #[serde(rename = "sourceAddress", skip_serializing_if = "Option::is_none")]
    pub source_address: Option<String>,
    pub range: Range,
    #[serde(skip)]
    pub span: SourceSpan,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<ColorPathStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct ColorPathStep {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<&'static str>,
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
        let mut keys = local.clone();
        keys.extend(palette_resource_keys(self.document));
        let mut indexed = BTreeSet::new();
        for include in self
            .includes
            .into_iter()
            .flat_map(|includes| includes.documents())
        {
            if indexed.insert(&include.uri) {
                keys.extend(palette_resource_keys(&include.document));
            }
        }
        let document = self.document;
        let sources = self.includes.map(IncludeLookup);
        let mut source_indexes = BTreeMap::new();
        let mut value_indexes = BTreeMap::new();
        let mut resolve = |key: &str,
                           context: &ComponentResourceContext|
         -> Result<Option<ComparedColor>, String> {
            let resource = ResourceValue::lookup(
                document,
                self.address,
                sources.as_ref().map(|sources| {
                    sources as &dyn ferroui_designer_tools::ComponentResourceSources
                }),
                owner,
                key,
                Some(context),
            )
            .map_err(|error| error.to_string())?;
            let Some(resource) = resource else {
                return Ok(None);
            };
            let trace = resource.color_trace(key, context);
            let closest_color = trace
                .steps
                .iter()
                .rev()
                .find_map(|step| {
                    let value = &step.value;
                    let color =
                        palette_color(value.document(), value.location().node).transpose()?;
                    Some(color.map(|color| (value, color)))
                })
                .transpose()
                .map_err(|error| error.to_string())?;
            let Some((terminal, color)) = closest_color else {
                return Ok(None);
            };
            let source_doc = terminal.document();
            let location = terminal.location();
            let include = location
                .uri
                .and_then(|_| self.includes?.authored_source(location.address));
            let ast = source_doc.ast();
            let declaration = ast
                .node(location.node)
                .ok_or("The palette declaration changed.")?;
            let source = include
                .map(|include| {
                    let index = source_indexes
                        .entry(include.uri.as_str())
                        .or_insert_with(|| IncludedSourceIndex::new(include));
                    Ok::<_, String>(StyleSource {
                        location: index.locate(declaration)?,
                        kind: StyleSourceKind::Color,
                    })
                })
                .transpose()?;
            let scope = location.uri.map(str::to_owned).or_else(|| {
                let parent = ast.node(declaration.parent?)?;
                let owner = if parent.kind == NodeKind::PropertyElement {
                    ast.node(parent.parent?)?
                } else {
                    parent
                };
                Some(owner.xaml_name().unwrap_or(owner.local_name()).into())
            });
            let range = value_indexes
                .entry(location.address)
                .or_insert_with(|| LineIndex::new(source_doc.source()))
                .range(color.span)
                .map_err(|error| error.to_string())?;
            let value = trace
                .issue
                .is_none()
                .then(|| color.value.as_deref().and_then(|value| argb(value).ok()))
                .flatten();
            let path = if trace.steps.len() > 1 {
                trace
                    .steps
                    .into_iter()
                    .map(|step| ColorPathStep {
                        key: step.key,
                        reference: step.reference,
                    })
                    .collect()
            } else {
                Vec::new()
            };
            Ok(Some(ComparedColor {
                node: location.uri.is_none().then_some(location.node),
                value,
                scope,
                source,
                source_address: include.map(|_| location.address.to_owned()),
                range,
                span: color.span,
                path,
                issue: trace.issue,
            }))
        };
        let mut rows = Vec::new();
        let light_context = PaletteLayer::Light.resource_context();
        let dark_context = PaletteLayer::Dark.resource_context();
        for key in keys {
            let light = resolve(&key, &light_context)?;
            let dark = resolve(&key, &dark_context)?;
            let high_contrast = self
                .contrast
                .as_ref()
                .map(|context| resolve(&key, context))
                .transpose()?
                .flatten();
            if light.is_some() || dark.is_some() || high_contrast.is_some() || local.contains(&key)
            {
                rows.push(PaletteComparisonRow {
                    key,
                    light,
                    dark,
                    high_contrast,
                });
            }
        }
        Ok(PaletteComparison {
            rows,
            high_contrast: self.contrast.is_some(),
        })
    }
}

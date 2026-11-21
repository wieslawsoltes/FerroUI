//! Shared palette reporting and edit validation for accepted XAML source graphs.
use crate::style_composition::{argb, contrast_providers};
use crate::style_palette_comparison::contrast_context;
use crate::DesignerSession;
use ferroui_designer_core::{Document, Edit, NodeId};
use ferroui_designer_language::LineIndex;
use ferroui_designer_project::PreviewIncludes;
use ferroui_designer_tools::component_palette::{
    palette_layers, plan_add_palette_color_with_provider, plan_palette_color, PaletteColorKind,
    PaletteLayer,
};
use ferroui_designer_tools::theme_variants::HighContrastProvider;
use ferroui_designer_tools::{ComponentCatalog, ComponentResourceContext};
use serde::Deserialize;
use serde_json::{json, Value};

/// A palette at a source address in one accepted native preview.
pub struct ScopedPalette<'a> {
    pub(crate) document: &'a Document,
    pub(crate) address: &'a str,
    pub(crate) includes: Option<&'a PreviewIncludes>,
    pub(crate) contrast: Option<ComponentResourceContext>,
    providers: Vec<HighContrastProvider>,
}

/// Source-backed palette actions shared by Studio and the application preview.
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum PaletteEdit {
    Set {
        node: NodeId,
        color: String,
    },
    Add {
        owner: NodeId,
        layer: PaletteLayer,
        key: String,
        kind: PaletteColorKind,
        color: String,
        provider: Option<HighContrastProvider>,
    },
}

impl<'a> ScopedPalette<'a> {
    pub fn new(
        document: &'a Document,
        address: &'a str,
        includes: Option<&'a PreviewIncludes>,
        contrast: Option<ComponentResourceContext>,
        providers: Vec<HighContrastProvider>,
    ) -> Self {
        Self {
            document,
            address,
            includes,
            contrast,
            providers,
        }
    }

    pub(crate) fn for_session(session: &'a DesignerSession) -> Self {
        Self::new(
            session.document(),
            session.document_uri(),
            session.accepted_preview_includes(),
            contrast_context(),
            contrast_providers(),
        )
    }

    pub fn report(&self, owner: NodeId) -> Result<Value, String> {
        let document = self.document;
        let index = LineIndex::new(document.source());
        let layers = palette_layers(document, owner).map_err(|error| error.to_string())?;
        let comparison = self.compare_palette(owner, &layers)?;
        let catalog =
            ComponentCatalog::from_document(document).map_err(|error| error.to_string())?;
        let layers = layers.into_iter().map(|layer| {
            let colors = layer.colors.into_iter().map(|color| {
                Ok(json!({"node":color.node,"key":color.key,"kind":color.kind,"authored":color.value,"editable":color.editable,
                    "color":color.value.as_deref().and_then(|value| argb(value).ok()),
                    "uses":catalog.entries.iter().find(|entry|entry.node==color.node).map_or(0,|entry|entry.usages),
                    "range":index.range(color.span).map_err(|error|error.to_string())?}))
            }).collect::<Result<Vec<_>, String>>()?;
            Ok(json!({"layer":layer.layer,"declaration":layer.declaration,"unavailable":layer.unavailable,"colors":colors}))
        }).collect::<Result<Vec<_>, String>>()?;
        Ok(json!({
            "owner":owner, "layers":layers, "comparison":comparison.rows,
            "contrastPreview":comparison.high_contrast, "contrastProviders":self.providers,
        }))
    }

    pub fn plan(&self, edit: &PaletteEdit) -> Result<Vec<Edit>, String> {
        match edit {
            PaletteEdit::Set { node, color } => {
                plan_palette_color(self.document, *node, &argb(color)?)
            }
            PaletteEdit::Add {
                owner,
                layer,
                key,
                kind,
                color,
                provider,
            } => {
                if let Some(provider) = provider {
                    if *layer != PaletteLayer::HighContrast {
                        return Err("Choose a theme provider for the High contrast layer".into());
                    }
                    if !self.providers.contains(provider) {
                        return Err(format!(
                            "{} is unavailable in this preview host",
                            provider.label()
                        ));
                    }
                }
                plan_add_palette_color_with_provider(
                    self.document,
                    *owner,
                    *layer,
                    key,
                    *kind,
                    &argb(color)?,
                    *provider,
                )
            }
        }
        .map_err(|error| error.to_string())
    }
}

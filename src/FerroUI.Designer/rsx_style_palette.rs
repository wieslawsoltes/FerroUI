//! The authored Rust palette uses the same comparison cells as XAML.
use crate::style_palette_comparison::{
    ColorPathStep, ComparedColor, PaletteComparison, PaletteComparisonRow,
};
use ferroui_designer_language::LineIndex;
use ferroui_designer_rsx::{RsxStyleResource, RsxStyleResourceFamily};
use ferroui_designer_tools::component_palette::PaletteLayer;
use serde_json::{Value, json};

pub(super) fn resource_report(index: &LineIndex, resource: &RsxStyleResource) -> Value {
    json!({"node":resource.declaration.start,"key":resource.key,"layer":resource.layer,
        "color":resource.layer.and_then(|_| resource.color()),"range":index.range(resource.color_source()).ok(),
        "shared_constant":resource.shared.as_ref().map(|constant| constant.name.as_str())})
}

pub(super) fn choice_report(index: &LineIndex, family: &RsxStyleResourceFamily<'_>) -> Value {
    match family.common_declaration() {
        Some(resource) => resource_report(index, resource),
        None => json!({"key":family.key}),
    }
}

pub(crate) fn comparison(
    index: &LineIndex,
    families: &[RsxStyleResourceFamily<'_>],
) -> PaletteComparison {
    let rows = families
        .iter()
        .map(|family| PaletteComparisonRow {
            key: family.key.into(),
            light: family.light.and_then(|resource| cell(index, resource)),
            dark: family.dark.and_then(|resource| cell(index, resource)),
            high_contrast: family
                .high_contrast
                .and_then(|resource| cell(index, resource)),
        })
        .collect();
    PaletteComparison {
        rows,
        high_contrast: true,
    }
}

fn cell(index: &LineIndex, resource: &RsxStyleResource) -> Option<ComparedColor> {
    Some(ComparedColor {
        node: None,
        value: resource.layer.and_then(|_| resource.color()),
        scope: resource.layer.map(PaletteLayer::title).map(str::to_owned),
        source: None,
        source_address: None,
        range: index.range(resource.color_source()).ok()?,
        span: resource.color_source(),
        path: resource
            .shared
            .iter()
            .map(|constant| ColorPathStep {
                key: format!("const {}", constant.name),
                reference: None,
            })
            .collect(),
        issue: resource
            .layer
            .is_none()
            .then(|| "The Rust theme is computed".into()),
    })
}

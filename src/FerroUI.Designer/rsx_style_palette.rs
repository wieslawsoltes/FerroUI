//! The authored Rust palette uses the same comparison cells as XAML.
use crate::style_palette_comparison::{
    ColorPathStep, ComparedColor, PaletteComparison, PaletteComparisonRow,
};
use ferroui_designer_language::{LineIndex, Range};
use ferroui_designer_project::RustModuleSources;
use ferroui_designer_rsx::{RsxStyleResource, RsxStyleResourceFamily};
use ferroui_designer_tools::component_palette::PaletteLayer;
use serde_json::{Value, json};

fn source_range<'a>(index: &LineIndex, modules: Option<&RustModuleSources>, resource: &'a RsxStyleResource) -> Option<(Range, Option<&'a str>)> {
    if let Some(shared) = &resource.shared && let Some(uri) = shared.uri.as_deref() {
        let owner = modules?.source(uri)?;
        let range = LineIndex::new(&owner.source).range(shared.value).ok()?;
        return Some((range, Some(uri)));
    }
    Some((index.range(resource.color_source()).ok()?, None))
}

pub(super) fn resource_report(index: &LineIndex, modules: Option<&RustModuleSources>, resource: &RsxStyleResource) -> Value {
    let source = source_range(index, modules, resource);
    json!({"node":resource.declaration.start,"key":resource.key,"layer":resource.layer,
        "color":resource.layer.and_then(|_| resource.color()),"range":source.as_ref().map(|(range, _)| range),
        "sourceAddress":source.and_then(|(_, address)| address),
        "shared_constant":resource.shared.as_ref().map(|constant| constant.name.as_str())})
}

pub(super) fn choice_report(index: &LineIndex, modules: Option<&RustModuleSources>, family: &RsxStyleResourceFamily<'_>) -> Value {
    match family.common_declaration() {
        Some(resource) => resource_report(index, modules, resource),
        None => json!({"key":family.key}),
    }
}

pub(crate) fn comparison(
    index: &LineIndex,
    modules: Option<&RustModuleSources>,
    families: &[RsxStyleResourceFamily<'_>],
) -> PaletteComparison {
    let rows = families
        .iter()
        .map(|family| PaletteComparisonRow {
            key: family.key.into(),
            light: family.light.and_then(|resource| cell(index, modules, resource)),
            dark: family.dark.and_then(|resource| cell(index, modules, resource)),
            high_contrast: family
                .high_contrast
                .and_then(|resource| cell(index, modules, resource)),
        })
        .collect();
    PaletteComparison {
        rows,
        high_contrast: true,
    }
}

fn cell(index: &LineIndex, modules: Option<&RustModuleSources>, resource: &RsxStyleResource) -> Option<ComparedColor> {
    let (range, address) = source_range(index, modules, resource)?;
    Some(ComparedColor {
        node: None,
        value: resource.layer.and_then(|_| resource.color()),
        scope: resource.layer.map(PaletteLayer::title).map(str::to_owned),
        source: None,
        source_address: address.map(str::to_owned),
        range,
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

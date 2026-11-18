use crate::{AssetIcon, DesignerAsset, default_designer_assets};
use ferroui_designer_project::MarkupCatalog;
use std::collections::{BTreeMap, BTreeSet};

/// Constructible controls from the app's registry and the registries selected by its build.
pub fn registered_designer_assets(
    catalog: &MarkupCatalog,
    package: Option<&str>,
) -> Vec<DesignerAsset> {
    let builtin_ids: BTreeSet<_> = default_designer_assets()
        .into_iter()
        .map(|asset| asset.id)
        .collect();
    let package = package.map(|name| name.replace('-', "_"));
    let mut assets = BTreeMap::new();
    for control in catalog.controls() {
        let name = &control.class.name;
        let assembly = &control.assembly.name;
        if catalog.is_top_level(control.class)
            || (assembly == "FerroUI.Controls" && builtin_ids.contains(name))
        {
            continue;
        }
        let namespace = control.namespace();
        let id = format!("registered:{namespace}:{name}");
        let category = match &package {
            Some(package) if *package == control.assembly.crate_name => "Project controls",
            Some(_) => "Dependency controls",
            None => "Registered controls",
        };
        let source = format!(
            r#"<ui:{name} xmlns:ui="{}" />"#,
            escape_namespace(&namespace)
        );
        let asset = DesignerAsset {
            id: id.clone(),
            title: name.clone(),
            category: category.into(),
            description: format!("{} · {assembly}", control.class.namespace),
            source,
            icon: AssetIcon::Control,
        };
        let origin = (&control.assembly.crate_name, &control.class.rust_path);
        let entry = assets.entry(id).or_insert_with(|| (origin, Some(asset)));
        if origin != entry.0 {
            entry.1 = None;
        }
    }
    assets
        .into_values()
        .filter_map(|(_, asset)| asset)
        .collect()
}

fn escape_namespace(namespace: &str) -> String {
    namespace
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('"', "&quot;")
}

use super::assembly_path;
use crate::animation::{Property, TargetType, ValueShape};
use ferroui_designer_project::MarkupCatalog;
use std::collections::BTreeMap;

const KEY: &str = "FerroUI.Designer.RsxAnimation.v1:";

pub(super) fn properties(
    catalog: &MarkupCatalog,
    package: Option<&str>,
) -> BTreeMap<String, Property> {
    let mut candidates = BTreeMap::new();
    for assembly in catalog.models().models() {
        for (key, value) in &assembly.metadata {
            let Some(path) = key
                .strip_prefix(KEY)
                .and_then(|path| assembly_path(path, &assembly.crate_name, package))
            else {
                continue;
            };
            let property = resolve(catalog, value, &assembly.crate_name);
            let name = path.trim_start_matches("::").to_owned();
            candidates
                .entry(name)
                .and_modify(|existing| *existing = None)
                .or_insert(property);
        }
    }
    candidates
        .into_iter()
        .filter_map(|(name, property)| Some((name, property?)))
        .collect()
}

fn resolve(catalog: &MarkupCatalog, value: &str, crate_name: &str) -> Option<Property> {
    let local = value
        .strip_prefix("crate::")
        .map(|path| format!("::{crate_name}::{path}"));
    let resolved = catalog.animation_property(local.as_deref().unwrap_or(value))?;
    let shape = ValueShape::from_type(&resolved.value_type)?;
    let name = resolved.owner.class.name.clone();
    let namespace = match resolved.owner.assembly.crate_name.as_str() {
        "ferroui_base" | "ferroui_controls" => None,
        _ => Some(resolved.owner.namespace()),
    };
    Some(Property {
        name: resolved.name.into(),
        shape,
        owner: Some(TargetType { name, namespace }),
    })
}

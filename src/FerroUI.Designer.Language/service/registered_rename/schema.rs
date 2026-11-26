use crate::{project_schema::ProjectSchema, require, Result, Schema};
use ferroui_designer_project::MarkupSource;

pub(super) fn proposed_schema(
    project: &ProjectSchema,
    registration: &MarkupSource,
    old: &str,
    new: &str,
) -> Result<Option<Schema>> {
    let schema = &project.effective;
    if !schema.types.values().any(|info| {
        info.properties
            .get(old)
            .is_some_and(|p| p.authored.as_deref() == Some(registration))
    }) {
        return Ok(None);
    }
    for info in schema.types.values() {
        let namespace = Some(info.namespace.as_str());
        if schema
            .property(&info.name, namespace, old)
            .is_some_and(|p| p.authored.as_deref() == Some(registration))
        {
            require(
                schema.property(&info.name, namespace, new).is_none(),
                format!("The name {new} already resolves on {}", info.name),
            )?;
        }
    }
    for assembly in project.catalog.models().models() {
        for owner in &assembly.types {
            if owner.content_property.as_deref() != Some(old) {
                continue;
            }
            let namespace = format!(
                "clr-namespace:{};assembly={}",
                owner.namespace, assembly.name
            );
            require(
                project
                    .effective
                    .property(&owner.name, Some(&namespace), old)
                    .is_none_or(|property| property.authored.as_deref() != Some(registration)),
                "This name also identifies the content property in Rust markup metadata",
            )?;
        }
    }
    let mut after = schema.clone();
    for info in after.types.values_mut() {
        if let Some(property) = info
            .properties
            .get(old)
            .filter(|p| p.authored.as_deref() == Some(registration))
        {
            let mut property = property.clone();
            property.name = new.into();
            info.properties.remove(old);
            info.properties.insert(new.into(), property);
        }
    }
    Ok(Some(after))
}

use crate::{require, Result, Schema, TypeInfo};
use ferroui_designer_project::MarkupSource;
use std::collections::BTreeSet;

pub(super) fn renamed(
    schema: &Schema,
    registration: &MarkupSource,
    new: &str,
) -> Result<Option<Schema>> {
    let matches = |info: &TypeInfo| {
        info.authored
            .as_ref()
            .is_some_and(|source| source.same_registration(registration))
    };
    let affected: Vec<_> = schema
        .types
        .iter()
        .filter(|(_, info)| matches(info))
        .collect();
    if affected.is_empty() {
        return Ok(None);
    }
    let mut identities = BTreeSet::new();
    for (_, info) in &affected {
        identities.insert((info.name.as_str(), info.namespace.as_str()));
        for spelling in [Some(new), shortened(new)].into_iter().flatten() {
            require(
                schema
                    .type_info(spelling, Some(&info.namespace))
                    .is_none_or(matches),
                format!(
                    "A type named {spelling} already resolves in {}",
                    info.namespace
                ),
            )?;
        }
    }
    let mut after = schema.clone();
    for (key, info) in affected {
        let mut info = info.clone();
        after.types.remove(key);
        info.name = new.into();
        after.register(info);
    }
    for info in after.types.values_mut() {
        if let Some(base) = info.model_base.as_mut() {
            if identities.contains(&(base.name.as_str(), base.namespace.as_str())) {
                base.name = new.into();
            }
        }
        for property in info.properties.values_mut() {
            if let Some(value) = property.model_type.as_mut() {
                if identities.contains(&(value.name.as_str(), value.namespace.as_str())) {
                    value.name = new.into();
                    property.type_name = new.into();
                }
            }
        }
    }
    Ok(Some(after))
}

pub(super) fn shortened(name: &str) -> Option<&str> {
    name.strip_suffix("Extension")
        .filter(|short| !short.is_empty())
}

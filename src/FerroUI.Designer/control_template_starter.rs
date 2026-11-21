//! Use the loader's registered template-part attributes to scaffold a valid control template.
use ferroui_base::{
    TypeInfo,
    metadata::{MarkupAttributeValue, MarkupType, attributes},
};
use ferroui_controls::{Border, Grid};
use ferroui_designer_tools::ControlTemplatePart;
use std::collections::BTreeSet;

pub(crate) fn required_parts(
    control: &'static TypeInfo,
) -> Result<Vec<ControlTemplatePart<'static>>, String> {
    use MarkupAttributeValue::{Bool, Str, Type};
    let mut seen = BTreeSet::new();
    let mut parts = Vec::new();
    let mut current = Some(control);
    while let Some(type_) = current {
        current = type_.base_type();
        let Some(markup) = MarkupType::find_by_type_info(type_) else {
            continue;
        };
        for part in markup
            .attributes
            .iter()
            .filter(|attribute| attribute.name == attributes::TEMPLATE_PART)
        {
            let Some(Str(name)) = part
                .property("Name")
                .or_else(|| part.arguments.first().copied())
            else {
                continue;
            };
            // A derived declaration overrides the base part even when it makes it optional.
            if name.is_empty()
                || !seen.insert(name)
                || part.property("IsRequired") != Some(Bool(true))
            {
                continue;
            }
            if !name
                .chars()
                .all(|character| character.is_alphanumeric() || character == '_')
            {
                return Err(format!(
                    "Required template part {name} has no supported XAML name"
                ));
            }
            let required = match part
                .property("Type")
                .or_else(|| part.arguments.get(1).copied())
            {
                Some(Type(handle)) => TypeInfo::find_by_handle(handle().id())
                    .map(|(type_, _)| type_)
                    .ok_or_else(|| {
                        format!("Register the type of required template part {name} first")
                    })?,
                _ => Border::TYPE,
            };
            let part_type = [required, Border::TYPE, Grid::TYPE]
                .into_iter()
                .find(|candidate| {
                    required.is_assignable_from(candidate) && has_default_constructor(candidate)
                })
                .ok_or_else(|| {
                    format!("Required template part {name} needs a constructible XAML type")
                })?;
            let markup = MarkupType::find_by_type_info(part_type);
            parts.push(ControlTemplatePart {
                name,
                type_name: markup.map_or_else(|| part_type.name(), |markup| markup.name),
                namespace: markup
                    .map_or_else(|| part_type.namespace(), |markup| markup.namespace()),
            });
        }
    }
    Ok(parts)
}

fn has_default_constructor(type_: &'static TypeInfo) -> bool {
    type_.default_constructor().is_some()
        || MarkupType::find_by_type_info(type_).is_some_and(|markup| {
            markup
                .constructors
                .iter()
                .any(|ctor| ctor.parameters.is_empty())
        })
}

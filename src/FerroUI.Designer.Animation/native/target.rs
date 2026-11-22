use ferroui_base::animation::{Animatable, Animation};
use ferroui_base::media::Transform;
use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::{FerroPropertyRegistry, TypeInfo};
use ferroui_designer_core::{Document, NodeId};

const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

/// Validate property ownership when a target was chosen without an authored scope.
pub fn validate_animation_target(
    animation: &Animation,
    target: &Animatable,
    role: &str,
) -> Result<(), String> {
    validate(animation, target, role, false)
}

pub(super) fn validate(
    animation: &Animation,
    target: &Animatable,
    role: &str,
    allow_added_owner: bool,
) -> Result<(), String> {
    for keyframe in animation.children().snapshot().iter() {
        for setter in keyframe.setters().snapshot().iter() {
            let property = setter
                .property()
                .ok_or("Native animation Setter has no target property")?;
            let owner = property.owner_type();
            if !property.is_attached()
                && !owner.is_assignable_from(target.get_type())
                && !Transform::TYPE.is_assignable_from(owner)
                && !(allow_added_owner
                    && FerroPropertyRegistry::instance().is_registered_for(target, property))
            {
                return Err(format!(
                    "{property} is incompatible with {role} {}",
                    target.get_type()
                ));
            }
        }
    }
    Ok(())
}

/// The authored type provides the context for properties registered through AddOwner.
pub(super) fn declared_type(document: &Document, id: NodeId) -> Option<&'static TypeInfo> {
    let directive = document
        .ast()
        .node(id)?
        .attributes
        .iter()
        .find(|attribute| {
            attribute.namespace_uri.as_deref() == Some(XAML)
                && attribute.name.rsplit(':').next() == Some("SetterTargetType")
        })?;
    let namespaces = document.ast().namespaces(id);
    let mut name = directive.value.trim();
    if let Some(extension) = name
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
    {
        let (token, value) = extension.split_once(char::is_whitespace)?;
        let (prefix, local) = token.split_once(':')?;
        if local != "Type" || namespaces.get(prefix).map(String::as_str) != Some(XAML) {
            return None;
        }
        name = value.trim();
    }
    let (prefix, local) = name.split_once(':').unwrap_or(("", name));
    let namespace = namespaces.get(prefix)?;
    if let Some(clr) = namespace
        .strip_prefix("using:")
        .or_else(|| namespace.strip_prefix("clr-namespace:"))
    {
        let (name, assembly) = clr
            .split_once(";assembly=")
            .map_or((clr, None), |(name, a)| (name, Some(a)));
        let type_ = TypeInfo::find(name, local)?;
        return assembly
            .is_none_or(|name| {
                MarkupAssembly::of_module(type_.module_path())
                    .is_some_and(|owner| owner.name == name)
            })
            .then_some(type_);
    }
    let mut matches = MarkupAssembly::registered_assemblies()
        .into_iter()
        .flat_map(|assembly| assembly.xmlns_definitions)
        .filter(|definition| definition.xml_namespace == namespace)
        .filter_map(|definition| TypeInfo::find(definition.namespace, local));
    let first = matches.next()?;
    matches
        .all(|other| std::ptr::eq(other, first))
        .then_some(first)
}

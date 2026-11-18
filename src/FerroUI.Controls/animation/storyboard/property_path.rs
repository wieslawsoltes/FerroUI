//! The `Storyboard.TargetProperty` paths of WinUI (`dxaml/xcp/core/animation/PropertyPath*` and
//! the path resolution of `CStoryboard` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md), with FerroUI property names.
//!
//! A path is a property of the target (`Opacity`), a property with its owner (`(Visual.Opacity)`,
//! `(Grid.Row)` for an attached property), or a chain of them through the values of the
//! properties (`(Visual.RenderTransform).(ScaleTransform.ScaleX)`), resolved when the animation
//! begins.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::media::{IBrush, ITransform};
use ferroui_base::{AnyValue, FerroObject, FerroProperty, FerroPropertyRegistry, Ref, TypeInfo};
use std::rc::Rc;

/// The segments of a path: each is `(owner, name)`, `owner` empty for a plain name.
pub(crate) fn parse(path: &str) -> Result<Vec<(String, String)>, String> {
    let mut segments = Vec::new();
    let mut rest = path.trim();
    while !rest.is_empty() {
        let (segment, after) = if let Some(inner) = rest.strip_prefix('(') {
            let end = inner
                .find(')')
                .ok_or_else(|| format!("unclosed '(' in the property path '{path}'"))?;
            let (owner, name) = inner[..end]
                .trim()
                .rsplit_once('.')
                .ok_or_else(|| format!("'({})' of the property path '{path}' has no owner", &inner[..end]))?;
            ((owner.trim().to_string(), name.trim().to_string()), &inner[end + 1..])
        } else {
            let end = rest.find('.').unwrap_or(rest.len());
            ((String::new(), rest[..end].trim().to_string()), &rest[end..])
        };
        if segment.1.is_empty() {
            return Err(format!("the property path '{path}' has an empty segment"));
        }
        segments.push(segment);
        rest = after.trim_start();
        if let Some(after_dot) = rest.strip_prefix('.') {
            rest = after_dot.trim_start();
            if rest.is_empty() {
                return Err(format!("the property path '{path}' ends with '.'"));
            }
        } else if !rest.is_empty() {
            return Err(format!("unexpected '{rest}' in the property path '{path}'"));
        }
    }
    if segments.is_empty() {
        return Err("the property path is empty".to_string());
    }
    Ok(segments)
}

/// The property of `object` that a segment names.
fn property_of(object: &FerroObject, owner: &str, name: &str) -> Option<&'static FerroProperty> {
    let registry = FerroPropertyRegistry::instance();
    // A property of the object (also when the owner is a base class of it: `(Visual.Opacity)`).
    let own = registry.find_registered_for(object, name);
    let mut current = Some(object.get_type());
    let owner_is_base = owner.is_empty()
        || std::iter::from_fn(|| {
            let type_ = current?;
            current = type_.base_type();
            Some(type_)
        })
        .any(|type_| type_.name() == owner);
    if let (Some(property), true) = (own, owner_is_base) {
        return Some(property);
    }
    if owner.is_empty() {
        return None;
    }
    // An attached property of the owner (`(Grid.Row)`), or a property of a type the object does
    // not derive from that it holds anyway.
    let owner_type = TypeInfo::find_by_name(owner)?;
    registry
        .get_registered_attached(owner_type)
        .iter()
        .copied()
        .find(|property| property.name() == name)
        .or(own)
}

/// Resolves `path` from `target`: the object whose property is animated, and the property.
pub(crate) fn resolve(
    target: &Ref<FerroObject>,
    path: &str,
) -> Result<(Ref<FerroObject>, &'static FerroProperty), String> {
    let segments = parse(path)?;
    let mut object = target.clone();
    let last = segments.len() - 1;
    for (index, (owner, name)) in segments.iter().enumerate() {
        let Some(property) = property_of(&object, owner, name) else {
            return Err(format!(
                "no property '{name}' on {} for the property path '{path}'",
                object.get_type().name()
            ));
        };
        if index == last {
            return Ok((object, property));
        }
        let value = object.get_value_untyped(property);
        object = object_of(&*value).ok_or_else(|| {
            format!("the value of '{name}' of the property path '{path}' is not an object that has properties")
        })?;
    }
    unreachable!("a parsed path has a segment")
}

/// The object a property value holds: a handle of a class, or a brush or a transform of the
/// object model (the values a path goes through: `(Border.Background).(SolidColorBrush.Color)`).
///
/// A brush or a transform shared by several elements (a resource) is animated for all of them,
/// as in WinUI.
fn object_of(value: &dyn AnyValue) -> Option<Ref<FerroObject>> {
    if let Some(object) = ValueTypes::as_object(value) {
        return Some(object);
    }
    let brush = |brush: &Rc<dyn IBrush>| brush.as_object().map(FerroObject::to_ref);
    let transform = |transform: &Rc<dyn ITransform>| transform.as_object().map(FerroObject::to_ref);
    if let Some(value) = value.downcast_ref::<Option<Rc<dyn IBrush>>>() {
        return value.as_ref().and_then(brush);
    }
    if let Some(value) = value.downcast_ref::<Rc<dyn IBrush>>() {
        return brush(value);
    }
    if let Some(value) = value.downcast_ref::<Option<Rc<dyn ITransform>>>() {
        return value.as_ref().and_then(transform);
    }
    value.downcast_ref::<Rc<dyn ITransform>>().and_then(transform)
}

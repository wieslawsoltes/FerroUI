use super::curve::matches_property;
use crate::{AnimationBoxShadows, AnimationEffect, AnimationProgram, AnimationValue, ImportOptions, ValueKind};
use ferroui_base::animation::{Animatable, Animation, animators::TransformAnimator};
use ferroui_base::media::{Color, IBrush};
use ferroui_base::{
    CornerRadius, FerroProperty, Point, Rect, RelativePoint, RelativeScalar, Size, Thickness,
    Vector,
};
use ferroui_designer_core::Document;
use std::any::TypeId;
use std::collections::BTreeMap;
use std::rc::Rc;

/// Resolve authored setter names against the app's loaded animation, where the
/// property's registered Rust type is authoritative for portable key values.
pub fn native_keyframe_options(
    document: &Document,
    program: &AnimationProgram,
    animation: &Animation,
) -> ImportOptions {
    native_keyframe_options_for_target(document, program, animation, None)
}

/// Include live underlying values for transform and box shadow tracks.
pub fn native_keyframe_options_for_target(
    document: &Document,
    program: &AnimationProgram,
    animation: &Animation,
    target: Option<&Animatable>,
) -> ImportOptions {
    let properties = animation
        .children()
        .snapshot()
        .iter()
        .flat_map(|frame| {
            frame
                .setters()
                .snapshot()
                .iter()
                .filter_map(|setter| setter.property())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut setters = BTreeMap::<_, Vec<_>>::new();
    for setter in document
        .ast()
        .subtree(program.id)
        .filter(|node| node.local_name() == "Setter")
    {
        if let Some(name) = setter.attribute("Property") {
            setters
                .entry(name)
                .or_default()
                .push(setter.attribute("Value"));
        }
    }
    let mut options = ImportOptions {
        native_effect_endpoints: true,
        effect_color: Some(AnimationEffect::resolve_color),
        effect_literal: Some(AnimationEffect::parse_native),
        ..ImportOptions::default()
    };
    let transform_components = animation.transform_components();
    for (name, values) in setters {
        let mut matches = properties
            .iter()
            .copied()
            .filter(|property| matches_property(property, name));
        let Some(property) = matches
            .next()
            .filter(|first| matches.all(|other| std::ptr::eq(*first, other)))
        else {
            continue;
        };
        if AnimationEffect::is_effect_property(property) {
            options.property_kinds.insert(name.into(), ValueKind::Effect);
            options.literal_values.extend(values.into_iter().flatten().filter_map(|text| {
                let text = text.strip_prefix("{}").unwrap_or(text);
                let value = AnimationEffect::parse_native(text).ok()?;
                Some(((name.to_owned(), text.to_owned()), AnimationValue::Effect(value)))
            }));
            continue;
        }
        if AnimationBoxShadows::is_property(property) {
            options.property_kinds.insert(name.into(), ValueKind::BoxShadows);
            if let Some(base) = target.and_then(|target| AnimationBoxShadows::base_value(target, property)) {
                options.base_values.insert(name.into(), AnimationValue::BoxShadows(base));
            }
            continue;
        }
        let mut kind = value_kind(property);
        if kind == Some(ValueKind::Color)
            || property.property_type() == TypeId::of::<Option<Rc<dyn IBrush>>>()
        {
            // A brush's registered type can also hold gradients and other payloads.
            // Only a complete track of static solid colors has RGBA components.
            let colors = values
                .into_iter()
                .map(|text| {
                    let text = text?;
                    let text = text.strip_prefix("{}").unwrap_or(text);
                    Some((text, Color::try_parse(text)?))
                })
                .collect::<Option<Vec<_>>>();
            if let Some(colors) = colors {
                kind = Some(ValueKind::Color);
                options.literal_values.extend(
                    colors
                        .into_iter()
                        .map(|(text, color)| ((name.to_owned(), text.to_owned()), color.into())),
                );
            }
        }
        if let Some(kind) = kind {
            options.property_kinds.insert(name.into(), kind);
        }
        if let Some(base) = target
            .filter(|_| transform_components.contains(&property))
            .and_then(|target| TransformAnimator::preview_base_value(target, property))
        {
            options.base_values.insert(name.into(), AnimationValue::Number(base));
        }
    }
    options
}

fn value_kind(property: &FerroProperty) -> Option<ValueKind> {
    let id = property.property_type();
    [
        (TypeId::of::<f64>(), ValueKind::Number),
        (TypeId::of::<f32>(), ValueKind::Number),
        (TypeId::of::<Color>(), ValueKind::Color),
        (TypeId::of::<Point>(), ValueKind::Point),
        (TypeId::of::<Vector>(), ValueKind::Vector),
        (TypeId::of::<Size>(), ValueKind::Size),
        (TypeId::of::<Rect>(), ValueKind::Rect),
        (TypeId::of::<CornerRadius>(), ValueKind::CornerRadius),
        (TypeId::of::<RelativePoint>(), ValueKind::RelativePoint),
        (TypeId::of::<RelativeScalar>(), ValueKind::RelativeScalar),
        (TypeId::of::<Thickness>(), ValueKind::Thickness),
    ]
    .into_iter()
    .find_map(|(property_type, kind)| (property_type == id).then_some(kind))
}

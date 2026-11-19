use super::curve::matches_property;
use crate::{AnimationProgram, ImportOptions, ValueKind};
use ferroui_base::animation::Animation;
use ferroui_base::media::Color;
use ferroui_base::{
    CornerRadius, FerroProperty, Point, Rect, RelativePoint, RelativeScalar, Size, Thickness,
    Vector,
};
use ferroui_designer_core::Document;
use std::any::TypeId;
use std::collections::BTreeSet;

/// Resolve authored setter names against the app's loaded animation, where the
/// property's registered Rust type is authoritative for portable key values.
pub fn native_keyframe_options(
    document: &Document,
    program: &AnimationProgram,
    animation: &Animation,
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
    let setters = document
        .ast()
        .subtree(program.id)
        .filter(|node| node.local_name() == "Setter")
        .filter_map(|node| node.attribute("Property"))
        .collect::<BTreeSet<_>>();
    let mut options = ImportOptions::default();
    for name in setters {
        let mut matches = properties
            .iter()
            .copied()
            .filter(|property| matches_property(property, name));
        if let Some(property) = matches
            .next()
            .filter(|first| matches.all(|other| std::ptr::eq(*first, other)))
        {
            if let Some(kind) = value_kind(property) {
                options.property_kinds.insert(name.into(), kind);
                continue;
            }
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

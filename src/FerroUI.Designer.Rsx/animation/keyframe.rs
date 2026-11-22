use super::syntax::{float, method_suffix, path, span};
use super::{KeySource, Property, TargetType, ValueShape, TRACK_TARGET};
use crate::RsxCatalog;
use ferroui_designer_animation::{native_key_time, Timeline};
use syn::{Expr, ExprMethodCall, Pat};

pub(super) struct MappedKey {
    pub(super) source: KeySource,
    pub(super) owner: Option<TargetType>,
}

pub(super) fn keyframe(
    method: &ExprMethodCall,
    duration: f64,
    catalog: &RsxCatalog,
) -> Option<Vec<MappedKey>> {
    if method.args.len() != 2 {
        return None;
    }
    let cue = float(&method.args[0])?;
    if !(0.0..=1.0).contains(&cue.value) {
        return None;
    }
    let Expr::Closure(closure) = &method.args[1] else {
        return None;
    };
    if closure.inputs.len() != 1 {
        return None;
    }
    let Pat::Ident(frame) = closure.inputs.first()? else {
        return None;
    };
    let Expr::MethodCall(outer) = closure.body.as_ref() else {
        return None;
    };
    let (setter, spline) = if outer.method == "key_spline" {
        let Expr::MethodCall(setter) = outer.receiver.as_ref() else {
            return None;
        };
        if outer.turbofish.is_some() || outer.args.len() != 4 {
            return None;
        }
        let points = [
            float(&outer.args[0])?,
            float(&outer.args[1])?,
            float(&outer.args[2])?,
            float(&outer.args[3])?,
        ];
        (setter, Some(points))
    } else {
        (outer, None)
    };
    if frame.subpat.is_some() || frame.by_ref.is_some() {
        return None;
    }
    let time = native_key_time(cue.value * duration).ok()?;
    let end = span(closure.body.as_ref()).end;
    let mut keys = Vec::new();
    let mut current = setter;
    loop {
        if current.method != "set" || current.turbofish.is_some() || current.args.len() != 2 {
            return None;
        }
        let (getter, property) = read_property(&current.args[0], catalog)?;
        let value = property.shape.read(&current.args[1])?;
        keys.push(MappedKey {
            source: KeySource {
                property: property.name,
                getter,
                time,
                cue,
                value,
                spline,
                spline_method: spline.map(|_| method_suffix(outer)),
                shared: false,
                end,
                frame: method_suffix(method),
                setter: method_suffix(current),
            },
            owner: property.owner,
        });
        match current.receiver.as_ref() {
            Expr::Path(receiver)
                if receiver.qself.is_none() && receiver.path.is_ident(&frame.ident) =>
            {
                break
            }
            Expr::MethodCall(previous) => current = previous,
            _ => return None,
        }
    }
    let shared = keys.len() > 1;
    for key in &mut keys {
        key.source.shared = shared;
    }
    keys.reverse();
    Some(keys)
}

pub(super) fn record(timeline: &mut Timeline, key: &KeySource) -> Option<()> {
    if timeline
        .track(TRACK_TARGET, &key.property)
        .is_some_and(|track| track.keyframes.iter().any(|prior| prior.time == key.time))
    {
        return None;
    }
    timeline
        .record(
            TRACK_TARGET,
            &key.property,
            key.time,
            key.value.value.clone(),
        )
        .ok()?;
    if key.spline.is_some() {
        timeline
            .track_mut(TRACK_TARGET, &key.property)?
            .set_easing(key.time, key.easing())
            .ok()?;
    }
    Some(())
}

pub(crate) fn property(expr: &Expr, catalog: &RsxCatalog) -> Option<Property> {
    read_property(expr, catalog).map(|(_, property)| property)
}

fn read_property(expr: &Expr, catalog: &RsxCatalog) -> Option<(String, Property)> {
    let Expr::Call(call) = expr else { return None };
    if !call.args.is_empty() {
        return None;
    }
    let getter = path(&call.func)?;
    let property = BUILTINS.iter().find(|(path, _, _)| *path == getter).map(|(_, name, shape)| Property {
        name: (*name).into(),
        shape: *shape,
        owner: None,
    }).or_else(|| catalog.animation_property(&getter).cloned())?;
    let absolute = matches!(call.func.as_ref(), Expr::Path(expr) if expr.path.leading_colon.is_some());
    Some((if absolute { format!("::{getter}") } else { getter }, property))
}

pub(crate) const BUILTINS: &[(&str, &str, ValueShape)] = &[
    ("ferroui_base::Visual::opacity_property", "Opacity", ValueShape::NUMBER),
    ("ferroui_base::Visual::render_transform_origin_property", "RenderTransformOrigin", ValueShape::RELATIVE_POINT),
    ("ferroui_base::Visual::effect_property", "Effect", ValueShape::EFFECT),
    ("ferroui_base::layout::Layoutable::width_property", "Width", ValueShape::NUMBER),
    ("ferroui_base::layout::Layoutable::height_property", "Height", ValueShape::NUMBER),
    ("ferroui_base::layout::Layoutable::min_width_property", "MinWidth", ValueShape::NUMBER),
    ("ferroui_base::layout::Layoutable::min_height_property", "MinHeight", ValueShape::NUMBER),
    ("ferroui_base::layout::Layoutable::max_width_property", "MaxWidth", ValueShape::NUMBER),
    ("ferroui_base::layout::Layoutable::max_height_property", "MaxHeight", ValueShape::NUMBER),
    ("ferroui_controls::Border::corner_radius_property", "CornerRadius", ValueShape::CORNER_RADIUS),
    ("ferroui_controls::Border::background_property", "Background", ValueShape::SOLID_BRUSH),
    ("ferroui_controls::Border::border_brush_property", "BorderBrush", ValueShape::SOLID_BRUSH),
    ("ferroui_controls::Panel::background_property", "Background", ValueShape::SOLID_BRUSH),
    ("ferroui_controls::TextBlock::background_property", "Background", ValueShape::SOLID_BRUSH),
    ("ferroui_controls::TextBlock::foreground_property", "Foreground", ValueShape::SOLID_BRUSH),
    ("ferroui_controls::primitives::TemplatedControl::background_property", "Background", ValueShape::SOLID_BRUSH),
    ("ferroui_controls::primitives::TemplatedControl::border_brush_property", "BorderBrush", ValueShape::SOLID_BRUSH),
    ("ferroui_controls::primitives::TemplatedControl::foreground_property", "Foreground", ValueShape::SOLID_BRUSH),
    ("ferroui_controls::shapes::Shape::fill_property", "Fill", ValueShape::SOLID_BRUSH),
    ("ferroui_controls::shapes::Shape::stroke_property", "Stroke", ValueShape::SOLID_BRUSH),
];

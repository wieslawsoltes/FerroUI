use super::{AnimationStyle, FloatSource, KeySource, RsxAnimation, TRACK_TARGET};
use ferroui_designer_animation::{native_key_time, AnimationValue, Easing, FillMode, Timeline};
use ferroui_designer_core::SourceSpan;
use syn::{spanned::Spanned, Expr, ExprCall, ExprMethodCall, ExprPath, Lit, Pat, UnOp};

pub(super) fn style(expr: &Expr) -> Option<AnimationStyle> {
    let (factory, methods) = chain(expr)?;
    let Expr::Lit(selector) = view_argument(factory, "style")? else {
        return None;
    };
    let Lit::Str(selector) = &selector.lit else {
        return None;
    };
    let selector = selector.value();
    if selector.trim().is_empty()
        || selector.starts_with('{')
        || selector.chars().any(|c| {
            c.is_control() && !matches!(c, '\t' | '\n' | '\r')
                || matches!(c, '\u{FFFE}' | '\u{FFFF}')
        })
    {
        return None;
    }
    let clips: Vec<_> = methods
        .into_iter()
        .map(|call| clip(argument(call, "animation")?))
        .collect::<Option<_>>()?;
    (!clips.is_empty()).then_some(AnimationStyle { selector, clips })
}

fn clip(expr: &Expr) -> Option<RsxAnimation> {
    let (factory, methods) = chain(expr)?;
    let duration = milliseconds(view_argument(factory, "animation")?)?;
    let mut timeline = Timeline::new(duration).ok()?;
    timeline.fill = FillMode::None;
    let mut keys = Vec::new();
    for method in methods {
        match method.method.to_string().as_str() {
            "key_frame" => {
                let key = keyframe(method, duration)?;
                if timeline
                    .track(TRACK_TARGET, key.property)
                    .is_some_and(|track| track.keyframes.iter().any(|prior| prior.time == key.time))
                {
                    return None;
                }
                timeline
                    .record(
                        TRACK_TARGET,
                        key.property,
                        key.time,
                        AnimationValue::Number(key.value.value),
                    )
                    .ok()?;
                if let Some(points) = key.spline {
                    let [x1, y1, x2, y2] = points.map(|point| point.value);
                    timeline
                        .track_mut(TRACK_TARGET, key.property)?
                        .set_easing(key.time, Easing::CubicBezier { x1, y1, x2, y2 })
                        .ok()?;
                }
                keys.push(key);
            }
            "fill_mode" => timeline.fill = fill_mode(argument(method, "fill_mode")?)?,
            "delay" => timeline.delay = milliseconds(argument(method, "delay")?)?,
            _ => return None,
        }
    }
    if keys.is_empty() {
        return None;
    }
    let xaml = timeline.to_xaml_authored_keys(TRACK_TARGET).ok()?;
    Some(RsxAnimation {
        declaration: span(expr),
        xaml,
        duration,
        keys,
    })
}

fn keyframe(method: &ExprMethodCall, duration: f64) -> Option<KeySource> {
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
    let Expr::Path(receiver) = setter.receiver.as_ref() else {
        return None;
    };
    if frame.subpat.is_some()
        || frame.by_ref.is_some()
        || !receiver.path.is_ident(&frame.ident)
        || setter.method != "set"
        || setter.turbofish.is_some()
        || setter.args.len() != 2
    {
        return None;
    }
    Some(KeySource {
        property: property(&setter.args[0])?,
        time: native_key_time(cue.value * duration).ok()?,
        cue,
        value: float(&setter.args[1])?,
        spline,
        end: span(closure.body.as_ref()).end,
    })
}

fn chain(mut expr: &Expr) -> Option<(&ExprCall, Vec<&ExprMethodCall>)> {
    let mut methods = Vec::new();
    while let Expr::MethodCall(method) = expr {
        if method.turbofish.is_some() {
            return None;
        }
        methods.push(method);
        expr = method.receiver.as_ref();
    }
    let Expr::Call(factory) = expr else {
        return None;
    };
    methods.reverse();
    Some((factory, methods))
}

fn view_argument<'a>(call: &'a ExprCall, name: &str) -> Option<&'a Expr> {
    let path = path(call.func.as_ref())?;
    if call.args.len() != 1
        || path != format!("ferroui_view::{name}")
            && path != format!("ferroui_view::prelude::{name}")
    {
        return None;
    }
    call.args.first()
}

fn argument<'a>(call: &'a ExprMethodCall, name: &str) -> Option<&'a Expr> {
    (call.method == name && call.args.len() == 1).then(|| &call.args[0])
}

fn fill_mode(expr: &Expr) -> Option<FillMode> {
    match path(expr)?.as_str() {
        "ferroui_base::animation::FillMode::None" => Some(FillMode::None),
        "ferroui_base::animation::FillMode::Forward" => Some(FillMode::Forward),
        "ferroui_base::animation::FillMode::Backward" => Some(FillMode::Backward),
        "ferroui_base::animation::FillMode::Both" => Some(FillMode::Both),
        _ => None,
    }
}

fn property(expr: &Expr) -> Option<&'static str> {
    let Expr::Call(call) = expr else { return None };
    if !call.args.is_empty() {
        return None;
    }
    match path(&call.func)?.as_str() {
        "ferroui_base::Visual::opacity_property" => Some("Opacity"),
        "ferroui_base::layout::Layoutable::width_property" => Some("Width"),
        "ferroui_base::layout::Layoutable::height_property" => Some("Height"),
        "ferroui_base::layout::Layoutable::min_width_property" => Some("MinWidth"),
        "ferroui_base::layout::Layoutable::min_height_property" => Some("MinHeight"),
        "ferroui_base::layout::Layoutable::max_width_property" => Some("MaxWidth"),
        "ferroui_base::layout::Layoutable::max_height_property" => Some("MaxHeight"),
        _ => None,
    }
}

fn path(expr: &Expr) -> Option<String> {
    let Expr::Path(ExprPath {
        qself: None, path, ..
    }) = expr
    else {
        return None;
    };
    if path.segments.iter().any(|part| !part.arguments.is_empty()) {
        return None;
    }
    Some(
        path.segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
    )
}

fn milliseconds(expr: &Expr) -> Option<f64> {
    let Expr::Call(call) = expr else { return None };
    let Expr::Lit(literal) = view_argument(call, "ms")? else {
        return None;
    };
    let Lit::Int(literal) = &literal.lit else {
        return None;
    };
    matches!(literal.suffix(), "" | "u64").then_some(())?;
    Some(literal.base10_parse::<u64>().ok()? as f64 / 1000.0)
}

fn float(expr: &Expr) -> Option<FloatSource> {
    let (expr, minus) = match expr {
        Expr::Unary(unary) if matches!(unary.op, UnOp::Neg(_)) => {
            (unary.expr.as_ref(), Some(span(&unary.op)))
        }
        _ => (expr, None),
    };
    let Expr::Lit(value) = expr else { return None };
    let Lit::Float(literal) = &value.lit else {
        return None;
    };
    let suffix = literal.suffix();
    if !matches!(suffix, "" | "f64") {
        return None;
    }
    let magnitude = literal.base10_parse::<f64>().ok()?;
    magnitude.is_finite().then_some(FloatSource {
        value: if minus.is_some() {
            -magnitude
        } else {
            magnitude
        },
        literal: span(literal),
        minus,
        explicit_type: suffix == "f64",
    })
}

fn span(tokens: &impl Spanned) -> SourceSpan {
    let range = tokens.span().byte_range();
    SourceSpan::new(range.start, range.end)
}

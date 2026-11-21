use super::keyframe::{keyframe, record, MappedKey};
use super::{
    DelaySource, FloatSource, RsxAnimation, TargetType, TRACK_TARGET,
};
use crate::{tree::escape_xml, RsxCatalog};
use ferroui_designer_animation::{Easing, FillMode, Repeat, Timeline};
use ferroui_designer_core::SourceSpan;
use syn::{spanned::Spanned, Expr, ExprCall, ExprMethodCall, ExprPath, Lit, LitInt, UnOp};

pub(crate) fn valid_selector(selector: &str) -> bool {
    !(selector.trim().is_empty()
        || selector.starts_with('{')
        || selector.chars().any(|c| {
            c.is_control() && !matches!(c, '\t' | '\n' | '\r')
                || matches!(c, '\u{FFFE}' | '\u{FFFF}')
        }))
}

pub(crate) fn clip(expr: &Expr, catalog: &RsxCatalog) -> Option<RsxAnimation> {
    let (factory, methods) = chain(expr)?;
    let duration = milliseconds(view_argument(factory, "animation")?)?;
    let mut timeline = Timeline::new(duration).ok()?;
    timeline.fill = FillMode::None;
    let mut keys = Vec::new();
    let mut target_type = None;
    let mut delay = None;
    let first_method = methods.first().map(|method| span(&method.dot_token).start);
    for method in methods {
        match method.method.to_string().as_str() {
            "key_frame" => {
                for MappedKey { source: key, owner } in keyframe(method, duration, catalog)? {
                    if let Some(owner) = owner {
                        if target_type.as_ref().is_some_and(|target| *target != owner) {
                            return None;
                        }
                        target_type = Some(owner);
                    }
                    record(&mut timeline, &key)?;
                    keys.push(key);
                }
            }
            "fill_mode" => timeline.fill = fill_mode(argument(method, "fill_mode")?)?,
            "delay" => {
                let literal = milliseconds_literal(argument(method, "delay")?)?;
                let millis = literal.base10_parse().ok()?;
                timeline.delay = millis as f64 / 1000.0;
                delay = Some(DelaySource { millis, literal: span(literal), explicit_type: literal.suffix() == "u64" });
            }
            "iterations" => timeline.repeat = repeat(argument(method, "iterations")?)?,
            "easing" => timeline.easing = easing(argument(method, "easing")?)?,
            _ => return None,
        }
    }
    let mut xaml = timeline.to_xaml_authored_keys(TRACK_TARGET).ok()?;
    if let Some(target) = target_type.as_ref().or_else(|| builtin_target(&keys, catalog)) {
        target.annotate(&mut xaml);
    }
    Some(RsxAnimation {
        declaration: span(expr),
        xaml,
        duration,
        factory_end: span(factory).end,
        first_method,
        delay,
        target_type,
        keys,
        attachment: None,
    })
}

// Rust getters carry their property types even when the selector only names a
// control. Supply the matching XAML scope for built-in tracks, too.
fn builtin_target<'a>(keys: &[super::KeySource], catalog: &'a RsxCatalog) -> Option<&'a TargetType> {
    let owners = keys.iter()
        .map(|key| catalog.style_field(key.getter.trim_start_matches("::")).map(|field| &field.owner))
        .collect::<Option<Vec<_>>>()?;
    let schema = crate::catalog::schema();
    let type_of = |owner: &TargetType| schema.type_info(&owner.name, owner.namespace.as_deref());
    owners.iter().copied().find(|owner| {
        owners.iter().all(|base| owner == base || type_of(owner).zip(type_of(base))
            .is_some_and(|(target, base)| schema.type_assignable_to(target, base)))
    })
}

impl TargetType {
    fn annotate(&self, xaml: &mut String) {
        let namespace = self.namespace.as_ref().map_or_else(String::new, |uri| {
            format!(" xmlns:motion=\"{}\"", escape_xml(uri, Some('"')))
        });
        let prefix = if self.namespace.is_some() {
            "motion:"
        } else {
            ""
        };
        xaml.insert_str("<Animation".len(), &format!(
            " xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\"{namespace} x:SetterTargetType=\"{prefix}{}\"", self.name));
    }
}

pub(crate) fn chain(mut expr: &Expr) -> Option<(&ExprCall, Vec<&ExprMethodCall>)> {
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

pub(crate) fn view_argument<'a>(call: &'a ExprCall, name: &str) -> Option<&'a Expr> {
    let path = path(call.func.as_ref())?;
    if call.args.len() != 1
        || path != format!("ferroui_view::{name}")
            && path != format!("ferroui_view::prelude::{name}")
    {
        return None;
    }
    call.args.first()
}

pub(crate) fn argument<'a>(call: &'a ExprMethodCall, name: &str) -> Option<&'a Expr> {
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

pub(crate) fn path(expr: &Expr) -> Option<String> {
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
    Some(milliseconds_literal(expr)?.base10_parse::<u64>().ok()? as f64 / 1000.0)
}

fn milliseconds_literal(expr: &Expr) -> Option<&LitInt> {
    let Expr::Call(call) = expr else { return None };
    unsigned_literal(view_argument(call, "ms")?)
}

pub(crate) fn unsigned(expr: &Expr) -> Option<u64> {
    unsigned_literal(expr)?.base10_parse().ok()
}

fn unsigned_literal(expr: &Expr) -> Option<&LitInt> {
    let Expr::Lit(literal) = expr else {
        return None;
    };
    let Lit::Int(literal) = &literal.lit else {
        return None;
    };
    matches!(literal.suffix(), "" | "u64").then_some(literal)
}

fn repeat(expr: &Expr) -> Option<Repeat> {
    if path(expr).as_deref() == Some("ferroui_base::animation::IterationCount::INFINITE") {
        return Some(Repeat::Infinite);
    }
    let Expr::Call(call) = expr else { return None };
    if path(&call.func).as_deref() != Some("ferroui_base::animation::IterationCount::new")
        || call.args.len() != 1
    {
        return None;
    }
    Some(Repeat::Count(unsigned(&call.args[0])?))
}

fn easing(expr: &Expr) -> Option<Easing> {
    let Expr::Lit(literal) = expr else {
        return None;
    };
    let Lit::Str(value) = &literal.lit else {
        return None;
    };
    Easing::parse(&value.value()).ok()
}

pub(super) fn float(expr: &Expr) -> Option<FloatSource> {
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

pub(crate) fn span(tokens: &impl Spanned) -> SourceSpan {
    let range = tokens.span().byte_range();
    SourceSpan::new(range.start, range.end)
}

pub(crate) fn method_suffix(method: &ExprMethodCall) -> SourceSpan {
    SourceSpan::new(span(&method.dot_token).start, span(method).end)
}

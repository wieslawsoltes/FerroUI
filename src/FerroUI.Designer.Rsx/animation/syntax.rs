use super::keyframe::{keyframe, record, MappedKey};
use super::brush::{shared_key, shared_values};
use super::constant_source::{value_in_scope, ConstantReader, ConstantType, ConstantValue};
use super::{
    DelaySource, FloatSource, RsxAnimation, TargetType, TRACK_TARGET,
};
use crate::{local_rust::LocalRust, tree::escape_xml, RsxCatalog};
use ferroui_designer_animation::{AnimationGradientStop, Easing, FillMode, Repeat, Timeline};
use ferroui_designer_core::SourceSpan;
use std::{cell::RefCell, collections::BTreeMap};
use syn::{spanned::Spanned, Expr, ExprCall, ExprMethodCall, ExprPath, Lit, LitInt};

pub(crate) fn valid_selector(selector: &str) -> bool {
    !(selector.trim().is_empty()
        || selector.starts_with('{')
        || selector.chars().any(|c| {
            c.is_control() && !matches!(c, '\t' | '\n' | '\r')
                || matches!(c, '\u{FFFE}' | '\u{FFFF}')
        }))
}

pub(crate) fn authored_clip(expr: &Expr, catalog: &RsxCatalog, bindings: &LocalRust) -> Option<RsxAnimation> {
    let scope = bindings.source_scope(expr)?;
    clip(
        expr,
        catalog,
        |expression| bindings.path(expression),
        |expression| shared_values(expression, scope),
        |path, ty| value_in_scope(path, ty, scope),
    )
}

pub(super) fn clip(
    expr: &Expr,
    catalog: &RsxCatalog,
    resolve: impl Fn(&Expr) -> Option<String>,
    read_stops: impl Fn(&Expr) -> Option<Vec<AnimationGradientStop>>,
    read_constant: impl Fn(&Expr, ConstantType) -> Option<ConstantValue>,
) -> Option<RsxAnimation> {
    // Structural edits reparse the clip by itself. Retain the imported paths
    // resolved in this accepted source scope for that detached check.
    let imports = RefCell::new(BTreeMap::new());
    let stops = RefCell::new(BTreeMap::new());
    let constants = RefCell::new(BTreeMap::new());
    let resolve = |expression: &Expr| {
        let resolved = resolve(expression)?;
        let written = authored_path(expression)?;
        if written.trim_start_matches("::") != resolved {
            imports.borrow_mut().insert(written, resolved.clone());
        }
        Some(resolved)
    };
    // Detached edits use the same authored constant values while replacing only
    // the selected use. They never patch source outside the animation builder.
    let read_stops = |expression: &Expr| {
        let name = shared_key(expression)?;
        let values = read_stops(expression)?;
        stops.borrow_mut().insert(name, values.clone());
        Some(values)
    };
    let read_constant = |expression: &Expr, ty| {
        let name = authored_path(expression)?;
        let value = read_constant(expression, ty)?;
        constants.borrow_mut().insert((name, ty), value.clone());
        Some(value)
    };
    let constant_reader = ConstantReader(&read_constant);
    let (factory, methods) = chain(expr)?;
    let duration = milliseconds(view_argument_with(factory, "animation", &resolve)?, &resolve)?;
    let mut timeline = Timeline::new(duration).ok()?;
    timeline.fill = FillMode::None;
    let mut keys = Vec::new();
    let mut target_type = None;
    let mut delay = None;
    let first_method = methods.first().map(|method| span(&method.dot_token).start);
    for method in methods {
        match method.method.to_string().as_str() {
            "key_frame" => {
                for MappedKey { source: key, owner } in
                    keyframe(method, duration, catalog, &resolve, &read_stops, &constant_reader)?
                {
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
            "fill_mode" => timeline.fill = fill_mode(argument(method, "fill_mode")?, &resolve)?,
            "delay" => {
                let literal = milliseconds_literal(argument(method, "delay")?, &resolve)?;
                let millis = literal.base10_parse().ok()?;
                timeline.delay = millis as f64 / 1000.0;
                delay = Some(DelaySource { millis, literal: span(literal), explicit_type: literal.suffix() == "u64" });
            }
            "iterations" => timeline.repeat = repeat(argument(method, "iterations")?, &resolve)?,
            "easing" => timeline.easing = easing(argument(method, "easing")?)?,
            _ => return None,
        }
    }
    let mut xaml = timeline.to_xaml_authored_keys(TRACK_TARGET).ok()?;
    if let Some(target) = &target_type {
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
        imports: imports.into_inner(),
        shared_stops: stops.into_inner(),
        constants: constants.into_inner(),
        attachment: None,
    })
}

pub(crate) fn scope_untyped_clip(clip: &mut RsxAnimation, catalog: &RsxCatalog) {
    if clip.target_type.is_none() {
        if let Some(target) = builtin_target(clip, catalog) {
            target.annotate(&mut clip.xaml);
        }
    }
}

// Rust getters carry their property types even when the selector only names a
// control. Supply the matching XAML scope for built-in tracks, too.
fn builtin_target<'a>(clip: &RsxAnimation, catalog: &'a RsxCatalog) -> Option<&'a TargetType> {
    let owners = clip.keys.iter()
        .map(|key| catalog.style_field(clip.resolved_path(&key.getter)).map(|field| &field.owner))
        .collect::<Option<Vec<_>>>()?;
    let schema = crate::catalog::schema();
    let type_of = |owner: &TargetType| schema.type_info(&owner.name, owner.namespace.as_deref());
    owners.iter().copied().find(|owner| {
        owners.iter().all(|base| owner == base || type_of(owner).zip(type_of(base))
            .is_some_and(|(target, base)| schema.type_assignable_to(target, base)))
    })
}

impl RsxAnimation {
    pub(super) fn resolved_path<'a>(&'a self, path: &'a str) -> &'a str {
        self.imports.get(path).map_or_else(|| path.trim_start_matches("::"), String::as_str)
    }
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
        if !method.attrs.is_empty() || method.turbofish.is_some() {
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
    view_argument_with(call, name, &path)
}

pub(crate) fn view_argument_with<'a>(
    call: &'a ExprCall,
    name: &str,
    resolve: &impl Fn(&Expr) -> Option<String>,
) -> Option<&'a Expr> {
    let path = resolve(call.func.as_ref())?;
    if !call.attrs.is_empty() || call.args.len() != 1
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

fn fill_mode(expr: &Expr, resolve: &impl Fn(&Expr) -> Option<String>) -> Option<FillMode> {
    match resolve(expr)?.as_str() {
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

pub(super) fn authored_path(expr: &Expr) -> Option<String> {
    let Expr::Path(site) = expr else { return None };
    if !site.attrs.is_empty() {
        return None;
    }
    let path = path(expr)?;
    Some(if site.path.leading_colon.is_some() { format!("::{path}") } else { path })
}

fn milliseconds(expr: &Expr, resolve: &impl Fn(&Expr) -> Option<String>) -> Option<f64> {
    Some(milliseconds_literal(expr, resolve)?.base10_parse::<u64>().ok()? as f64 / 1000.0)
}

fn milliseconds_literal<'a>(expr: &'a Expr, resolve: &impl Fn(&Expr) -> Option<String>) -> Option<&'a LitInt> {
    let Expr::Call(call) = expr else { return None };
    unsigned_literal(view_argument_with(call, "ms", resolve)?)
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

fn repeat(expr: &Expr, resolve: &impl Fn(&Expr) -> Option<String>) -> Option<Repeat> {
    if resolve(expr).as_deref() == Some("ferroui_base::animation::IterationCount::INFINITE") {
        return Some(Repeat::Infinite);
    }
    let Expr::Call(call) = expr else { return None };
    if resolve(&call.func).as_deref() != Some("ferroui_base::animation::IterationCount::new")
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

pub(crate) fn float(expr: &Expr) -> Option<FloatSource> {
    ConstantReader::LITERALS.float(expr)
}

pub(crate) fn span(tokens: &impl Spanned) -> SourceSpan {
    let range = tokens.span().byte_range();
    SourceSpan::new(range.start, range.end)
}

pub(super) fn unparen(mut expression: &Expr) -> &Expr {
    loop {
        expression = match expression {
            Expr::Paren(paren) if paren.attrs.is_empty() => &paren.expr,
            Expr::Group(group) if group.attrs.is_empty() => &group.expr,
            _ => return expression,
        };
    }
}

pub(super) fn rc_interface_cast(
    ty: &syn::Type, interface: &str, resolve: &impl Fn(&Expr) -> Option<String>,
) -> bool {
    use syn::{GenericArgument, PathArguments, Type, TypeParamBound};
    let Type::Path(path) = ty else { return false };
    let Some(rc) = path.path.segments.last() else { return false };
    let PathArguments::AngleBracketed(arguments) = &rc.arguments else { return false };
    let Some(GenericArgument::Type(Type::TraitObject(object))) = arguments.args.first() else { return false };
    let Some(TypeParamBound::Trait(bound)) = object.bounds.first() else { return false };
    let resolve_type = |path: syn::Path| resolve(&Expr::Path(ExprPath { attrs: vec![], qself: None, path }));
    let mut container = path.path.clone();
    container.segments.last_mut().expect("Rc type").arguments = PathArguments::None;
    path.qself.is_none() && arguments.args.len() == 1 && object.bounds.len() == 1
        && matches!(resolve_type(container).as_deref(), Some("std::rc::Rc" | "alloc::rc::Rc"))
        && resolve_type(bound.path.clone()).as_deref() == Some(interface)
}

pub(crate) fn method_suffix(method: &ExprMethodCall) -> SourceSpan {
    SourceSpan::new(span(&method.dot_token).start, span(method).end)
}

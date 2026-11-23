use super::RsxPropertyTransition;
use crate::animation::syntax::{span, unsigned};
use crate::catalog;
use crate::local_rust::LocalRust;
use crate::registered::Class;
use crate::{RsxCatalog, RsxStyleField};
use ferroui_designer_animation::{Easing, PropertyTransitionTiming, XamlTransitionTarget};
use syn::{Expr, ExprCall, Lit};

const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

pub(super) fn collection<'a>(value: &'a Expr, bindings: &LocalRust) -> Option<&'a Expr> {
    let Expr::Call(call) = value else { return None };
    (view_function(call, bindings, "transitions") && call.args.len() == 1).then(|| &call.args[0])
}

pub(super) fn entry(
    declaration: &Expr,
    target: &Class,
    catalog: &RsxCatalog,
    bindings: &LocalRust,
) -> Option<RsxPropertyTransition> {
    let Expr::Call(call) = declaration else {
        return None;
    };
    let easing = if view_function(call, bindings, "transition") && call.args.len() == 2 {
        None
    } else if view_function(call, bindings, "transition_eased") && call.args.len() == 3 {
        Some(read_easing(&call.args[2], bindings)?)
    } else {
        return None;
    };
    let Expr::Call(getter) = &call.args[0] else {
        return None;
    };
    if !getter.attrs.is_empty() || !getter.args.is_empty() {
        return None;
    }
    let field = catalog.style_field(&bindings.path(&getter.func)?)?;
    let (property, target) = property_target(target, field)?;
    let duration = duration(&call.args[1], bindings)?;
    let markup = target
        .to_xaml(&PropertyTransitionTiming {
            duration: Some(duration),
            easing: easing.clone(),
            ..Default::default()
        })
        .ok()?;
    Some(RsxPropertyTransition {
        source: span(declaration),
        property,
        type_name: target.type_name,
        duration,
        easing,
        markup,
    })
}

fn view_function(call: &ExprCall, bindings: &LocalRust, name: &str) -> bool {
    call.attrs.is_empty()
        && bindings.path(&call.func).is_some_and(|path| {
            path.strip_prefix("ferroui_view::").is_some_and(|path| {
                path == name
                    || name != "transition_eased" && path.strip_prefix("prelude::") == Some(name)
            })
        })
}

fn duration(expression: &Expr, bindings: &LocalRust) -> Option<f64> {
    let Expr::Call(call) = expression else {
        return None;
    };
    if !call.attrs.is_empty() || call.args.len() != 1 {
        return None;
    }
    let units = match bindings.path(&call.func)?.as_str() {
        "ferroui_view::ms"
        | "ferroui_view::prelude::ms"
        | "std::time::Duration::from_millis"
        | "core::time::Duration::from_millis" => 1000.0,
        "std::time::Duration::from_secs" | "core::time::Duration::from_secs" => 1.0,
        _ => return None,
    };
    Some(unsigned(&call.args[0])? as f64 / units)
}

fn read_easing(expression: &Expr, bindings: &LocalRust) -> Option<Easing> {
    if let Expr::Call(call) = expression {
        if call.attrs.is_empty() && call.args.is_empty() {
            let path = bindings.path(&call.func)?;
            let name = path
                .strip_prefix("ferroui_base::animation::easings::")?
                .strip_suffix("::new")?;
            let easing = Easing::parse(name).ok()?;
            return matches!(easing, Easing::Named(_) | Easing::Linear).then_some(easing);
        }
    }
    let Expr::MethodCall(method) = expression else {
        return None;
    };
    if !method.attrs.is_empty()
        || method.method != "unwrap"
        || !method.args.is_empty()
        || method.turbofish.is_some()
    {
        return None;
    }
    let Expr::Call(call) = method.receiver.as_ref() else {
        return None;
    };
    if bindings.path(&call.func)?.as_str() != "ferroui_base::animation::easings::Easing::parse"
        || !call.attrs.is_empty()
        || call.args.len() != 1
    {
        return None;
    }
    let Expr::Lit(literal) = &call.args[0] else {
        return None;
    };
    let Lit::Str(value) = &literal.lit else {
        return None;
    };
    let value = value.value();
    let easing = Easing::parse(&value).ok()?;
    match &easing {
        Easing::Named(name) if name == &value => Some(easing),
        Easing::Linear if value == "LinearEasing" => Some(easing),
        Easing::CubicBezier { .. } if value.contains(',') => Some(easing),
        _ => None,
    }
}

fn property_target(
    target: &Class,
    field: &RsxStyleField,
) -> Option<(String, XamlTransitionTarget)> {
    let schema = catalog::schema();
    let base = target.transition_base()?;
    let owner = &field.owner;
    let owner_name = &owner.name;
    let member = &field.name;
    let mut namespaces = vec![("x".into(), XAML.into())];
    let (name, reference_owner) = if let Some(namespace) = &owner.namespace {
        if target.namespace() != Some(namespace) || target.name() != owner_name {
            return None;
        }
        namespaces.push(("property".into(), namespace.clone()));
        (member.clone(), format!("property:{owner_name}"))
    } else {
        let info = schema.property(owner_name, None, member)?;
        let attached = info.attached;
        if info.read_only || info.event || info.style_settable == Some(false) {
            return None;
        }
        if !attached
            && !schema.type_assignable_to(
                schema.type_info(base, None)?,
                schema.type_info(owner_name, None)?,
            )
        {
            return None;
        }
        let name = if attached {
            format!("{owner_name}.{member}")
        } else {
            member.clone()
        };
        (name, owner_name.clone())
    };
    let type_name = match field.type_name.as_str() {
        "f64" => "DoubleTransition",
        "f32" => "FloatTransition",
        "i32" => "IntegerTransition",
        "bool" => "BoolTransition",
        "Thickness" => "ThicknessTransition",
        "CornerRadius" => "CornerRadiusTransition",
        "Color" => "ColorTransition",
        "Option<Rc<dyn IBrush>>" => "BrushTransition",
        "Size" => "SizeTransition",
        "Point" => "PointTransition",
        "Vector" => "VectorTransition",
        _ => return None,
    };
    Some((
        name,
        XamlTransitionTarget {
            property: format!("{{x:Static {reference_owner}.{member}Property}}"),
            type_name,
            namespaces,
        },
    ))
}

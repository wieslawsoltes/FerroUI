use super::{mapped_easing, RouteSource, RsxStateGroup};
use crate::animation::{
    syntax::{argument, chain, path, span, unsigned, view_argument},
    view_property, ValueSource,
};
use ferroui_designer_animation::{
    native_key_time, Easing, StateSetter, StateTransition, VisualState, VisualStateGroup,
};
use ferroui_designer_core::SourceSpan;
use syn::{Expr, ExprMethodCall, Lit};

pub(super) fn group(expr: &Expr, catalog: &crate::RsxCatalog) -> Option<RsxStateGroup> {
    let (factory, methods) = chain(expr)?;
    let mut group = RsxStateGroup {
        model: VisualStateGroup {
            name: text(view_argument(factory, "visual_state_group")?)?.value(),
            owner: 1,
            states: Vec::new(),
            transitions: Vec::new(),
        },
        declaration: span(expr),
        routes: Vec::new(),
        values: Vec::new(),
    };
    for method in methods {
        if let Some(declaration) = argument(method, "state") {
            let (factory, methods) = chain(declaration)?;
            let name = text(view_argument(factory, "visual_state")?)?.value();
            let mut state = VisualState {
                name,
                setters: Vec::new(),
            };
            let mut values = Vec::new();
            for setter in methods {
                let (setter, value) = state_setter(setter, catalog)?;
                values.push(value);
                state.setters.push(setter);
            }
            group.model.states.push(state);
            group.values.push(values);
        } else {
            let (route, source) = transition(method)?;
            group.model.transitions.push(route);
            group.routes.push(source);
        }
    }
    group.model.validate().ok()?;
    Some(group)
}

fn state_setter(
    method: &ExprMethodCall,
    catalog: &crate::RsxCatalog,
) -> Option<(StateSetter, ValueSource)> {
    if method.turbofish.is_some() {
        return None;
    }
    let args = method.args.iter().collect::<Vec<_>>();
    let (target, property, value) = match (method.method.to_string().as_str(), args.as_slice()) {
        ("set", [property, value]) => (String::new(), *property, *value),
        ("set_on", [target, property, value]) => (text(target)?.value(), *property, *value),
        _ => return None,
    };
    let property = view_property(property, catalog)?;
    let value = property.shape.read(value)?;
    Some((
        StateSetter {
            target,
            property: property.name,
            value: value.value.to_xaml(),
        },
        value,
    ))
}

fn transition(method: &ExprMethodCall) -> Option<(StateTransition, RouteSource)> {
    if method.method != "transition" || method.turbofish.is_some() || method.args.len() != 4 {
        return None;
    }
    let from = match &method.args[0] {
        Expr::Call(call) if path(&call.func).as_deref() == Some("Some") && call.args.len() == 1 => {
            Some(text(&call.args[0])?.value())
        }
        value if path(value).as_deref() == Some("None") => None,
        _ => return None,
    };
    let to = text(&method.args[1])?.value();
    if from.as_deref() == Some(to.as_str()) {
        return None;
    }
    let Expr::Call(duration) = &method.args[2] else {
        return None;
    };
    let milliseconds = view_argument(duration, "ms")?;
    let value = unsigned(milliseconds)?;
    let duration = native_key_time(value as f64 / 1000.0).ok()?;
    let explicit_u64 = matches!(milliseconds, Expr::Lit(literal) if matches!(&literal.lit, Lit::Int(value) if value.suffix() == "u64"));
    let literal = text(&method.args[3])?;
    let expression = literal.value();
    let easing = Easing::parse(&expression).ok()?;
    if matches!(easing, Easing::CubicBezier { .. }) && !expression.contains(',') {
        return None;
    }
    mapped_easing(&easing).then_some((
        StateTransition {
            from,
            to,
            duration,
            easing,
        },
        RouteSource {
            declaration: SourceSpan::new(
                method.method.span().byte_range().start,
                method.paren_token.span.close().byte_range().end,
            ),
            milliseconds: span(milliseconds),
            explicit_u64,
            easing: span(literal),
        },
    ))
}

fn text(expr: &Expr) -> Option<&syn::LitStr> {
    let Expr::Lit(literal) = expr else {
        return None;
    };
    let Lit::Str(value) = &literal.lit else {
        return None;
    };
    Some(value)
}

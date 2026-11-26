//! The literal subset shared by the Rust declaration and native behavior inspector.
use crate::animation::syntax::{path, span};
use crate::local_rust::LocalRust;
use ferroui_base::data::core::parsers::BindingExpressionGrammar;
use ferroui_designer_animation::{BehaviorAction, BehaviorTrigger};
use ferroui_designer_core::SourceSpan;
use syn::{Expr, ExprArray, ExprCall, ExprLit, Lit};

const MODULE: &str = "ferroui_view::behavior::";
const COMPARISONS: &[&str] = &[
    "Equal",
    "NotEqual",
    "LessThan",
    "LessThanOrEqual",
    "GreaterThan",
    "GreaterThanOrEqual",
];

pub(super) struct Arguments {
    pub(super) span: SourceSpan,
    pub(super) operands: Vec<SourceSpan>,
}

impl Arguments {
    fn new(expr: &Expr, args: &[&Expr]) -> Self {
        Self {
            span: span(expr),
            operands: args.iter().map(|expr| span(*expr)).collect(),
        }
    }
}

pub(super) fn trigger<'a>(
    expr: &'a Expr,
    rust: &LocalRust,
) -> Option<(BehaviorTrigger, Arguments, &'a ExprArray)> {
    let (kind, call) = call(expr, rust)?;
    let args = call.args.iter().collect::<Vec<_>>();
    let (trigger, actions) = match (kind.as_str(), args.as_slice()) {
        ("event", [event, source, Expr::Array(actions)]) => (
            BehaviorTrigger::Event {
                event: text(event)?,
                source: optional_name(source)?,
            },
            actions,
        ),
        ("data", [binding, comparison, value, revert, Expr::Array(actions)]) => {
            let comparison = rust.path(comparison)?;
            let comparison = comparison
                .strip_prefix("ferroui_view::behavior::Comparison::")
                .or_else(|| {
                    comparison.strip_prefix("ferroui_xaml_interactivity::ComparisonConditionType::")
                })?;
            let Expr::Lit(ExprLit {
                lit: Lit::Bool(revert),
                ..
            }) = revert
            else {
                return None;
            };
            (
                BehaviorTrigger::Data {
                    binding: binding_markup(&text(binding)?)?,
                    comparison: COMPARISONS
                        .contains(&comparison)
                        .then(|| comparison.to_owned())?,
                    value: literal_markup(&text(value)?),
                    revert_on_false: revert.value,
                },
                actions,
            )
        }
        _ => return None,
    };
    Some((
        trigger,
        Arguments::new(expr, &args[..args.len() - 1]),
        actions,
    ))
}

pub(super) fn action(expr: &Expr, rust: &LocalRust) -> Option<(BehaviorAction, Arguments)> {
    let (kind, call) = call(expr, rust)?;
    let args = call.args.iter().collect::<Vec<_>>();
    let value = match (kind.as_str(), args.as_slice()) {
        ("change_property", [target, property, value]) => BehaviorAction::ChangeProperty {
            target: optional_name(target)?,
            property: text(property)?,
            value: literal_markup(&text(value)?),
        },
        ("invoke_command", [command, parameter]) => BehaviorAction::InvokeCommand {
            command: binding_markup(&text(command)?)?,
            parameter: optional_text(parameter)?.map(|value| literal_markup(&value)),
        },
        ("call_method", [target, method]) => BehaviorAction::CallMethod {
            target: optional_name(target)?,
            method: text(method)?,
        },
        ("add_class", [target, name]) => BehaviorAction::AddClass {
            target: optional_name(target)?,
            class: name_text(name)?,
        },
        ("remove_class", [target, name]) => BehaviorAction::RemoveClass {
            target: optional_name(target)?,
            class: name_text(name)?,
        },
        _ => return None,
    };
    value.to_xaml().ok()?;
    Some((value, Arguments::new(expr, &args)))
}

fn call<'a>(expr: &'a Expr, rust: &LocalRust) -> Option<(String, &'a ExprCall)> {
    let Expr::Call(call) = expr else { return None };
    let resolved = rust.path(&call.func)?;
    Some((resolved.strip_prefix(MODULE)?.into(), call))
}

fn text(expr: &Expr) -> Option<String> {
    let Expr::Lit(ExprLit {
        lit: Lit::Str(text),
        ..
    }) = expr
    else {
        return None;
    };
    Some(text.value())
}

fn name_text(expr: &Expr) -> Option<String> {
    let value = text(expr)?;
    is_name(&value).then_some(value)
}

fn optional_text(expr: &Expr) -> Option<Option<String>> {
    match expr {
        Expr::Call(call) if is_option(&call.func, "Some") && call.args.len() == 1 => {
            Some(Some(text(&call.args[0])?))
        }
        value if is_option(value, "None") => Some(None),
        _ => None,
    }
}

fn is_option(expr: &Expr, variant: &str) -> bool {
    path(expr).is_some_and(|path| {
        path == variant
            || [
                "Option::",
                "std::option::Option::",
                "core::option::Option::",
            ]
            .iter()
            .any(|prefix| path.strip_prefix(prefix) == Some(variant))
    })
}

fn optional_name(expr: &Expr) -> Option<Option<String>> {
    let value = optional_text(expr)?;
    match value {
        Some(name) => Some(Some(name_markup(&name)?)),
        None => Some(None),
    }
}

pub(super) fn name_markup(name: &str) -> Option<String> {
    if !is_name(name) {
        return None;
    }
    let path = format!("#{name}");
    BindingExpressionGrammar::parse(&path).ok()?;
    Some(format!("{{Binding {path}}}"))
}

pub(super) fn control_name(value: &str) -> Option<&str> {
    let value = value.trim();
    let name = value
        .strip_prefix("{Binding #")
        .or_else(|| value.strip_prefix("{ReflectionBinding #"))
        .and_then(|value| value.strip_suffix('}'))
        .unwrap_or(value);
    is_name(name).then_some(name)
}

pub(super) fn is_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_alphanumeric() || matches!(character, '_' | '-'))
}

pub(super) fn literal_markup(value: &str) -> String {
    if value.starts_with('{') {
        format!("{{}}{value}")
    } else {
        value.to_owned()
    }
}

fn binding_markup(path: &str) -> Option<String> {
    let path = data_context_path(path.trim())?;
    Some(format!("{{ReflectionBinding {path}}}"))
}

pub(super) fn binding_path(value: &str) -> Option<&str> {
    let value = value.trim();
    let path = value
        .strip_prefix("{ReflectionBinding ")
        .or_else(|| value.strip_prefix("{Binding "))
        .and_then(|value| value.strip_suffix('}'))
        .unwrap_or(value);
    let path = path.trim().strip_prefix("Path=").unwrap_or(path.trim());
    data_context_path(path)
}

fn data_context_path(path: &str) -> Option<&str> {
    if path.is_empty()
        || !path.chars().all(|character| {
            character.is_alphanumeric() || matches!(character, '_' | '.' | '[' | ']')
        })
    {
        return None;
    }
    BindingExpressionGrammar::parse(path).ok()?;
    Some(path)
}

pub(super) fn comparison(name: &str) -> Option<String> {
    COMPARISONS
        .contains(&name)
        .then(|| format!("{MODULE}Comparison::{name}"))
}

pub(super) fn factory(name: &str) -> String {
    format!("{MODULE}{name}")
}

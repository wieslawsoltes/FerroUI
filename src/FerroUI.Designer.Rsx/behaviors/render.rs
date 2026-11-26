use super::syntax;
use ferroui_designer_animation::{BehaviorAction, BehaviorModel, BehaviorTrigger};

pub(crate) fn normalize(model: &mut BehaviorModel) -> Result<(), String> {
    model.id = None;
    model.owner = 1;
    match &mut model.trigger {
        BehaviorTrigger::Event { source, .. } => normalize_target(source)?,
        BehaviorTrigger::Data { binding, value, .. } => {
            *binding = markup_binding(binding)?;
            *value = syntax::literal_markup(literal(value)?);
        }
        _ => {}
    }
    trigger(&model.trigger)?;
    for item in &mut model.actions {
        match item {
            BehaviorAction::ChangeProperty { target, value, .. } => {
                normalize_target(target)?;
                *value = syntax::literal_markup(literal(value)?);
            }
            BehaviorAction::CallMethod { target, .. }
            | BehaviorAction::AddClass { target, .. }
            | BehaviorAction::RemoveClass { target, .. } => normalize_target(target)?,
            BehaviorAction::InvokeCommand { command, parameter } => {
                *command = markup_binding(command)?;
                if let Some(value) = parameter {
                    *value = syntax::literal_markup(literal(value)?);
                }
            }
            _ => {}
        }
        action(item)?;
    }
    model.to_xaml().map_err(|error| error.to_string())?;
    Ok(())
}

pub(crate) fn declaration(
    model: &BehaviorModel,
    indent: &str,
    eol: &str,
) -> Result<String, String> {
    let (kind, operands) = trigger(&model.trigger)?;
    let actions = action_array(&model.actions, indent, eol)?;
    Ok(format!(
        "{}({}, {actions})",
        syntax::factory(kind),
        operands.join(", ")
    ))
}

pub(super) fn action_array(
    actions: &[BehaviorAction],
    indent: &str,
    eol: &str,
) -> Result<String, String> {
    let mut items = String::new();
    for item in actions {
        let action = declaration_action(item)?;
        items.push_str(&format!("{eol}{indent}    {action},"));
    }
    if !items.is_empty() {
        items.push_str(eol);
        items.push_str(indent);
    }
    Ok(format!("[{items}]"))
}

pub(super) fn declaration_action(action_model: &BehaviorAction) -> Result<String, String> {
    let (kind, operands) = action(action_model)?;
    Ok(format!(
        "{}({})",
        syntax::factory(kind),
        operands.join(", ")
    ))
}

pub(super) fn trigger(model: &BehaviorTrigger) -> Result<(&'static str, Vec<String>), String> {
    Ok(match model {
        BehaviorTrigger::Event { event, source } => ("event", vec![string(event), named(source)?]),
        BehaviorTrigger::Data {
            binding,
            comparison,
            value,
            revert_on_false,
        } => (
            "data",
            vec![
                string(
                    syntax::binding_path(binding).ok_or("Enter a simple Rust data-context path")?,
                ),
                syntax::comparison(comparison).ok_or("Choose a native data comparison")?,
                string(literal(value)?),
                revert_on_false.to_string(),
            ],
        ),
        BehaviorTrigger::Custom { .. } => {
            return Err("Choose a routed event or data condition for Rust".into())
        }
    })
}

pub(super) fn action(model: &BehaviorAction) -> Result<(&'static str, Vec<String>), String> {
    Ok(match model {
        BehaviorAction::ChangeProperty {
            target,
            property,
            value,
        } => (
            "change_property",
            vec![named(target)?, string(property), string(literal(value)?)],
        ),
        BehaviorAction::InvokeCommand { command, parameter } => (
            "invoke_command",
            vec![
                string(syntax::binding_path(command).ok_or("Enter a simple Rust command path")?),
                optional(parameter.as_deref().map(literal).transpose()?),
            ],
        ),
        BehaviorAction::CallMethod { target, method } => {
            ("call_method", vec![named(target)?, string(method)])
        }
        BehaviorAction::AddClass { target, class }
        | BehaviorAction::RemoveClass { target, class } => {
            if !syntax::is_name(class) {
                return Err("Enter a literal style class name".into());
            }
            let factory = if matches!(model, BehaviorAction::AddClass { .. }) {
                "add_class"
            } else {
                "remove_class"
            };
            (factory, vec![named(target)?, string(class)])
        }
        BehaviorAction::Custom { .. } | BehaviorAction::Markup { .. } => {
            return Err("Choose a native action with a mapped Rust declaration".into())
        }
    })
}

fn literal(value: &str) -> Result<&str, String> {
    if let Some(escaped) = value.strip_prefix("{}") {
        return Ok(escaped);
    }
    if value.starts_with('{') {
        return Err("Enter a literal Rust action or comparison value".into());
    }
    Ok(value)
}

fn named(value: &Option<String>) -> Result<String, String> {
    Ok(optional(value.as_deref().map(control_name).transpose()?))
}

fn normalize_target(value: &mut Option<String>) -> Result<(), String> {
    if let Some(value) = value {
        *value = syntax::name_markup(control_name(value)?)
            .ok_or("Enter a literal control name for the Rust behavior")?;
    }
    Ok(())
}

fn control_name(value: &str) -> Result<&str, String> {
    syntax::control_name(value)
        .ok_or_else(|| "Enter a literal control name for the Rust behavior".into())
}

fn optional(value: Option<&str>) -> String {
    value.map_or_else(|| "None".into(), |value| format!("Some({})", string(value)))
}

fn string(value: &str) -> String {
    proc_macro2::Literal::string(value).to_string()
}

fn markup_binding(value: &str) -> Result<String, String> {
    let path = syntax::binding_path(value).ok_or("Enter a simple Rust data-context path")?;
    Ok(format!("{{ReflectionBinding {path}}}"))
}

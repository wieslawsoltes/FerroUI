use super::{
    choice, choice_field, labeled_field, optional, section, text, AutomationProperties, StackPanel,
};
use ferroui_designer_animation::{BehaviorAction, BehaviorTrigger};

pub(super) fn trigger(panel: &StackPanel) -> impl Fn() -> Result<BehaviorTrigger, String> {
    section(panel, "Trigger");
    let trigger = choice_field(
        panel,
        Some("BehaviorTriggerKind"),
        "When",
        "Event",
        &["Event", "Data condition", "Custom"],
    );
    let custom_trigger = labeled_field(panel, Some("BehaviorTriggerType"), "Trigger type", "");
    custom_trigger
        .input
        .set_placeholder_text(Some("CustomTrigger"));
    custom_trigger.row.set_is_visible(false);
    let event = labeled_field(panel, Some("BehaviorTriggerSource"), "Event", "Click");
    let source = labeled_field(panel, Some("BehaviorEventSource"), "Source", "");
    source.input.set_placeholder_text(Some("Selected control"));
    let comparison = labeled_field(panel, Some("BehaviorComparison"), "Comparison", "Equal");
    comparison.row.set_is_visible(false);
    let restore = labeled_field(
        panel,
        Some("BehaviorRevertOnFalse"),
        "Revert on false",
        "False",
    );
    restore.row.set_is_visible(false);
    let compare_value = labeled_field(
        panel,
        Some("BehaviorComparisonValue"),
        "Compare value",
        "True",
    );
    compare_value.row.set_is_visible(false);
    let event_source_row = source.row.clone();
    let restore_row = restore.row.clone();
    let weak_trigger = trigger.downgrade();
    let source_label = event.label.clone();
    let source_input = event.input.clone();
    let custom_trigger_row = custom_trigger.row.clone();
    let comparison_row = comparison.row.clone();
    let compare_value_row = compare_value.row.clone();
    trigger.selection_changed(move |_, _| {
        if let Some(trigger) = weak_trigger.upgrade() {
            let selected = choice(&trigger);
            let label = match selected.as_str() {
                "Data condition" => "Binding",
                "Custom" => "Properties JSON",
                _ => "Event",
            };
            source_label.set_text(Some(label));
            AutomationProperties::set_name(&source_input, Some(label));
            custom_trigger_row.set_is_visible(selected == "Custom");
            event_source_row.set_is_visible(selected == "Event");
            restore_row.set_is_visible(selected == "Data condition");
            comparison_row.set_is_visible(selected == "Data condition");
            compare_value_row.set_is_visible(selected == "Data condition");
        }
    });
    move || {
        Ok(match choice(&trigger).as_str() {
            "Event" => BehaviorTrigger::Event {
                event: text(&event.input),
                source: optional(&source.input),
            },
            "Data condition" => BehaviorTrigger::Data {
                binding: text(&event.input),
                comparison: text(&comparison.input),
                value: text(&compare_value.input),
                revert_on_false: parse_bool(&text(&restore.input))?,
            },
            "Custom" => BehaviorTrigger::Custom {
                type_name: text(&custom_trigger.input),
                properties: serde_json::from_str(&text(&event.input)).map_err(|e| e.to_string())?,
            },
            _ => return Err("Choose a trigger".into()),
        })
    }
}

pub(super) fn action(
    panel: &StackPanel,
    named: bool,
    initial: Option<&BehaviorAction>,
) -> impl Fn() -> Result<BehaviorAction, String> {
    let (selected, destination, member, argument) = action_values(initial);
    let choices = [
        "Change property",
        "Invoke command",
        "Call method",
        "Add class",
        "Remove class",
        "Custom",
    ];
    let available = if initial.is_some() {
        &choices[..5]
    } else {
        &choices
    };
    let kind = choice_field(
        panel,
        named.then_some("BehaviorActionKind"),
        "Do",
        selected,
        available,
    );
    let custom_action = labeled_field(
        panel,
        named.then_some("BehaviorActionType"),
        "Action type",
        "",
    );
    custom_action
        .input
        .set_placeholder_text(Some("CustomAction"));
    custom_action.row.set_is_visible(false);
    let target = labeled_field(
        panel,
        named.then_some("BehaviorActionTarget"),
        "Target (Name or binding)",
        destination,
    );
    target.input.set_placeholder_text(Some("Selected object"));
    let property = labeled_field(
        panel,
        named.then_some("BehaviorActionMember"),
        "Property",
        member,
    );
    let value = labeled_field(
        panel,
        named.then_some("BehaviorActionValue"),
        "Value",
        argument,
    );
    let weak_kind = kind.downgrade();
    let custom_action_row = custom_action.row.clone();
    let target_row = target.row.clone();
    let property_row = property.row.clone();
    let property_label = property.label.clone();
    let property_input = property.input.clone();
    let value_row = value.row.clone();
    let value_label = value.label.clone();
    let value_input = value.input.clone();
    let update = move || {
        if let Some(kind) = weak_kind.upgrade() {
            let selected = choice(&kind);
            custom_action_row.set_is_visible(selected == "Custom");
            target_row.set_is_visible(!matches!(selected.as_str(), "Invoke command" | "Custom"));
            property_row.set_is_visible(selected != "Custom");
            let member = match selected.as_str() {
                "Invoke command" => "Command",
                "Call method" => "Method",
                "Add class" | "Remove class" => "Class",
                _ => "Property",
            };
            property_label.set_text(Some(member));
            AutomationProperties::set_name(&property_input, Some(member));
            value_row.set_is_visible(matches!(
                selected.as_str(),
                "Change property" | "Invoke command" | "Custom"
            ));
            let value = match selected.as_str() {
                "Invoke command" => "Parameter",
                "Custom" => "Properties JSON",
                _ => "Value",
            };
            value_label.set_text(Some(value));
            AutomationProperties::set_name(&value_input, Some(value));
        }
    };
    update();
    kind.selection_changed(move |_, _| update());
    move || {
        let target = optional(&target.input);
        Ok(match choice(&kind).as_str() {
            "Change property" => BehaviorAction::ChangeProperty {
                target,
                property: text(&property.input),
                value: text(&value.input),
            },
            "Invoke command" => BehaviorAction::InvokeCommand {
                command: text(&property.input),
                parameter: optional(&value.input),
            },
            "Call method" => BehaviorAction::CallMethod {
                target,
                method: text(&property.input),
            },
            "Add class" => BehaviorAction::AddClass {
                target,
                class: text(&property.input),
            },
            "Remove class" => BehaviorAction::RemoveClass {
                target,
                class: text(&property.input),
            },
            "Custom" => BehaviorAction::Custom {
                type_name: text(&custom_action.input),
                properties: serde_json::from_str(&text(&value.input)).map_err(|e| e.to_string())?,
                children: String::new(),
            },
            _ => return Err("Choose an action".into()),
        })
    }
}

fn action_values(initial: Option<&BehaviorAction>) -> (&str, &str, &str, &str) {
    match initial {
        Some(BehaviorAction::ChangeProperty {
            target,
            property,
            value,
        }) => (
            "Change property",
            target.as_deref().unwrap_or(""),
            property,
            value,
        ),
        Some(BehaviorAction::InvokeCommand { command, parameter }) => (
            "Invoke command",
            "",
            command,
            parameter.as_deref().unwrap_or(""),
        ),
        Some(BehaviorAction::CallMethod { target, method }) => {
            ("Call method", target.as_deref().unwrap_or(""), method, "")
        }
        Some(BehaviorAction::AddClass { target, class }) => {
            ("Add class", target.as_deref().unwrap_or(""), class, "")
        }
        Some(BehaviorAction::RemoveClass { target, class }) => {
            ("Remove class", target.as_deref().unwrap_or(""), class, "")
        }
        _ => ("Change property", "", "Opacity", "0.5"),
    }
}

pub(super) fn parse_bool(value: &str) -> Result<bool, String> {
    value
        .trim()
        .to_ascii_lowercase()
        .parse()
        .map_err(|_| "Enter True or False".into())
}

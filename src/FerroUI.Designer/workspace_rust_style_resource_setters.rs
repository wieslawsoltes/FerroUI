//! Choose the mapped color resource for an existing or new Rust style setter.
use super::{SOURCE, Scope, add_button, fields, picker};
use crate::icons;
use ferroui_controls::{Control, StackPanel};
use ferroui_designer_rsx::{
    RsxStyle, RsxStyleEdit, RsxStyleField, RsxStyleResourceFamily, RsxStyleSetter,
};

pub(super) fn change_setter(
    row: &StackPanel,
    scope: &Scope,
    style: usize,
    setter: &RsxStyleSetter,
    choices: &[RsxStyleResourceFamily<'_>],
) {
    let current = setter.resource_key();
    let member = setter.declaration.start;
    let pane = fields::prompt("Assign a color resource");
    let active = scope.clone();
    resource_picker(&pane, choices, current, move |key| {
        active.apply(
            RsxStyleEdit::SetResource {
                style,
                key,
                setter: Some(member),
                getter: None,
            },
            None,
        );
    });
    fields::flyout_button(row, icons::EXPRESSION, "Assign color resource", pane);
    if let Some(source) = choices
        .iter()
        .find(|entry| Some(entry.key) == current)
        .and_then(RsxStyleResourceFamily::common_declaration)
    {
        let span = source.color_source();
        let active = scope.clone();
        add_button(
            row,
            SOURCE,
            &format!("Reveal declared resource {}", source.key),
            move || active.reveal(span),
        );
    }
}

pub(super) fn add_setter(
    bar: &StackPanel,
    scope: &Scope,
    rule: &RsxStyle,
    fields: &[&RsxStyleField],
    choices: &[RsxStyleResourceFamily<'_>],
) {
    if choices.is_empty() {
        return;
    }
    let mut available = fields
        .iter()
        .filter(|field| field.accepts_color_resource() && !rule.has_setter(field))
        .map(|field| {
            (
                format!("{} · {}", field.name, field.owner_name()),
                field.getter.clone(),
            )
        })
        .collect::<Vec<_>>();
    available.sort();
    available.dedup_by(|a, b| a.0 == b.0);
    if available.is_empty() {
        return;
    }
    let style = rule.declaration.start;
    let pane = fields::prompt("Add a color resource setter");
    let property = picker(
        "Setter property",
        available.iter().map(|(label, _)| label.as_str()),
        0,
    );
    pane.children().add(property.clone().upcast::<Control>());
    let active = scope.clone();
    resource_picker(&pane, choices, None, move |key| {
        if let Some((_, getter)) = available.get(property.selected_index() as usize) {
            active.apply(
                RsxStyleEdit::SetResource {
                    style,
                    key,
                    setter: None,
                    getter: Some(getter.clone()),
                },
                None,
            );
        }
    });
    fields::flyout_button(bar, icons::EXPRESSION, "Add color resource setter", pane);
}

fn resource_picker(
    pane: &StackPanel,
    choices: &[RsxStyleResourceFamily<'_>],
    current: Option<&str>,
    apply: impl Fn(String) + 'static,
) {
    let selected = choices
        .iter()
        .position(|entry| Some(entry.key) == current)
        .unwrap_or(0);
    let keys = choices
        .iter()
        .map(|entry| entry.key.to_owned())
        .collect::<Vec<_>>();
    let labels = choices
        .iter()
        .map(|entry| {
            format!(
                "{} · {}",
                entry.key,
                entry
                    .common_declaration()
                    .and_then(|entry| entry.color())
                    .unwrap_or_else(|| "Theme colors".into())
            )
        })
        .collect::<Vec<_>>();
    let control = picker(
        "Color resource",
        labels.iter().map(String::as_str),
        selected,
    );
    pane.children().add(control.clone().upcast::<Control>());
    add_button(pane, icons::CHECK, "Apply color resource", move || {
        if let Some(key) = keys.get(control.selected_index() as usize) {
            apply(key.clone());
        }
    });
}

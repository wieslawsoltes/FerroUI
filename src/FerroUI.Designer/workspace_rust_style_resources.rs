//! Color resources and compatible setters in Studio's Rust style editor.
use super::{SOURCE, Scope, add_button, fields, picker, text};
use crate::{PropertyColorEditor, icons};
use ferroui_base::layout::Orientation;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{Control, Grid, StackPanel};
use ferroui_designer_rsx::{
    RsxStyle, RsxStyleEdit, RsxStyleField, RsxStyleResource, RsxStyleSetter,
};

pub(super) fn section(
    body: &StackPanel,
    scope: &Scope,
    rule: &RsxStyle,
    fields: &[&RsxStyleField],
    choices: &[&RsxStyleResource],
) {
    let style = rule.declaration.start;
    let bar = StackPanel::new();
    bar.set_orientation(Orientation::Horizontal);
    bar.set_spacing(4.0);
    bar.children()
        .add(text("Color resources").upcast::<Control>());
    let pane = fields::prompt("Add Rust color resource");
    let key = fields::input("Resource key", "");
    key.set_placeholder_text(Some("Resource key"));
    pane.children().add(key.clone().upcast::<Control>());
    let active = scope.clone();
    fields::value_row(&pane, "Color value", "#FF4C82D0", &[], move |value| {
        let key = key.text().unwrap_or_default().trim().to_owned();
        active.apply(RsxStyleEdit::AddColorResource { style, key, value }, None);
    });
    fields::flyout_button(&bar, icons::ADD, "Add Rust color resource", pane);
    add_setter(&bar, scope, rule, fields, choices);
    body.children().add(bar.upcast::<Control>());

    for resource in &rule.resources {
        let resource_id = resource.declaration.start;
        let row = if let (Some(color), Ok(editor)) = (resource.color(), PropertyColorEditor::new())
        {
            let (row, tools) = fields::row(body, &resource.key);
            let _ = editor.configure(Some(&color), false);
            AutomationProperties::set_name(
                &editor.text_editor(),
                Some(&format!("{} color resource value", resource.key)),
            );
            let active = scope.clone();
            editor.on_commit(move |value| {
                active.apply(
                    RsxStyleEdit::ColorResource {
                        style,
                        resource: resource_id,
                        value,
                    },
                    None,
                );
            });
            let field = editor.control();
            Grid::set_column(&field, 1);
            row.children().add(field);
            tools
        } else {
            fields::source_row(body, &resource.key)
        };
        let span = resource.source;
        let active = scope.clone();
        add_button(
            &row,
            SOURCE,
            &format!("Reveal {} resource", resource.key),
            move || active.reveal(span),
        );
    }
}

pub(super) fn change_setter(
    row: &StackPanel,
    scope: &Scope,
    style: usize,
    setter: &RsxStyleSetter,
    choices: &[&RsxStyleResource],
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
        .find(|entry| Some(entry.key.as_str()) == current)
    {
        let span = source.source;
        let active = scope.clone();
        add_button(
            row,
            SOURCE,
            &format!("Reveal declared resource {}", source.key),
            move || active.reveal(span),
        );
    }
}

fn add_setter(
    bar: &StackPanel,
    scope: &Scope,
    rule: &RsxStyle,
    fields: &[&RsxStyleField],
    choices: &[&RsxStyleResource],
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
    choices: &[&RsxStyleResource],
    current: Option<&str>,
    apply: impl Fn(String) + 'static,
) {
    let selected = choices
        .iter()
        .position(|entry| Some(entry.key.as_str()) == current)
        .unwrap_or(0);
    let keys = choices
        .iter()
        .map(|entry| entry.key.clone())
        .collect::<Vec<_>>();
    let labels = choices
        .iter()
        .map(|entry| format!("{} · {}", entry.key, entry.color().unwrap_or_default()))
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

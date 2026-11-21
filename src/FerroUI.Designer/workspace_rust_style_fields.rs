//! Native fields and source pickers for Studio Rust styles.
use super::{add_button, icon_button, picker, text, Scope, SOURCE};
use crate::icons;
use ferroui_base::{input::Key, layout::Orientation, Ref};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Button, ColumnDefinitions, Control, Flyout, Grid, PlacementMode, StackPanel, TextBox,
};
use ferroui_designer_rsx::{RsxStyle, RsxStyleEdit, RsxStyleField, RsxStyleIssue};
use std::rc::Rc;

pub(super) fn value_row(
    body: &StackPanel,
    label: &str,
    value: &str,
    options: &[String],
    apply: impl Fn(String) + 'static,
) -> Ref<StackPanel> {
    let (row, tools) = row(body, label);
    let apply = Rc::new(apply);
    if options.is_empty() {
        let input = input(label, value);
        let commit = {
            let input = input.downgrade();
            let apply = apply.clone();
            move || {
                if let Some(input) = input.upgrade() {
                    apply(input.text().unwrap_or_default());
                }
            }
        };
        let enter = commit.clone();
        input.key_down(move |_, event| {
            if event.key == Key::Enter {
                event.set_handled(true);
                enter();
            }
        });
        Grid::set_column(&input, 1);
        row.children().add(input.upcast::<Control>());
        add_button(&tools, icons::CHECK, &format!("Apply {label}"), commit);
    } else {
        let selected = options
            .iter()
            .position(|option| option == value)
            .unwrap_or(0);
        let values = options.to_vec();
        let input = picker(label, values.iter().map(String::as_str), selected);
        let weak = input.downgrade();
        add_button(&tools, icons::CHECK, &format!("Apply {label}"), move || {
            if let Some(value) = weak
                .upgrade()
                .and_then(|input| values.get(input.selected_index() as usize))
            {
                apply(value.clone());
            }
        });
        Grid::set_column(&input, 1);
        row.children().add(input.upcast::<Control>());
    }
    tools
}

pub(super) fn source_row(body: &StackPanel, label: &str) -> Ref<StackPanel> {
    read_only_row(body, label, "Rust expression")
}

pub(super) fn read_only_row(body: &StackPanel, label: &str, value: &str) -> Ref<StackPanel> {
    let (row, tools) = row(body, label);
    let value = text(value);
    Grid::set_column(&value, 1);
    row.children().add(value.upcast::<Control>());
    tools
}

pub(super) fn row(body: &StackPanel, label: &str) -> (Ref<Grid>, Ref<StackPanel>) {
    let row = Grid::new();
    row.set_column_definitions(
        ColumnDefinitions::parse("112,*,Auto").expect("Style editor columns"),
    );
    row.set_column_spacing(4.0);
    let title = text(label);
    title.set_text_trimming(<dyn ferroui_base::media::TextTrimming>::character_ellipsis());
    let buttons = StackPanel::new();
    buttons.set_orientation(Orientation::Horizontal);
    buttons.set_spacing(2.0);
    Grid::set_column(&buttons, 2);
    row.children().add(title.upcast::<Control>());
    row.children().add(buttons.clone().upcast::<Control>());
    body.children().add(row.clone().upcast::<Control>());
    (row, buttons)
}

pub(super) fn new_style_button(tools: &StackPanel, scope: &Scope, hosts: Vec<(usize, String)>) {
    if hosts.is_empty() {
        return;
    }
    let pane = prompt("New Rust style");
    let owners = picker(
        "Style owner",
        hosts.iter().map(|(_, label)| label.as_str()),
        0,
    );
    pane.children().add(owners.clone().upcast::<Control>());
    let active = scope.clone();
    value_row(&pane, "Selector", "", &[], move |selector| {
        if let Some((owner, _)) = hosts.get(owners.selected_index() as usize) {
            active.apply(
                RsxStyleEdit::Create {
                    owner: *owner,
                    selector: selector.clone(),
                },
                Some(&selector),
            );
        }
    });
    flyout_button(tools, icons::ADD, "New Rust style", pane);
}

pub(super) fn new_nested_button(tools: &StackPanel, scope: &Scope, style: usize) {
    let pane = prompt("Nested Rust style");
    let active = scope.clone();
    value_row(&pane, "Selector", "^:pointerover", &[], move |selector| {
        active.apply(
            RsxStyleEdit::CreateNested {
                style,
                selector: selector.clone(),
            },
            Some(&selector),
        );
    });
    flyout_button(tools, icons::ADD, "Add nested style", pane);
}

pub(super) fn add_setter_button(
    tools: &StackPanel,
    scope: &Scope,
    rule: &RsxStyle,
    fields: &[&RsxStyleField],
) {
    let style = rule.declaration.start;
    let mut choices = fields
        .iter()
        .filter(|field| !rule.has_setter(field))
        .map(|field| {
            (
                format!("{} · {}", field.name, field.owner_name()),
                field.getter.clone(),
            )
        })
        .collect::<Vec<_>>();
    choices.sort();
    choices.dedup_by(|a, b| a.0 == b.0);
    if choices.is_empty() {
        return;
    }
    let pane = prompt("Add Rust setter");
    let properties = picker(
        "Setter property",
        choices.iter().map(|(label, _)| label.as_str()),
        0,
    );
    pane.children().add(properties.clone().upcast::<Control>());
    let active = scope.clone();
    value_row(&pane, "Value", "", &[], move |value| {
        if let Some((_, getter)) = choices.get(properties.selected_index() as usize) {
            active.apply(
                RsxStyleEdit::AddSetter {
                    style,
                    getter: getter.clone(),
                    value,
                },
                None,
            );
        }
    });
    flyout_button(tools, icons::ADD, "Add Rust setter", pane);
}

pub(super) fn issues_button(tools: &StackPanel, scope: &Scope, issues: &[RsxStyleIssue]) {
    if issues.is_empty() {
        return;
    }
    let pane = prompt("Rust style expressions");
    let issues = issues.to_vec();
    let choices = picker(
        "Source expression",
        issues.iter().map(|issue| issue.reason),
        0,
    );
    pane.children().add(choices.clone().upcast::<Control>());
    let active = scope.clone();
    add_button(&pane, SOURCE, "Reveal Rust style expression", move || {
        if let Some(issue) = issues.get(choices.selected_index() as usize) {
            active.reveal(issue.source);
        }
    });
    flyout_button(
        tools,
        SOURCE,
        "Reveal styles with source-only expressions",
        pane,
    );
}

pub(super) fn input(label: &str, value: &str) -> Ref<TextBox> {
    let input = TextBox::new();
    input.classes().add("designer-field");
    input.set_min_width(0.0);
    input.set_text(Some(value));
    AutomationProperties::set_name(&input, Some(label));
    input
}

pub(super) fn prompt(title: &str) -> Ref<StackPanel> {
    let pane = StackPanel::new();
    pane.set_width(340.0);
    pane.set_spacing(5.0);
    pane.children().add(text(title).upcast::<Control>());
    pane
}

pub(super) fn flyout_button(tools: &StackPanel, glyph: &str, label: &str, pane: Ref<StackPanel>) {
    let button: Ref<Button> = icon_button(glyph, label, || {});
    let flyout = Flyout::new();
    flyout.set_placement(PlacementMode::Bottom);
    flyout.popup().set_should_use_overlay_layer(true);
    flyout.flyout_presenter_classes().add("designer-chrome");
    flyout.set_content(Some(Control::boxed(pane)));
    button.set_flyout(Some(flyout.upcast()));
    tools.children().add(button.upcast::<Control>());
}

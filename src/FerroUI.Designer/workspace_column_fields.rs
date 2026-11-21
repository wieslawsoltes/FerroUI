use super::*;
use ferroui_base::input::Key;
use ferroui_controls::{TextBox, ToolTip};
use ferroui_controls::automation::AutomationProperties;
use ferroui_designer_tools::ColumnField;

pub(super) fn add_fields(panel: &StackPanel, column: &AuthoredColumn, scope: &ColumnScope) {
    for field in &column.fields {
        if scope.rust && matches!(field.name, "CellTemplate" | "CellEditingTemplate") {
            continue;
        }
        let editable = scope.writable && field.editable;
        if !editable && field.source.is_none() {
            continue;
        }
        let row = Grid::new();
        row.set_column_definitions(
            ColumnDefinitions::parse("126,*,Auto,Auto").expect("column property row"),
        );
        let title = text(field.name);
        title.set_font_size(11.0);
        row.children().add(title.upcast::<Control>());
        if editable {
            input(&row, column.node, field, scope);
        } else if let Some(source) = field.source {
            let caption = field.value.as_deref().unwrap_or("Object");
            let value = text(caption);
            value.set_text_trimming(<dyn ferroui_base::media::TextTrimming>::character_ellipsis());
            ToolTip::set_tip(&value, Some(Rc::new(caption.to_owned())));
            Grid::set_column(&value, 1);
            row.children().add(value.upcast::<Control>());
            let scope = scope.clone();
            let reveal = icon_button(
                SOURCE,
                &format!("Reveal {} in source", field.name),
                move || scope.reveal(source),
            );
            Grid::set_column(&reveal, 2);
            row.children().add(reveal.upcast::<Control>());
        }
        panel.children().add(row.upcast::<Control>());
    }
    for child in &column.children {
        let title = text(&format!("Inner column · {}", child.label));
        title.set_font_weight(ferroui_base::media::FontWeight::SemiBold);
        panel.children().add(title.upcast::<Control>());
        add_template_action(panel, child, scope);
        add_fields(panel, child, scope);
    }
}

fn input(row: &Grid, column: NodeId, field: &ColumnField, scope: &ColumnScope) {
    let property = field.name;
    let apply = {
        let scope = scope.clone();
        move |value| {
            scope.apply(
                ColumnChange::Set {
                    column,
                    field: property.into(),
                    value,
                },
                scope.selected,
            )
        }
    };
    let caption = format!("Apply {property}");
    let button = if field.choices.is_empty() {
        let input = TextBox::new();
        input.classes().add("designer-field");
        input.set_min_width(0.0);
        input.set_text(field.value.as_deref());
        AutomationProperties::set_name(&input, Some(property));
        Grid::set_column(&input, 1);
        row.children().add(input.clone().upcast::<Control>());
        let weak = input.downgrade();
        let commit = move || {
            if let Some(input) = weak.upgrade() {
                apply(input.text().unwrap_or_default());
            }
        };
        let enter = commit.clone();
        input.key_down(move |_, event| {
            if event.key == Key::Enter {
                event.set_handled(true);
                enter();
            }
        });
        icon_button(icons::CHECK, &caption, commit)
    } else {
        let placeholder = "Default";
        let values = std::iter::once(placeholder).chain(field.choices.iter().copied());
        let selected = field
            .choices
            .iter()
            .position(|choice| Some(*choice) == field.value.as_deref())
            .map_or(0, |index| index + 1);
        let input = picker(property, values, selected);
        Grid::set_column(&input, 1);
        row.children().add(input.clone().upcast::<Control>());
        let options = field.choices;
        icon_button(icons::CHECK, &caption, move || {
            if let Some(value) = (input.selected_index() - 1)
                .try_into()
                .ok()
                .and_then(|index: usize| options.get(index))
            {
                apply((*value).to_owned());
            }
        })
    };
    Grid::set_column(&button, 2);
    row.children().add(button.upcast::<Control>());
    if field.can_clear {
        let scope = scope.clone();
        let button = icon_button(RESET, &format!("Reset {property}"), move || {
            scope.apply(
                ColumnChange::Clear {
                    column,
                    field: property.into(),
                },
                scope.selected,
            );
        });
        Grid::set_column(&button, 3);
        row.children().add(button.upcast::<Control>());
    }
}

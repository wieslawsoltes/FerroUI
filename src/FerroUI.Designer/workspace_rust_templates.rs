use super::{RESET, SOURCE, TemplateActions, add_button, icon_button, slot_toolbar, text};
use crate::icons;
use ferroui_base::{input::Key, media::TextWrapping};
use ferroui_controls::{
    ColumnDefinitions, Control, Grid, StackPanel, TextBox, automation::AutomationProperties,
};
use ferroui_designer_rsx::RsxItemTemplate;
use ferroui_designer_tools::{ItemTemplateChange, ItemTemplateMember};

pub(super) fn add_slot(body: &StackPanel, actions: TemplateActions, slot: RsxItemTemplate) {
    let value = match (slot.body, slot.expression) {
        (Some(_), _) => "Inline RSX",
        (None, Some(_)) => "Rust expression",
        (None, None) => "Default",
    };
    let property = slot.property;
    let tools = slot_toolbar(body, property.name(), value);
    if slot.body.is_some() {
        let actions = actions.clone();
        add_button(&tools, icons::FIT, "Edit template on canvas", move || {
            actions.run(|workspace| {
                workspace
                    .session
                    .borrow_mut()
                    .preview_rust_template(actions.owner, property)?;
                actions.close();
                workspace.focus_definition_canvas();
                Ok(())
            });
        });
    }
    if let Some(span) = slot.expression {
        let actions = actions.clone();
        add_button(&tools, SOURCE, "Reveal template source", move || {
            actions.reveal(span)
        });
    }
    if slot.can_reset {
        let actions = actions.clone();
        add_button(&tools, RESET, "Reset template", move || {
            actions.apply(|session| {
                session.edit_rust_template(actions.owner, property, ItemTemplateChange::Reset, None)
            });
        });
    }
    if slot.can_create {
        add_type_input(body, actions, property);
    } else if let Some(reason) = slot.reason {
        let message = text(reason);
        message.set_text_wrapping(TextWrapping::Wrap);
        body.children().add(message.upcast::<Control>());
    }
}

fn add_type_input(body: &StackPanel, actions: TemplateActions, property: ItemTemplateMember) {
    let row = Grid::new();
    row.set_column_definitions(ColumnDefinitions::parse("*,Auto").expect("template type input"));
    let input = TextBox::new();
    input.classes().add("designer-field");
    input.set_placeholder_text(Some("Rust item type"));
    AutomationProperties::set_name(&input, Some(&format!("{} Rust item type", property.name())));
    let weak = input.downgrade();
    let create = move || {
        if let Some(input) = weak.upgrade() {
            let row_type = input.text().unwrap_or_default();
            actions.apply(|session| {
                session.edit_rust_template(
                    actions.owner,
                    property,
                    ItemTemplateChange::CreateInline,
                    Some(&row_type),
                )
            });
        }
    };
    let enter = create.clone();
    input.key_down(move |_, event| {
        if event.key == Key::Enter {
            event.set_handled(true);
            enter();
        }
    });
    let button = icon_button(icons::ADD, "Create inline Rust template", create);
    Grid::set_column(&button, 1);
    row.children().add(input.upcast::<Control>());
    row.children().add(button.upcast::<Control>());
    body.children().add(row.upcast::<Control>());
}

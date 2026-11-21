use super::{
    DesignerWorkspace, RESET, SOURCE, TemplateActions, add_button, icon_button, slot_toolbar, text,
};
use crate::{NodeId, icons};
use ferroui_base::{input::Key, media::TextWrapping};
use ferroui_controls::{
    ColumnDefinitions, Control, Grid, StackPanel, TextBox, automation::AutomationProperties,
};
use ferroui_designer_rsx::{RsxItemTemplate, RsxSourceLocation};
use ferroui_designer_tools::{ItemTemplateChange, ItemTemplateMember};
use std::rc::Rc;

impl DesignerWorkspace {
    pub(crate) fn open_rust_template(
        self: &Rc<Self>,
        owner: NodeId,
        property: ItemTemplateMember,
    ) -> Result<(), String> {
        let ((body, consumer), modules) =
            self.with_current_rust_sources(|session, projection| {
                Ok((
                    session.rust_template_body(projection, owner, property)?,
                    session.document_uri().to_owned(),
                ))
            })?;
        let location = body.location;
        if let Some(uri) = &location.uri {
            let source = modules
                .as_ref()
                .and_then(|modules| modules.source(uri))
                .ok_or("The template module is outside the accepted Cargo target")?;
            self.visit_source_document(uri, |workspace| {
                let mut session = workspace.session.borrow_mut();
                if session.authored_source() != source.source.as_ref() {
                    return Err("The Rust template source changed. Reopen Item Templates.".into());
                }
                session.enter_companion_rust_template(location.span, consumer, property)
            })?;
        } else {
            self.session
                .borrow_mut()
                .enter_rust_template(owner, location.span, property)?;
        }
        self.focus_definition_canvas();
        Ok(())
    }

    pub(crate) fn edit_workspace_rust_template(
        self: &Rc<Self>,
        owner: NodeId,
        property: ItemTemplateMember,
        change: ItemTemplateChange,
        row_type: Option<&str>,
    ) -> Result<(), String> {
        let (edit, modules) = self.with_current_rust_sources(|session, projection| {
            session.rust_template_plan(projection, owner, property, &change, row_type)
        })?;
        if let Some(uri) = &edit.source.uri {
            self.apply_rust_module_patches(
                uri,
                edit.patches,
                modules.ok_or("The Rust template module is unavailable")?,
                change.label(),
            )
        } else {
            self.session
                .borrow_mut()
                .apply_rust_template_plan(change.label(), edit)
        }
    }
}

pub(super) fn add_slot(body: &StackPanel, actions: TemplateActions, slot: RsxItemTemplate) {
    let value = match (&slot.body, &slot.expression) {
        (Some(body), _) => body
            .factory
            .as_ref()
            .map_or_else(|| "Inline RSX".into(), |name| format!("{name}()")),
        (None, Some(_)) => "Rust expression".into(),
        (None, None) => "Default".into(),
    };
    let property = slot.property;
    let tools = slot_toolbar(body, property.name(), &value);
    if let Some(body) = slot.body {
        let preview = actions.clone();
        add_button(&tools, icons::FIT, "Edit template on canvas", move || {
            preview.run(|workspace| {
                workspace.open_rust_template(preview.owner, property)?;
                preview.close();
                Ok(())
            });
        });
        if body.factory.is_some() {
            add_source_button(
                &tools,
                actions.clone(),
                "Reveal shared template body",
                body.location,
            );
        }
    }
    if let Some(source) = slot.expression {
        add_source_button(
            &tools,
            actions.clone(),
            "Reveal template assignment",
            source,
        );
    }
    if slot.can_reset {
        let actions = actions.clone();
        add_button(&tools, RESET, "Reset template", move || {
            actions.edit_rust(property, ItemTemplateChange::Reset, None);
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

fn add_source_button(
    bar: &StackPanel,
    actions: TemplateActions,
    label: &str,
    source: RsxSourceLocation,
) {
    add_button(bar, SOURCE, label, move || {
        actions.run(|workspace| {
            if let Some(uri) = &source.uri {
                workspace.reveal_rust_module_span(uri, source.span)?;
            } else {
                workspace.select_source_span(source.span);
            }
            actions.close();
            Ok(())
        });
    });
}

impl TemplateActions {
    fn edit_rust(
        &self,
        property: ItemTemplateMember,
        change: ItemTemplateChange,
        row_type: Option<&str>,
    ) {
        self.run(|workspace| {
            workspace.edit_workspace_rust_template(self.owner, property, change, row_type)?;
            self.refresh(workspace);
            Ok(())
        });
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
            actions.edit_rust(property, ItemTemplateChange::CreateInline, Some(&row_type));
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

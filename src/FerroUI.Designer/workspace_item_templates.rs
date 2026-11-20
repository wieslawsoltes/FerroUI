//! Contextual native Studio controls. Source edits use the same template plans as MCP.
use super::DesignerWorkspace;
use super::inspector_actions::{COPY, RESET, SOURCE, add_button, icon_button, picker, text};
use crate::{design_time::DesignTimeSnapshot, icons};
use ferroui_base::layout::Orientation;
use ferroui_base::media::TextWrapping;
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{
    Button, ColumnDefinitions, Control, Flyout, Grid, PlacementMode, StackPanel, TextBlock,
};
use ferroui_designer_core::{NodeId, SourceSpan};
use ferroui_designer_tools::{
    ItemTemplateChange, ItemTemplateMember, ItemTemplateResource, ItemTemplateSlot,
    ItemTemplateValue, item_template_resources,
};
use std::rc::{Rc, Weak};

#[cfg(test)]
#[path = "workspace_item_templates_tests.rs"]
mod tests;

impl DesignerWorkspace {
    pub(super) fn connect_item_templates(self: &Rc<Self>) {
        let button = self.root.get_control::<Button>("EditItemTemplates");
        let flyout = Flyout::new();
        flyout.set_placement(PlacementMode::Left);
        flyout.popup().set_should_use_overlay_layer(true);
        flyout.flyout_presenter_classes().add("designer-chrome");
        let workspace = Rc::downgrade(self);
        let popup = flyout.downgrade();
        flyout
            .opening(move |_| {
                if let (Some(workspace), Some(popup)) = (workspace.upgrade(), popup.upgrade()) {
                    render(&workspace, &popup);
                }
            })
            .detach();
        button.set_flyout(Some(flyout.upcast()));
    }

    pub(super) fn refresh_item_template_command(&self) {
        let session = self.session.borrow();
        self.root
            .get_control::<Button>("EditItemTemplates")
            .set_is_visible(matches!(
                session.selection(), [node] if !session.item_template_members(*node).is_empty()
            ));
    }
}

#[derive(Clone)]
struct TemplateActions {
    workspace: Weak<DesignerWorkspace>,
    flyout: WeakRef<Flyout>,
    snapshot: DesignTimeSnapshot,
    status: WeakRef<TextBlock>,
    owner: NodeId,
}

impl TemplateActions {
    fn run(&self, action: impl FnOnce(&Rc<DesignerWorkspace>) -> Result<(), String>) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };
        let result = if workspace.pending_source.get()
            || !self.snapshot.is_current(&workspace.session.borrow())
        {
            Err("The source, selection or preview changed. Reopen Item Templates.".into())
        } else {
            action(&workspace)
        };
        if let Err(error) = result {
            if let Some(status) = self.status.upgrade() {
                status.set_text(Some(&error));
                status.set_is_visible(true);
            }
        }
    }

    fn edit(&self, property: ItemTemplateMember, change: ItemTemplateChange) {
        self.run(|workspace| {
            workspace
                .session
                .borrow_mut()
                .edit_item_template(self.owner, property, change)?;
            workspace.refresh(true);
            if let Some(flyout) = self.flyout.upgrade() {
                render(workspace, &flyout);
            }
            Ok(())
        });
    }

    fn preview(&self, property: ItemTemplateMember) {
        self.run(|workspace| {
            workspace
                .session
                .borrow_mut()
                .preview_item_template(self.owner, property)?;
            self.close();
            workspace.focus_definition_canvas();
            Ok(())
        });
    }

    fn reveal(&self, span: SourceSpan) {
        self.run(|workspace| {
            self.close();
            workspace.select_source_span(span);
            Ok(())
        });
    }

    fn close(&self) {
        if let Some(flyout) = self.flyout.upgrade() {
            flyout.hide();
        }
    }
}

fn render(workspace: &Rc<DesignerWorkspace>, flyout: &Ref<Flyout>) {
    let body = StackPanel::new();
    body.set_width(360.0);
    body.set_spacing(8.0);
    flyout.set_content(Some(Control::boxed(body.clone())));
    let status = text("");
    status.set_text_wrapping(TextWrapping::Wrap);
    status.set_is_visible(false);
    body.children().add(status.clone().upcast::<Control>());
    let session = workspace.session.borrow();
    let [owner] = session.selection() else { return };
    let result = session.item_template_slots(*owner).and_then(|slots| {
        let resources = if slots.iter().any(|slot| slot.reason.is_none()) {
            item_template_resources(session.document(), *owner)
                .map_err(|error| error.to_string())?
        } else {
            Vec::new()
        };
        Ok((slots, resources))
    });
    let (slots, resources) = match result {
        Ok(model) => model,
        Err(error) => {
            status.set_text(Some(&error));
            status.set_is_visible(true);
            return;
        }
    };
    let actions = TemplateActions {
        workspace: Rc::downgrade(workspace),
        flyout: flyout.downgrade(),
        snapshot: DesignTimeSnapshot::capture(&session),
        status: status.downgrade(),
        owner: *owner,
    };
    for slot in slots {
        let span = slot
            .value
            .definition()
            .and_then(|node| session.document().ast().node(node))
            .map(|node| node.name_span)
            .or(slot.span);
        let can_copy = slot.can_copy_inline(session.document());
        add_slot(&body, actions.clone(), &slot, span, can_copy, &resources);
    }
}

fn add_slot(
    body: &StackPanel,
    actions: TemplateActions,
    slot: &ItemTemplateSlot,
    source: Option<SourceSpan>,
    can_copy: bool,
    resources: &[ItemTemplateResource],
) {
    let title = StackPanel::new();
    let label = text(slot.property.name());
    label.set_font_weight(ferroui_base::media::FontWeight::SemiBold);
    title.children().add(label.upcast::<Control>());
    let value = match &slot.value {
        ItemTemplateValue::Unset => "Default",
        ItemTemplateValue::Inline { .. } => "Inline",
        ItemTemplateValue::Resource { key, .. } => key,
        ItemTemplateValue::Expression => "Authored expression",
    };
    let value = text(value);
    value.set_font_size(11.0);
    value.set_text_trimming(<dyn ferroui_base::media::TextTrimming>::character_ellipsis());
    title.children().add(value.upcast::<Control>());

    let bar = Grid::new();
    bar.set_column_definitions(ColumnDefinitions::parse("*,Auto").expect("template toolbar"));
    bar.children().add(title.upcast::<Control>());
    let tools = StackPanel::new();
    tools.set_orientation(Orientation::Horizontal);
    Grid::set_column(&tools, 1);
    bar.children().add(tools.clone().upcast::<Control>());
    body.children().add(bar.upcast::<Control>());
    let property = slot.property;
    if slot.value.definition().is_some() {
        let action = actions.clone();
        add_button(&tools, icons::FIT, "Edit template on canvas", move || {
            action.preview(property)
        });
    }
    if let Some(span) = source {
        let action = actions.clone();
        add_button(&tools, SOURCE, "Reveal template source", move || {
            action.reveal(span)
        });
    }
    if let Some(reason) = &slot.reason {
        let message = text(reason);
        message.set_text_wrapping(TextWrapping::Wrap);
        body.children().add(message.upcast::<Control>());
        return;
    }
    if matches!(slot.value, ItemTemplateValue::Unset) {
        let action = actions.clone();
        add_button(&tools, icons::ADD, "Create inline template", move || {
            action.edit(property, ItemTemplateChange::CreateInline)
        });
    }
    if can_copy {
        let action = actions.clone();
        add_button(&tools, COPY, "Make inline copy", move || {
            action.edit(property, ItemTemplateChange::CopyInline)
        });
    }
    if slot.can_reset {
        let action = actions.clone();
        add_button(&tools, RESET, "Reset item template", move || {
            action.edit(property, ItemTemplateChange::Reset)
        });
    }
    if !resources.is_empty() {
        add_resource_picker(body, actions, property, resources);
    }
}

fn add_resource_picker(
    body: &StackPanel,
    actions: TemplateActions,
    property: ItemTemplateMember,
    resources: &[ItemTemplateResource],
) {
    let bar = Grid::new();
    bar.set_column_definitions(
        ColumnDefinitions::parse("*,Auto").expect("template resource picker"),
    );
    let choice = picker(
        "Template resource",
        resources.iter().map(|resource| resource.key.as_str()),
        0,
    );
    let ids = resources
        .iter()
        .map(|resource| resource.node)
        .collect::<Vec<_>>();
    let select = choice.downgrade();
    let assign = icon_button(icons::CHECK, "Assign template resource", move || {
        if let Some(resource) = select
            .upgrade()
            .and_then(|choice| ids.get(choice.selected_index() as usize))
        {
            actions.edit(
                property,
                ItemTemplateChange::AssignResource {
                    resource: *resource,
                },
            );
        }
    });
    Grid::set_column(&assign, 1);
    bar.children().add(choice.upcast::<Control>());
    bar.children().add(assign.upcast::<Control>());
    body.children().add(bar.upcast::<Control>());
}

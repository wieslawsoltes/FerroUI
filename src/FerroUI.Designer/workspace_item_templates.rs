//! Contextual native Studio controls. Source edits use the same template plans as MCP.
use super::inspector_actions::{add_button, icon_button, picker, text, COPY, RESET, SOURCE};
use super::DesignerWorkspace;
use crate::item_template_project::{IncludedItemTemplate, IncludedTemplatePreview};
use crate::{design_time::DesignTimeSnapshot, icons};
use ferroui_base::layout::Orientation;
use ferroui_base::media::TextWrapping;
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{
    Button, ColumnDefinitions, Control, Flyout, Grid, PlacementMode, StackPanel, TextBlock,
};
use ferroui_designer_core::{Edit, NodeId, SourceSpan};
use ferroui_designer_tools::{
    ItemTemplateChange, ItemTemplateMember, ItemTemplateSlot,
    ItemTemplateValue,
};
use std::rc::{Rc, Weak};

#[path = "workspace_rust_templates.rs"]
mod rust;
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
                session.selection(), [node] if session.has_control_template(*node)
                    || !session.item_template_members(*node).is_empty()
                    || session.has_rust_template_owner(*node)
            ));
    }

    pub(crate) fn open_included_template(
        self: &Rc<Self>,
        target: IncludedTemplatePreview,
    ) -> Result<(), String> {
        let source = target.source.clone();
        self.in_template_document(&source, |workspace, _| {
            workspace
                .session
                .borrow_mut()
                .enter_included_item_template(target)?;
            workspace.focus_definition_canvas();
            Ok(())
        })
    }

    fn in_template_document(
        self: &Rc<Self>,
        source: &IncludedItemTemplate,
        action: impl FnOnce(&Rc<Self>, SourceSpan) -> Result<(), String>,
    ) -> Result<(), String> {
        self.visit_source_document(&source.location.uri, |workspace| {
            let (_, span) = {
                let session = workspace.session.borrow();
                source.node_in(session.document(), session.document_uri())?
            };
            action(workspace, span)
        })
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
        self.apply(|session| session.edit_item_template(self.owner, property, change));
    }

    fn copy(&self, property: ItemTemplateMember, reviewed: &[Edit]) {
        self.apply(|session| {
            let slot = session.item_template_slot(self.owner, property)?;
            let edits = session.plan_inline_template_copy(&slot)?;
            if edits != reviewed {
                return Err("A template dependency changed. Reopen Item Templates.".into());
            }
            session.apply(ItemTemplateChange::CopyInline.label(), edits).map_err(|error| error.to_string())?;
            Ok(())
        });
    }

    fn apply(&self, edit: impl FnOnce(&mut crate::DesignerSession) -> Result<(), String>) {
        self.run(|workspace| {
            edit(&mut workspace.session.borrow_mut())?;
            workspace.refresh(true);
            if let Some(flyout) = self.flyout.upgrade() {
                render(workspace, &flyout);
            }
            Ok(())
        });
    }

    fn preview(&self, property: ItemTemplateMember) {
        self.run(|workspace| {
            let included = {
                let session = workspace.session.borrow();
                let slot = session.item_template_slot(self.owner, property)?;
                session.included_item_template_preview(&slot)?
            };
            if let Some(target) = included {
                workspace.open_included_template(target)?;
                self.close();
                return Ok(());
            }
            workspace
                .session
                .borrow_mut()
                .preview_item_template(self.owner, property)?;
            self.close();
            workspace.focus_definition_canvas();
            Ok(())
        });
    }

    fn reveal_included(&self, source: &IncludedItemTemplate) {
        self.run(|workspace| {
            workspace.in_template_document(source, |workspace, span| {
                workspace.select_source_span(span);
                Ok(())
            })?;
            self.close();
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
    let actions = TemplateActions {
        workspace: Rc::downgrade(workspace),
        flyout: flyout.downgrade(),
        snapshot: DesignTimeSnapshot::capture(&session),
        status: status.downgrade(),
        owner: *owner,
    };
    if session.uses_rust_source() {
        for slot in session.rust_item_templates(*owner) {
            rust::add_slot(&body, actions.clone(), slot);
        }
        return;
    }
    let result = session.item_template_slots(*owner).and_then(|mut slots| {
        if session.has_control_template(*owner) {
            slots.push(session.item_template_slot(*owner, ItemTemplateMember::Template)?);
        }
        slots.into_iter().map(|slot| {
            let included = session.assigned_included_template(&slot)?;
            let resources = if slot.reason.is_none() { template_choices(&session, *owner, slot.property)? } else { Vec::new() };
            Ok((slot, included, resources))
        }).collect::<Result<Vec<_>, String>>()
    });
    let slots = match result {
        Ok(model) => model,
        Err(error) => {
            status.set_text(Some(&error));
            status.set_is_visible(true);
            return;
        }
    };
    for (slot, included, resources) in slots {
        let span = slot
            .value
            .definition()
            .and_then(|node| session.document().ast().node(node))
            .map(|node| node.name_span)
            .or(slot.span);
        let copy = session.inline_template_copy_offer(&slot);
        add_slot(
            &body,
            actions.clone(),
            &slot,
            span,
            included,
            copy,
            &resources,
        );
    }
}

fn slot_toolbar(body: &StackPanel, label: &str, value: &str) -> Ref<StackPanel> {
    let title = StackPanel::new();
    let label = text(label);
    label.set_font_weight(ferroui_base::media::FontWeight::SemiBold);
    title.children().add(label.upcast::<Control>());
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
    tools
}

fn add_slot(
    body: &StackPanel,
    actions: TemplateActions,
    slot: &ItemTemplateSlot,
    source: Option<SourceSpan>,
    included: Option<IncludedItemTemplate>,
    copy: Option<Result<Vec<Edit>, String>>,
    resources: &[TemplateChoice],
) {
    let label = if slot.property == ItemTemplateMember::Template { "Control Template" } else { slot.property.name() };
    let value = match &slot.value {
        ItemTemplateValue::Unset => "Default",
        ItemTemplateValue::Inline { .. } => "Inline",
        ItemTemplateValue::Resource { key, .. } => key,
        ItemTemplateValue::Expression => "Authored expression",
    };
    let tools = slot_toolbar(body, label, value);
    let property = slot.property;
    if slot.value.definition().is_some() || included.is_some() {
        let action = actions.clone();
        add_button(&tools, icons::FIT, "Edit template on canvas", move || {
            action.preview(property)
        });
    }
    if let Some(included) = included {
        let action = actions.clone();
        add_button(
            &tools,
            SOURCE,
            "Reveal included template source",
            move || action.reveal_included(&included),
        );
    } else if let Some(span) = source {
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
    match copy {
        Some(Ok(edits)) => {
            let action = actions.clone();
            add_button(&tools, COPY, "Make inline copy", move || action.copy(property, &edits));
        }
        Some(Err(reason)) => {
            let message = text(&reason);
            message.set_text_wrapping(TextWrapping::Wrap);
            body.children().add(message.upcast::<Control>());
        }
        None => {}
    }
    if slot.can_reset {
        let action = actions.clone();
        add_button(&tools, RESET, "Reset template", move || {
            action.edit(property, ItemTemplateChange::Reset)
        });
    }
    if !resources.is_empty() {
        add_resource_picker(body, actions, property, resources);
    }
}

struct TemplateChoice {
    label: String,
    change: ItemTemplateChange,
}

fn template_choices(
    session: &crate::DesignerSession,
    owner: NodeId,
    property: ItemTemplateMember,
) -> Result<Vec<TemplateChoice>, String> {
    let local = session.local_template_resources(owner, property)?;
    let mut choices = local
        .into_iter()
        .map(|resource| TemplateChoice {
            label: resource.key,
            change: ItemTemplateChange::AssignResource {
                resource: resource.node,
            },
        })
        .collect::<Vec<_>>();
    choices.extend(
        session
            .included_template_resources(owner, property)?
            .into_iter()
            .map(|resource| {
                let name = ferroui_designer_project::file_path(&resource.uri)
                    .ok()
                    .and_then(|path| {
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .map(str::to_owned)
                    });
                TemplateChoice {
                    label: name.map_or_else(
                        || resource.key.clone(),
                        |name| format!("{} · {name}", resource.key),
                    ),
                    change: ItemTemplateChange::AssignIncludedResource { key: resource.key },
                }
            }),
    );
    Ok(choices)
}

fn add_resource_picker(
    body: &StackPanel,
    actions: TemplateActions,
    property: ItemTemplateMember,
    resources: &[TemplateChoice],
) {
    let bar = Grid::new();
    bar.set_column_definitions(
        ColumnDefinitions::parse("*,Auto").expect("template resource picker"),
    );
    let choice = picker(
        "Template resource",
        resources.iter().map(|resource| resource.label.as_str()),
        0,
    );
    let changes = resources
        .iter()
        .map(|resource| resource.change.clone())
        .collect::<Vec<_>>();
    let select = choice.downgrade();
    let assign = icon_button(icons::CHECK, "Assign template resource", move || {
        if let Some(change) = select
            .upgrade()
            .and_then(|choice| changes.get(choice.selected_index() as usize))
        {
            actions.edit(property, change.clone());
        }
    });
    Grid::set_column(&assign, 1);
    bar.children().add(choice.upcast::<Control>());
    bar.children().add(assign.upcast::<Control>());
    body.children().add(bar.upcast::<Control>());
}

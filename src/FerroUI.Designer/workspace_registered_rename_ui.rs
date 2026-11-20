use super::super::reviewed_sources::ui::{panel, relative, source_diffs, toolbar};
use super::{DesignerWorkspace, RenameAnchor};
use crate::{
    icons,
    workspace::inspector_actions::{SOURCE, add_button, text},
};
use ferroui_base::{Ref, media::TextWrapping};
use ferroui_controls::{
    Button, ComboBox, Control, Flyout, PlacementMode, TextBox,
    automation::{AutomationLiveSetting, AutomationProperties},
};
use ferroui_designer_language::RegisteredPropertyPlan;
use std::rc::Rc;

impl DesignerWorkspace {
    pub(in super::super) fn connect_registered_rename(self: &Rc<Self>) {
        let button = self.root.get_control::<Button>("RenameRegisteredProperty");
        let flyout = Flyout::new();
        flyout.set_placement(PlacementMode::Top);
        flyout.popup().set_should_use_overlay_layer(true);
        flyout.flyout_presenter_classes().add("designer-chrome");
        let owner = Rc::downgrade(self);
        let popup = flyout.downgrade();
        flyout
            .opening(move |_| {
                if let (Some(workspace), Some(popup)) = (owner.upgrade(), popup.upgrade()) {
                    choose_name(&workspace, &popup);
                }
            })
            .detach();
        button.set_flyout(Some(flyout.upcast()));
    }
}

fn choose_name(workspace: &Rc<DesignerWorkspace>, flyout: &Ref<Flyout>) {
    let caret = workspace.source.caret_index();
    if workspace.pending_source.get() {
        workspace.apply_source();
    }
    let body = panel(workspace);
    flyout.set_content(Some(Control::boxed(body.clone())));
    let source = workspace.session.borrow();
    let offset = crate::workspace::source_tools::byte_from_utf16(source.authored_source(), caret);
    drop(source);
    let (anchor, target) = match workspace
        .registered_property_anchor(offset)
        .and_then(|anchor| {
            workspace
                .registered_property_target(&anchor)
                .map(|target| (anchor, target))
        }) {
        Ok(result) => result,
        Err(error) => {
            body.children().add(text(&error).upcast::<Control>());
            return;
        }
    };
    let input = TextBox::new();
    input.set_min_width(240.0);
    input.set_text(Some(&target.name));
    AutomationProperties::set_name(&input, Some("New public property name"));
    let row = toolbar();
    row.children().add(input.clone().upcast::<Control>());
    let status = text("");
    status.set_text_wrapping(TextWrapping::Wrap);
    AutomationProperties::set_live_setting(&status, AutomationLiveSetting::Polite);
    let (popup, name, message) = (flyout.downgrade(), input.downgrade(), status.downgrade());
    let owner = Rc::downgrade(workspace);
    let anchor = Rc::new(anchor);
    add_button(&row, SOURCE, "Review property rename", move || {
        let (Some(workspace), Some(popup), Some(name)) =
            (owner.upgrade(), popup.upgrade(), name.upgrade())
        else {
            return;
        };
        let name = name.text().unwrap_or_default().trim().to_owned();
        match workspace.review_registered_property(&anchor, &name) {
            Ok(plan) => show_plan(&workspace, &popup, anchor.clone(), plan),
            Err(error) => {
                if let Some(status) = message.upgrade() {
                    status.set_text(Some(&error));
                }
            }
        }
    });
    body.children().add(row.upcast::<Control>());
    body.children().add(status.upcast::<Control>());
}

fn show_plan(
    workspace: &Rc<DesignerWorkspace>,
    flyout: &Ref<Flyout>,
    anchor: Rc<RenameAnchor>,
    plan: RegisteredPropertyPlan,
) {
    let body = panel(workspace);
    body.children()
        .add(text(&format!("{}  →  {}", plan.name, plan.new_name)).upcast::<Control>());
    source_diffs(workspace, &body, &anchor.uri, Rc::new(plan.documents));
    let actions = toolbar();
    let status = text("");
    status.set_text_wrapping(TextWrapping::Wrap);
    AutomationProperties::set_live_setting(&status, AutomationLiveSetting::Polite);
    let (popup, message) = (flyout.downgrade(), status.downgrade());
    let owner = Rc::downgrade(workspace);
    let has_unmapped = !plan.unmapped.is_empty();
    let apply = if has_unmapped {
        "Apply mapped rename"
    } else {
        "Apply property rename"
    };
    add_button(&actions, icons::CHECK, apply, move || {
        let (Some(workspace), Some(popup)) = (owner.upgrade(), popup.upgrade()) else {
            return;
        };
        match workspace.apply_registered_property(&anchor, &plan.new_name, &plan.review) {
            Ok(_) => popup.hide(),
            Err(error) => {
                if let Some(status) = message.upgrade() {
                    status.set_text(Some(&error));
                }
            }
        }
    });
    let popup = flyout.downgrade();
    add_button(
        &actions,
        icons::CLOSE,
        "Cancel property rename",
        move || {
            if let Some(popup) = popup.upgrade() {
                popup.hide();
            }
        },
    );
    if has_unmapped {
        let row = toolbar();
        let choices = ComboBox::new();
        AutomationProperties::set_name(&choices, Some("Potential property references"));
        for location in &plan.unmapped {
            choices.items().add(Some(Rc::new(format!(
                "{}  ·  line {}",
                relative(workspace, &location.uri),
                location.range.start.line + 1
            ))));
        }
        choices.set_selected_index(0);
        row.children().add(choices.clone().upcast::<Control>());
        let owner = Rc::downgrade(workspace);
        let popup = flyout.downgrade();
        let choices = choices.downgrade();
        add_button(
            &row,
            SOURCE,
            "Reveal potential property reference",
            move || {
                if let (Some(workspace), Some(choices), Some(popup)) =
                    (owner.upgrade(), choices.upgrade(), popup.upgrade())
                {
                    if let Some(location) =
                        plan.unmapped.get(choices.selected_index().max(0) as usize)
                    {
                        popup.hide();
                        let result = workspace.navigate_to(location);
                        workspace.report(result);
                    }
                }
            },
        );
        body.children()
            .add(text("Potential references for manual review").upcast::<Control>());
        body.children().add(row.upcast::<Control>());
    }
    body.children().add(actions.upcast::<Control>());
    body.children().add(status.upcast::<Control>());
    flyout.set_content(Some(Control::boxed(body)));
}

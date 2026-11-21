use super::super::reviewed_sources::ui::{panel, relative, source_diffs, toolbar};
use super::{DesignerWorkspace, SourceReviewAnchor};
use crate::{
    icons,
    workspace::inspector_actions::{SOURCE, add_button, text},
};
use ferroui_base::{Ref, media::TextWrapping};
use ferroui_controls::{
    Button, ComboBox, Control, Flyout, PlacementMode, TextBlock,
    automation::{AutomationLiveSetting, AutomationProperties},
};
use ferroui_designer_language::{ResourceMoveDestination, ResourceMovePlan};
use std::rc::Rc;

impl DesignerWorkspace {
    pub(in super::super) fn connect_resource_move(self: &Rc<Self>) {
        let button = self.root.get_control::<Button>("MoveResource");
        let flyout = Flyout::new();
        flyout.set_placement(PlacementMode::Top);
        flyout.popup().set_should_use_overlay_layer(true);
        flyout.flyout_presenter_classes().add("designer-chrome");
        let (owner, popup) = (Rc::downgrade(self), flyout.downgrade());
        flyout
            .opening(move |_| {
                if let (Some(workspace), Some(flyout)) = (owner.upgrade(), popup.upgrade()) {
                    choose_destination(&workspace, &flyout);
                }
            })
            .detach();
        button.set_flyout(Some(flyout.upcast()));
    }
}

fn choose_destination(workspace: &Rc<DesignerWorkspace>, flyout: &Ref<Flyout>) {
    let caret = workspace.source.caret_index();
    if workspace.pending_source.get() {
        workspace.apply_source();
    }
    let body = panel(workspace);
    flyout.set_content(Some(Control::boxed(body.clone())));
    let offset = crate::workspace::source_tools::byte_from_utf16(
        workspace.session.borrow().authored_source(),
        caret,
    );
    let selection = workspace.source_review_anchor(offset).and_then(|anchor| {
        let target = workspace.resource_move_target(&anchor)?;
        Ok((Rc::new(anchor), target))
    });
    let (anchor, target) = match selection {
        Ok(selection) => selection,
        Err(error) => {
            body.children().add(status(&error).upcast::<Control>());
            return;
        }
    };
    body.children()
        .add(text(&format!("Move {}", target.name)).upcast::<Control>());
    if target.destinations.is_empty() {
        body.children().add(
            status("Create or open another resource dictionary in this project.")
                .upcast::<Control>(),
        );
        return;
    }
    let choices = ComboBox::new();
    AutomationProperties::set_name(&choices, Some("Destination resource dictionary"));
    for destination in &target.destinations {
        choices.items().add(Some(Rc::new(format!(
            "{} · {}:{}",
            destination.label,
            relative(workspace, &destination.uri),
            destination.range.start.line + 1,
        ))));
    }
    choices.set_selected_index(0);
    let actions = toolbar();
    let status = status("");
    let (owner, popup, choices_ref, message) = (
        Rc::downgrade(workspace),
        flyout.downgrade(),
        choices.downgrade(),
        status.downgrade(),
    );
    add_button(&actions, SOURCE, "Review resource move", move || {
        let (Some(workspace), Some(flyout), Some(choices)) =
            (owner.upgrade(), popup.upgrade(), choices_ref.upgrade())
        else {
            return;
        };
        let Some(destination) = usize::try_from(choices.selected_index())
            .ok()
            .and_then(|index| target.destinations.get(index))
        else {
            return;
        };
        match workspace.review_resource_move(&anchor, &destination.uri, &destination.request) {
            Ok(plan) => show_plan(
                &workspace,
                &flyout,
                anchor.clone(),
                destination.clone(),
                plan,
            ),
            Err(error) => {
                if let Some(message) = message.upgrade() {
                    message.set_text(Some(&error));
                }
            }
        }
    });
    body.children().add(choices.upcast::<Control>());
    body.children().add(actions.upcast::<Control>());
    body.children().add(status.upcast::<Control>());
}

fn show_plan(
    workspace: &Rc<DesignerWorkspace>,
    flyout: &Ref<Flyout>,
    anchor: Rc<SourceReviewAnchor>,
    destination: ResourceMoveDestination,
    plan: ResourceMovePlan,
) {
    let body = panel(workspace);
    let summary = format!("{} → {}", plan.name, relative(workspace, &destination.uri));
    body.children().add(text(&summary).upcast::<Control>());
    source_diffs(workspace, &body, &plan.from.uri, Rc::new(plan.documents));
    let actions = toolbar();
    let status = status("");
    let (owner, popup, message) = (
        Rc::downgrade(workspace),
        flyout.downgrade(),
        status.downgrade(),
    );
    add_button(&actions, icons::CHECK, "Apply resource move", move || {
        let (Some(workspace), Some(flyout)) = (owner.upgrade(), popup.upgrade()) else {
            return;
        };
        match workspace.apply_resource_move(
            &anchor,
            &destination.uri,
            &destination.request,
            &plan.review,
        ) {
            Ok(_) => flyout.hide(),
            Err(error) => {
                if let Some(message) = message.upgrade() {
                    message.set_text(Some(&error));
                }
            }
        }
    });
    let popup = flyout.downgrade();
    add_button(&actions, icons::CLOSE, "Cancel resource move", move || {
        if let Some(flyout) = popup.upgrade() {
            flyout.hide();
        }
    });
    body.children().add(actions.upcast::<Control>());
    body.children().add(status.upcast::<Control>());
    flyout.set_content(Some(Control::boxed(body)));
}

fn status(message: &str) -> Ref<TextBlock> {
    let status = text(message);
    status.set_text_wrapping(TextWrapping::Wrap);
    AutomationProperties::set_live_setting(&status, AutomationLiveSetting::Polite);
    status
}

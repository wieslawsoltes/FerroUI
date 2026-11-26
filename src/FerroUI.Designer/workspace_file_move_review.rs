use super::super::reviewed_sources::ui::{panel, relative, source_diffs, toolbar};
use super::{resolution::MoveTarget, DesignerWorkspace};
use crate::{
    icons,
    workspace::inspector_actions::{add_button, text, SOURCE},
};
use ferroui_base::{
    threading::{DispatcherPriority, DispatcherTimer},
    Ref,
};
use ferroui_controls::{
    automation::{AutomationLiveSetting, AutomationProperties},
    primitives::FlyoutBase,
    Button, ContentControl, Control, Flyout, PlacementMode, TextBlock, TextBox,
};
use ferroui_designer_language::FileMovePlan;
use std::{cell::Cell, rc::Rc, time::Duration};

impl DesignerWorkspace {
    pub(crate) fn connect_file_move(self: &Rc<Self>) {
        let button = self.root.get_control::<Button>("MoveXamlFile");
        let flyout = Flyout::new();
        flyout.set_placement(PlacementMode::Top);
        flyout.popup().set_should_use_overlay_layer(true);
        flyout.flyout_presenter_classes().add("designer-chrome");
        let owner = Rc::downgrade(self);
        let popup = flyout.downgrade();
        flyout
            .opening(move |_| {
                if let (Some(workspace), Some(popup)) = (owner.upgrade(), popup.upgrade()) {
                    choose_destination(&workspace, &popup);
                }
            })
            .detach();
        button.set_flyout(Some(flyout.upcast()));
    }
}

fn choose_destination(workspace: &Rc<DesignerWorkspace>, flyout: &Ref<Flyout>) {
    if workspace.pending_source.get() {
        workspace.apply_source();
    }
    let body = panel(workspace);
    flyout.set_content(Some(Control::boxed(body.clone())));
    let from = workspace.current_xaml_file();
    let from = match from {
        Ok(uri) => uri,
        Err(message) => {
            body.children().add(text(&message).upcast::<Control>());
            return;
        }
    };
    body.children()
        .add(text("Destination in the project").upcast::<Control>());
    let row = toolbar();
    let input = TextBox::new();
    input.set_min_width(280.0);
    input.set_text(Some(&relative(workspace, &from)));
    AutomationProperties::set_name(&input, Some("Destination XAML path"));
    row.children().add(input.clone().upcast::<Control>());
    let status = text("");
    status.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
    AutomationProperties::set_live_setting(&status, AutomationLiveSetting::Polite);
    let message = status.downgrade();
    let owner = Rc::downgrade(workspace);
    let popup = flyout.downgrade();
    let path = input.downgrade();
    let busy = Rc::new(Cell::new(false));
    add_button(&row, SOURCE, "Review file move", move || {
        if busy.get() {
            return;
        }
        let (Some(workspace), Some(popup), Some(input)) =
            (owner.upgrade(), popup.upgrade(), path.upgrade())
        else {
            return;
        };
        let destination = input.text().unwrap_or_default().trim().to_owned();
        let result = workspace.move_target(&destination).and_then(|target| {
            workspace
                .plan_project_file_move(&destination, true)
                .map(|plan| (target, plan))
        });
        match result {
            Ok((_, Some(plan))) => show_review(&workspace, &popup, destination, plan),
            Ok((target, None)) => {
                if let Some(message) = message.upgrade() {
                    message.set_text(Some("Resolving Rust references…"));
                    busy.set(true);
                    wait_for_review(
                        &workspace,
                        &popup,
                        &message,
                        &input,
                        target,
                        destination,
                        busy.clone(),
                    );
                }
            }
            Err(error) => {
                if let Some(message) = message.upgrade() {
                    message.set_text(Some(&error));
                }
            }
        }
    });
    body.children().add(row.upcast::<Control>());
    body.children().add(status.upcast::<Control>());
}

fn wait_for_review(
    workspace: &Rc<DesignerWorkspace>,
    flyout: &Ref<Flyout>,
    message: &Ref<TextBlock>,
    input: &Ref<TextBox>,
    target: MoveTarget,
    destination: String,
    busy: Rc<Cell<bool>>,
) {
    let owner = Rc::downgrade(workspace);
    let (flyout, message, input) = (flyout.downgrade(), message.downgrade(), input.downgrade());
    DispatcherTimer::run(
        move || {
            let Some(workspace) = owner.upgrade() else {
                return false;
            };
            let controls = flyout
                .upgrade()
                .filter(|flyout| flyout.is_open())
                .zip(message.upgrade());
            let same_input = input
                .upgrade()
                .and_then(|input| input.text())
                .is_some_and(|path| path.trim() == destination);
            if let Some((flyout, message)) = controls.filter(|_| {
                same_input && workspace.move_target(&destination).as_ref().ok() == Some(&target)
            }) {
                match workspace.plan_project_file_move(&destination, false) {
                    Ok(None) => return true,
                    Ok(Some(plan)) => show_review(&workspace, &flyout, destination.clone(), plan),
                    Err(error) => message.set_text(Some(&error)),
                }
            } else {
                workspace.cancel_move_resolution(&target);
                if let Some(message) = message.upgrade() {
                    message.set_text(Some(""));
                }
            }
            busy.set(false);
            false
        },
        Duration::from_millis(120),
        DispatcherPriority::BACKGROUND,
    )
    .detach();
}

fn show_review(
    workspace: &Rc<DesignerWorkspace>,
    flyout: &Ref<Flyout>,
    destination: String,
    plan: FileMovePlan,
) {
    let target = workspace.move_target(&destination).ok();
    show_move_review(
        workspace,
        flyout,
        plan,
        move |workspace, review| workspace.apply_project_file_move(&destination, review),
        move |workspace| {
            if let Some(target) = &target {
                workspace.cancel_move_resolution(target);
            }
        },
    );
}

impl DesignerWorkspace {
    pub(crate) fn show_asset_move(self: &Rc<Self>, address: String) -> Result<(), String> {
        let from = self.published_asset_source(&address)?;
        let anchor = self
            .root
            .get_control::<ContentControl>("AssetHost")
            .upcast::<Control>();
        let flyout = Flyout::new();
        flyout.set_placement(PlacementMode::Right);
        flyout.popup().set_should_use_overlay_layer(true);
        flyout.flyout_presenter_classes().add("designer-chrome");
        let body = panel(self);
        body.children()
            .add(text("Destination in the project").upcast::<Control>());
        let row = toolbar();
        let input = TextBox::new();
        input.set_min_width(280.0);
        input.set_text(Some(&relative(self, &from)));
        AutomationProperties::set_name(&input, Some("Destination asset path"));
        row.children().add(input.clone().upcast::<Control>());
        let status = text("");
        status.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
        AutomationProperties::set_live_setting(&status, AutomationLiveSetting::Polite);
        let (owner, popup, path, message) = (
            Rc::downgrade(self),
            flyout.downgrade(),
            input.downgrade(),
            status.downgrade(),
        );
        add_button(&row, SOURCE, "Review asset move", move || {
            let (Some(workspace), Some(popup), Some(input)) =
                (owner.upgrade(), popup.upgrade(), path.upgrade())
            else {
                return;
            };
            let destination = input.text().unwrap_or_default().trim().to_owned();
            match workspace.plan_project_asset_move(&address, &destination) {
                Ok(plan) => {
                    let asset = address.clone();
                    show_move_review(
                        &workspace,
                        &popup,
                        plan,
                        move |workspace, review| {
                            workspace.apply_project_asset_move(&asset, &destination, review)
                        },
                        |_| {},
                    );
                }
                Err(error) => {
                    if let Some(message) = message.upgrade() {
                        message.set_text(Some(&error));
                    }
                }
            }
        });
        body.children().add(row.upcast::<Control>());
        body.children().add(status.upcast::<Control>());
        flyout.set_content(Some(Control::boxed(body)));
        FlyoutBase::set_attached_flyout(&anchor, Some(flyout.upcast()));
        FlyoutBase::show_attached_flyout(&anchor);
        Ok(())
    }
}

fn show_move_review(
    workspace: &Rc<DesignerWorkspace>,
    flyout: &Ref<Flyout>,
    plan: FileMovePlan,
    apply: impl Fn(&Rc<DesignerWorkspace>, &str) -> Result<FileMovePlan, String> + 'static,
    cancel: impl Fn(&Rc<DesignerWorkspace>) + 'static,
) {
    let body = panel(workspace);
    let title = format!(
        "{}  →  {}",
        relative(workspace, &plan.from),
        relative(workspace, &plan.to)
    );
    body.children().add(text(&title).upcast::<Control>());
    for address in &plan.published_assets {
        body.children()
            .add(text(&format!("{}  →  {}", address.from, address.to)).upcast::<Control>());
    }
    source_diffs(workspace, &body, &plan.from, Rc::new(plan.documents));
    let review = plan.review;
    let actions = toolbar();
    let status = text("");
    status.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
    let popup = flyout.downgrade();
    let owner = Rc::downgrade(workspace);
    let message = status.downgrade();
    add_button(&actions, icons::CHECK, "Apply file move", move || {
        let (Some(workspace), Some(popup)) = (owner.upgrade(), popup.upgrade()) else {
            return;
        };
        match apply(&workspace, &review) {
            Ok(_) => popup.hide(),
            Err(error) => {
                if let Some(message) = message.upgrade() {
                    message.set_text(Some(&error));
                }
            }
        }
    });
    let popup = flyout.downgrade();
    let owner = Rc::downgrade(workspace);
    add_button(&actions, icons::CLOSE, "Cancel file move", move || {
        if let Some(workspace) = owner.upgrade() {
            cancel(&workspace);
        }
        if let Some(popup) = popup.upgrade() {
            popup.hide();
        }
    });
    body.children().add(actions.upcast::<Control>());
    body.children().add(status.upcast::<Control>());
    flyout.set_content(Some(Control::boxed(body)));
}

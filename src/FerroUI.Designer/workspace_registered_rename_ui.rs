use super::super::reviewed_sources::ui::{panel, relative, source_diffs, toolbar};
use super::{DesignerWorkspace, SourceReviewAnchor};
use crate::{
    icons,
    workspace::inspector_actions::{add_button, text, SOURCE},
};
use ferroui_base::{
    media::TextWrapping,
    threading::{DispatcherPriority, DispatcherTimer},
    Ref,
};
use ferroui_controls::{
    automation::{AutomationLiveSetting, AutomationProperties},
    Button, ComboBox, Control, Flyout, PlacementMode, TextBlock, TextBox,
};
use ferroui_designer_core::NodeId;
use ferroui_designer_language::SourceRenamePlan;
use std::{cell::{Cell, RefCell}, rc::Rc, time::Duration};

#[derive(Clone, Copy)]
enum Subject {
    Property,
    Type,
    Resource,
}

impl Subject {
    fn label(self) -> &'static str {
        match self {
            Self::Property => "property",
            Self::Type => "type",
            Self::Resource => "resource",
        }
    }

    fn target_name(
        self,
        workspace: &DesignerWorkspace,
        anchor: &SourceReviewAnchor,
    ) -> Result<String, String> {
        match self {
            Self::Property => workspace
                .registered_property_target(anchor)
                .map(|target| target.name),
            Self::Type => workspace
                .try_language_service()?
                .registered_type_target(&anchor.uri, &anchor.request)
                .map(|target| target.name)
                .map_err(|error| error.to_string()),
            Self::Resource => workspace
                .try_language_service()?
                .resource_rename_target(&anchor.uri, &anchor.request)
                .map(|target| target.name)
                .map_err(|error| error.to_string()),
        }
    }

    fn review(
        self,
        workspace: &DesignerWorkspace,
        anchor: &SourceReviewAnchor,
        name: &str,
    ) -> Result<Option<SourceRenamePlan>, String> {
        match self {
            Self::Property => workspace.review_registered_property(anchor, name).map(Some),
            Self::Type => workspace.review_registered_type(anchor, name, true),
            Self::Resource => workspace.review_resource_rename(anchor, name).map(Some),
        }
    }

    fn apply(
        self,
        workspace: &Rc<DesignerWorkspace>,
        anchor: &SourceReviewAnchor,
        name: &str,
        review: &str,
    ) -> Result<SourceRenamePlan, String> {
        match self {
            Self::Property => workspace.apply_registered_property(anchor, name, review),
            Self::Type => workspace.apply_registered_type(anchor, name, review),
            Self::Resource => workspace.apply_resource_rename(anchor, name, review),
        }
    }
}

impl DesignerWorkspace {
    pub(in super::super) fn connect_registered_rename(self: &Rc<Self>) {
        self.connect_rename_flyout("RenameRegisteredProperty", Subject::Property);
        self.connect_rename_flyout("RenameRegisteredType", Subject::Type);
        self.connect_rename_flyout("RenameResource", Subject::Resource);
    }

    fn connect_rename_flyout(self: &Rc<Self>, name: &str, subject: Subject) {
        let button = self.root.get_control::<Button>(name);
        let flyout = rename_flyout();
        let owner = Rc::downgrade(self);
        let popup = flyout.downgrade();
        flyout
            .opening(move |_| {
                if let (Some(workspace), Some(popup)) = (owner.upgrade(), popup.upgrade()) {
                    choose_name(&workspace, &popup, subject);
                }
            })
            .detach();
        button.set_flyout(Some(flyout.upcast()));
    }

    pub(in super::super) fn show_library_resource_rename(
        self: &Rc<Self>,
        resource: NodeId,
        new_key: &str,
    ) -> Result<(), String> {
        let offset = self.session.borrow().document().ast().node(resource)
            .ok_or("The resource definition changed")?.name_span.start;
        let anchor = Rc::new(self.source_review_anchor(offset)?);
        let plan = self.review_resource_rename(&anchor, new_key)?;
        let flyout = rename_flyout();
        show_plan(self, &flyout, Subject::Resource, anchor, plan);
        let keep = Rc::new(RefCell::new(Some(flyout.clone())));
        flyout.closed(move || { keep.borrow_mut().take(); }).detach();
        flyout.show_at(&self.root.get_control::<Button>("CloseAuthoring"));
        Ok(())
    }
}

fn rename_flyout() -> Ref<Flyout> {
    let flyout = Flyout::new();
    flyout.set_placement(PlacementMode::Top);
    flyout.popup().set_should_use_overlay_layer(true);
    flyout.flyout_presenter_classes().add("designer-chrome");
    flyout
}

fn choose_name(workspace: &Rc<DesignerWorkspace>, flyout: &Ref<Flyout>, subject: Subject) {
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
        .source_review_anchor(offset)
        .and_then(|anchor| {
            subject
                .target_name(workspace, &anchor)
                .map(|name| (anchor, name))
        }) {
        Ok(result) => result,
        Err(error) => {
            body.children().add(text(&error).upcast::<Control>());
            return;
        }
    };
    let input = TextBox::new();
    input.set_min_width(240.0);
    input.set_text(Some(&target));
    AutomationProperties::set_name(
        &input,
        Some(match subject {
            Subject::Resource => "New resource key",
            Subject::Property => "New public property name",
            Subject::Type => "New public type name",
        }),
    );
    let row = toolbar();
    row.children().add(input.clone().upcast::<Control>());
    let status = text("");
    status.set_text_wrapping(TextWrapping::Wrap);
    AutomationProperties::set_live_setting(&status, AutomationLiveSetting::Polite);
    let (popup, name, message) = (flyout.downgrade(), input.downgrade(), status.downgrade());
    let owner = Rc::downgrade(workspace);
    let anchor = Rc::new(anchor);
    let busy = Rc::new(Cell::new(false));
    add_button(
        &row,
        SOURCE,
        &format!("Review {} rename", subject.label()),
        move || {
            if busy.get() {
                return;
            }
            let (Some(workspace), Some(popup), Some(name)) =
                (owner.upgrade(), popup.upgrade(), name.upgrade())
            else {
                return;
            };
            let new_name = name.text().unwrap_or_default().trim().to_owned();
            match subject.review(&workspace, &anchor, &new_name) {
                Ok(Some(plan)) => show_plan(&workspace, &popup, subject, anchor.clone(), plan),
                Ok(None) => {
                    if let Some(message) = message.upgrade() {
                        message.set_text(Some("Resolving Rust references…"));
                        busy.set(true);
                        poll_type_review(
                            &workspace,
                            &popup,
                            &name,
                            &message,
                            anchor.clone(),
                            new_name,
                            busy.clone(),
                        );
                    }
                }
                Err(error) => {
                    if let Some(status) = message.upgrade() {
                        status.set_text(Some(&error));
                    }
                }
            }
        },
    );
    body.children().add(row.upcast::<Control>());
    body.children().add(status.upcast::<Control>());
}

fn poll_type_review(
    workspace: &Rc<DesignerWorkspace>,
    popup: &Ref<Flyout>,
    name: &Ref<TextBox>,
    message: &Ref<TextBlock>,
    anchor: Rc<SourceReviewAnchor>,
    new_name: String,
    busy: Rc<Cell<bool>>,
) {
    let owner = Rc::downgrade(workspace);
    let (popup, name, message) = (popup.downgrade(), name.downgrade(), message.downgrade());
    DispatcherTimer::run(
        move || {
            let Some(workspace) = owner.upgrade() else {
                return false;
            };
            let controls = popup
                .upgrade()
                .filter(|popup| popup.is_open())
                .zip(message.upgrade());
            let same_input = name
                .upgrade()
                .and_then(|name| name.text())
                .is_some_and(|name| name.trim() == new_name);
            if let Some((popup, message)) = controls.filter(|_| same_input) {
                match workspace.review_registered_type(&anchor, &new_name, false) {
                    Ok(None) => return true,
                    Ok(Some(plan)) => {
                        show_plan(&workspace, &popup, Subject::Type, anchor.clone(), plan)
                    }
                    Err(error) => {
                        workspace.cancel_registered_type(&anchor, &new_name);
                        message.set_text(Some(&error));
                    }
                }
            } else {
                workspace.cancel_registered_type(&anchor, &new_name);
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

fn show_plan(
    workspace: &Rc<DesignerWorkspace>,
    flyout: &Ref<Flyout>,
    subject: Subject,
    anchor: Rc<SourceReviewAnchor>,
    plan: SourceRenamePlan,
) {
    let body = panel(workspace);
    body.children()
        .add(text(&format!("{}  →  {}", plan.name, plan.new_name)).upcast::<Control>());
    if let Some(family) = &plan.theme_family {
        let layers = family.members.iter().flat_map(|member| member.layers.iter().map(String::as_str)).collect::<Vec<_>>();
        let label = text(&format!("Theme family · {}", layers.join(", ")));
        label.set_text_wrapping(TextWrapping::Wrap);
        body.children().add(label.upcast::<Control>());
    }
    source_diffs(workspace, &body, &anchor.uri, Rc::new(plan.documents));
    let actions = toolbar();
    let status = text("");
    status.set_text_wrapping(TextWrapping::Wrap);
    AutomationProperties::set_live_setting(&status, AutomationLiveSetting::Polite);
    let (popup, message) = (flyout.downgrade(), status.downgrade());
    let owner = Rc::downgrade(workspace);
    let has_unmapped = !plan.unmapped.is_empty();
    let apply = if has_unmapped {
        "Apply mapped rename".into()
    } else {
        format!("Apply {} rename", subject.label())
    };
    let apply_anchor = anchor.clone();
    let new_name = plan.new_name.clone();
    add_button(&actions, icons::CHECK, &apply, move || {
        let (Some(workspace), Some(popup)) = (owner.upgrade(), popup.upgrade()) else {
            return;
        };
        match subject.apply(&workspace, &apply_anchor, &new_name, &plan.review) {
            Ok(_) => popup.hide(),
            Err(error) => {
                if let Some(status) = message.upgrade() {
                    status.set_text(Some(&error));
                }
            }
        }
    });
    let popup = flyout.downgrade();
    let owner = Rc::downgrade(workspace);
    add_button(
        &actions,
        icons::CLOSE,
        &format!("Cancel {} rename", subject.label()),
        move || {
            if matches!(subject, Subject::Type) {
                if let Some(workspace) = owner.upgrade() {
                    workspace.cancel_registered_type(&anchor, &plan.new_name);
                }
            }
            if let Some(popup) = popup.upgrade() {
                popup.hide();
            }
        },
    );
    if has_unmapped {
        let row = toolbar();
        let choices = ComboBox::new();
        AutomationProperties::set_name(
            &choices,
            Some(&format!("Potential {} references", subject.label())),
        );
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
            &format!("Reveal potential {} reference", subject.label()),
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

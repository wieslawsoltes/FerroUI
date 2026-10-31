//! Native sharing controls. Transport, authentication and persistence belong to SharingController.
use super::*;
use crate::sharing::{SharingController, SharingOptions, SharingPhase, SharingStatus};
use ferroui_controls::{Border, ColumnDefinitions, Expander, ScrollViewer};
use ferroui_designer_collaboration_host::Role;

pub(super) struct SharingPanel {
    directory: Ref<TextBox>,
    display_name: Ref<TextBox>,
    bind: Ref<TextBox>,
    public_url: Ref<TextBox>,
    join: Ref<TextBox>,
    status: Ref<TextBlock>,
    invitation: Ref<TextBox>,
    members: Ref<StackPanel>,
    review: Ref<StackPanel>,
    remote: Ref<TextBox>,
    resolved: Ref<TextBox>,
    restore_draft: Ref<Button>,
    review_source: RefCell<Option<String>>,
    review_revision: Cell<u64>,
    fingerprint: RefCell<String>,
    start_buttons: Vec<Ref<Button>>,
    owner_buttons: Vec<Ref<Button>>,
    session_details: Ref<StackPanel>,
}
fn label(panel: &StackPanel, text: &str) -> Ref<TextBlock> {
    let label = TextBlock::new();
    label.set_text(Some(text));
    label.set_foreground(Some(SolidColorBrush::from_uint32(0xffa9adba).into()));
    label.set_font_size(12.0);
    label.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
    panel.children().add(label.clone().upcast::<Control>());
    label
}
fn field(panel: &StackPanel, title: &str, text: &str) -> Ref<TextBox> {
    let row = Grid::new();
    row.set_column_definitions(ColumnDefinitions::parse("146,*").unwrap());
    let title_control = TextBlock::new();
    title_control.set_text(Some(title));
    title_control.set_font_size(11.0);
    title_control.set_foreground(Some(SolidColorBrush::from_uint32(0xffa9adba).into()));
    title_control.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Center);
    title_control.set_margin(Thickness::new(0.0, 0.0, 10.0, 0.0));
    row.children().add(title_control.upcast::<Control>());
    let field = TextBox::new();
    field.set_text(Some(text));
    ferroui_controls::automation::AutomationProperties::set_name(&field, Some(title));
    Grid::set_column(&field, 1);
    row.children().add(field.clone().upcast::<Control>());
    panel.children().add(row.upcast::<Control>());
    field
}
fn section(panel: &StackPanel, title: &str) -> Ref<StackPanel> {
    let section = StackPanel::new();
    section.set_spacing(8.0);
    let heading = label(&section, title);
    heading.set_font_weight(ferroui_base::media::FontWeight::SemiBold);
    heading.set_foreground(Some(SolidColorBrush::from_uint32(0xfff2f3f5).into()));
    heading.set_margin(Thickness::new(0.0, 5.0, 0.0, 3.0));
    panel.children().add(section.clone().upcast::<Control>());
    section
}
fn row(panel: &StackPanel) -> Ref<StackPanel> {
    let row = StackPanel::new();
    row.set_orientation(Orientation::Horizontal);
    row.set_spacing(6.0);
    panel.children().add(row.clone().upcast::<Control>());
    row
}
fn action(
    panel: &StackPanel,
    workspace: &Rc<DesignerWorkspace>,
    title: &str,
    run: impl Fn(&Rc<DesignerWorkspace>) -> Result<(), String> + 'static,
) -> Ref<Button> {
    let control = button(title);
    let weak = Rc::downgrade(workspace);
    control.click(move |_, _| {
        if let Some(workspace) = weak.upgrade() {
            let result = run(&workspace);
            workspace.refresh_sharing_panel();
            workspace.report(result);
        }
    });
    panel.children().add(control.clone().upcast::<Control>());
    control
}
impl SharingPanel {
    fn options(&self) -> Result<SharingOptions, String> {
        let directory = self.directory.text().unwrap_or_default();
        if directory.trim().is_empty() {
            return Err("Choose a directory for durable sharing state".into());
        }
        let bind = self.bind.text().unwrap_or_default();
        Ok(SharingOptions {
            directory: directory.into(),
            display_name: self.display_name.text().unwrap_or_default(),
            bind: if bind.trim().is_empty() {
                None
            } else {
                Some(bind.parse().map_err(
                    |_| "Bind address must be an IP address and port, for example 127.0.0.1:0",
                )?)
            },
            public_url: self
                .public_url
                .text()
                .filter(|text| !text.trim().is_empty()),
        })
    }
}
impl DesignerWorkspace {
    pub(super) fn connect_sharing(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.root.get_control::<Button>("Share").click(move |_, _| {
            if let Some(workspace) = weak.upgrade() {
                workspace.show_sharing();
            }
        });
    }
    pub fn sharing_status(&self) -> Option<SharingStatus> {
        self.sharing
            .borrow()
            .as_ref()
            .map(|controller| controller.status())
    }
    pub fn sharing_controller(&self) -> Option<Rc<SharingController>> {
        self.sharing.borrow().clone()
    }
    pub fn host_sharing(self: &Rc<Self>, options: SharingOptions) -> Result<(), String> {
        self.ensure_sharing_detached()?;
        if self.pending_source.get() {
            self.apply_source();
        }
        *self.sharing.borrow_mut() = Some(SharingController::host(&self.session, options)?);
        self.refresh_sharing_panel();
        Ok(())
    }
    pub fn join_sharing(
        self: &Rc<Self>,
        invitation: String,
        options: SharingOptions,
    ) -> Result<(), String> {
        self.ensure_sharing_detached()?;
        *self.sharing.borrow_mut() =
            Some(SharingController::join(&self.session, invitation, options)?);
        self.refresh_sharing_panel();
        Ok(())
    }
    pub fn resume_sharing(self: &Rc<Self>, options: SharingOptions) -> Result<(), String> {
        self.ensure_sharing_detached()?;
        *self.sharing.borrow_mut() = Some(SharingController::resume(&self.session, options)?);
        self.refresh_sharing_panel();
        Ok(())
    }
    fn ensure_sharing_detached(&self) -> Result<(), String> {
        if self
            .sharing
            .borrow()
            .as_ref()
            .is_some_and(|controller| controller.status().phase != SharingPhase::Disconnected)
        {
            return Err("Leave the current room before starting another connection".into());
        }
        Ok(())
    }
    pub fn disconnect_sharing(&self) {
        if let Some(controller) = self.sharing.borrow().as_ref() {
            controller.disconnect();
        }
        self.refresh_sharing_panel();
    }
    pub fn show_sharing(self: &Rc<Self>) {
        let host = self.root.get_control::<ContentControl>("SharingHost");
        if self.sharing_panel.borrow().is_none() {
            let options = SharingOptions::default();
            let body = StackPanel::new();
            body.set_spacing(12.0);
            let title = label(&body, "Share document");
            title.set_font_size(19.0);
            title.set_font_weight(ferroui_base::media::FontWeight::SemiBold);
            title.set_foreground(Some(SolidColorBrush::from_uint32(0xfff2f3f5).into()));
            let status = label(&body, "Disconnected");
            let display_name = field(&body, "Display name", &options.display_name);
            let advanced = StackPanel::new();
            advanced.set_spacing(8.0);
            advanced.set_margin(Thickness::new(0.0, 8.0, 0.0, 8.0));
            let directory = field(
                &advanced,
                "Session storage",
                &options.directory.display().to_string(),
            );
            let bind = field(&advanced, "Host address", "");
            bind.set_placeholder_text(Some("Automatic · loopback with an available port"));
            let public_url = field(&advanced, "Public endpoint", "");
            public_url.set_placeholder_text(Some("Optional HTTPS endpoint"));
            let connection = Expander::new();
            connection.set_header(Some(Rc::new("Connection options".to_owned())));
            connection.set_content(Some(Control::boxed(advanced)));
            connection.set_horizontal_alignment(HorizontalAlignment::Stretch);
            connection.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
            body.children().add(connection.upcast::<Control>());
            let start = section(&body, "Start or join a session");
            let join = field(&start, "Invitation", "");
            join.set_placeholder_text(Some("Paste an invitation to join a document"));
            join.set_accepts_return(true);
            join.set_max_height(90.0);
            let commands = row(&start);
            let start_buttons = vec![
                action(&commands, self, "Host document", |workspace| {
                    let panel = workspace
                        .sharing_panel
                        .borrow()
                        .clone()
                        .ok_or("Sharing panel closed")?;
                    workspace.host_sharing(panel.options()?)
                }),
                action(&commands, self, "Join invitation", |workspace| {
                    let panel = workspace
                        .sharing_panel
                        .borrow()
                        .clone()
                        .ok_or("Sharing panel closed")?;
                    workspace.join_sharing(panel.join.text().unwrap_or_default(), panel.options()?)
                }),
                action(&commands, self, "Resume saved session", |workspace| {
                    let panel = workspace
                        .sharing_panel
                        .borrow()
                        .clone()
                        .ok_or("Sharing panel closed")?;
                    workspace.resume_sharing(panel.options()?)
                }),
            ];
            start_buttons[0].classes().add("primary");
            let session_details = section(&body, "Session access");
            session_details.set_is_visible(false);
            let controls = row(&session_details);
            let owner_buttons = vec![
                action(&controls, self, "Invite editor", |workspace| {
                    workspace
                        .sharing_controller()
                        .ok_or("Start sharing first")?
                        .invite(Role::Editor)
                }),
                action(&controls, self, "Invite viewer", |workspace| {
                    workspace
                        .sharing_controller()
                        .ok_or("Start sharing first")?
                        .invite(Role::Viewer)
                }),
                action(&controls, self, "Checkpoint current source", |workspace| {
                    workspace
                        .sharing_controller()
                        .ok_or("Start sharing first")?
                        .checkpoint()
                }),
            ];
            action(&controls, self, "Refresh", |workspace| {
                workspace
                    .sharing_controller()
                    .ok_or("Start sharing first")?
                    .refresh_room()
            });
            action(&controls, self, "Leave", |workspace| {
                workspace.disconnect_sharing();
                Ok(())
            });
            label(
                &session_details,
                "Checkpoint saves a shared baseline after pending edits synchronize.",
            )
            .set_font_size(11.0);
            let invitation = field(&session_details, "Share invitation", "");
            invitation
                .set_placeholder_text(Some("Create an editor or viewer invitation, then copy it"));
            invitation.set_is_read_only(true);
            invitation.set_accepts_return(true);
            invitation.set_max_height(90.0);
            label(&session_details, "People")
                .set_font_weight(ferroui_base::media::FontWeight::SemiBold);
            let members = StackPanel::new();
            members.set_spacing(5.0);
            session_details
                .children()
                .add(members.clone().upcast::<Control>());
            let review = StackPanel::new();
            review.set_spacing(6.0);
            review.set_is_visible(false);
            body.children().add(review.clone().upcast::<Control>());
            label(&review, "Review incoming document").set_font_size(15.0);
            label(
                &review,
                "Review the remote snapshot and your local source. Acceptance checks the captured local revision and creates a durable backup.",
            );
            let remote = field(&review, "Incoming XAML", "");
            remote.set_is_read_only(true);
            remote.set_accepts_return(true);
            remote.set_height(140.0);
            remote.set_font_family(ferroui_base::media::FontFamily::new("monospace"));
            let resolved = field(&review, "Merged XAML", "");
            resolved.set_accepts_return(true);
            resolved.set_height(140.0);
            let review_actions = row(&review);
            action(&review_actions, self, "Use remote snapshot", |workspace| {
                let panel = workspace
                    .sharing_panel
                    .borrow()
                    .clone()
                    .ok_or("Review closed")?;
                let controller = workspace
                    .sharing_controller()
                    .ok_or("Start sharing first")?;
                let source = controller
                    .pending_source()
                    .ok_or("No incoming snapshot is pending")?;
                controller.accept_snapshot(panel.review_revision.get(), Some(source))
            });
            action(&review_actions, self, "Use resolved source", |workspace| {
                let panel = workspace
                    .sharing_panel
                    .borrow()
                    .clone()
                    .ok_or("Review closed")?;
                workspace
                    .sharing_controller()
                    .ok_or("Start sharing first")?
                    .accept_snapshot(
                        panel.review_revision.get(),
                        Some(panel.resolved.text().unwrap_or_default()),
                    )
            });
            let restore_draft = action(&review_actions, self, "Restore saved draft", |workspace| {
                let panel = workspace
                    .sharing_panel
                    .borrow()
                    .clone()
                    .ok_or("Review closed")?;
                let controller = workspace
                    .sharing_controller()
                    .ok_or("Start sharing first")?;
                if controller.pending_local_source().is_none() {
                    return Err("There is no saved local draft to restore".into());
                }
                controller.accept_snapshot(panel.review_revision.get(), None)
            });
            action(&review_actions, self, "Refresh local draft", |workspace| {
                let panel = workspace
                    .sharing_panel
                    .borrow()
                    .clone()
                    .ok_or("Review closed")?;
                let session = workspace.session.borrow();
                panel.review_revision.set(session.document().revision());
                panel.resolved.set_text(Some(session.document().source()));
                Ok(())
            });
            let close = action(&body, self, "Close panel", |workspace| {
                workspace
                    .root
                    .get_control::<ContentControl>("SharingHost")
                    .set_is_visible(false);
                Ok(())
            });
            close.classes().add("nav");
            close.set_horizontal_alignment(HorizontalAlignment::Right);
            let scroll = ScrollViewer::new();
            scroll.set_max_height(650.0);
            scroll.set_content(Some(Control::boxed(body)));
            let frame = Border::new();
            frame.set_width(672.0);
            frame.set_padding(Thickness::uniform(20.0));
            frame.set_background(Some(SolidColorBrush::from_uint32(0xff25262b).into()));
            frame.set_border_brush(Some(SolidColorBrush::from_uint32(0xff3b3d46).into()));
            frame.set_border_thickness(Thickness::uniform(1.0));
            frame.set_corner_radius(ferroui_base::CornerRadius::uniform(8.0));
            frame.classes().add("designer-chrome");
            frame.set_child(Some(scroll.upcast::<Control>()));
            host.set_content(Some(Control::boxed(frame)));
            *self.sharing_panel.borrow_mut() = Some(Rc::new(SharingPanel {
                directory,
                display_name,
                bind,
                public_url,
                join,
                status,
                invitation,
                members,
                review,
                remote,
                resolved,
                restore_draft,
                review_source: RefCell::new(None),
                review_revision: Cell::new(0),
                fingerprint: RefCell::new(String::new()),
                start_buttons,
                owner_buttons,
                session_details,
            }));
        }
        host.set_is_visible(true);
        self.refresh_sharing_panel();
    }
    pub(super) fn tick_sharing(self: &Rc<Self>) {
        let controller = self.sharing_controller();
        if let Some(controller) = controller {
            if let Err(error) = controller.set_cursor(self.surface.cursor_position().map(|point| {
                ferroui_designer_collaboration::Cursor {
                    x: point.x,
                    y: point.y,
                }
            })) {
                self.report(Err(error));
            }
            match controller.poll() {
                Ok(changed) => {
                    if changed {
                        self.refresh(false);
                    }
                }
                Err(error) => self.report(Err(error)),
            }
            self.refresh_sharing_panel();
        }
    }
    fn refresh_sharing_panel(&self) {
        let Some(panel) = self.sharing_panel.borrow().clone() else {
            return;
        };
        let Some(controller) = self.sharing_controller() else {
            panel.status.set_text(Some("Disconnected"));
            panel.session_details.set_is_visible(false);
            for button in &panel.owner_buttons {
                button.set_is_enabled(false);
            }
            for button in &panel.start_buttons {
                button.set_is_enabled(true);
            }
            return;
        };
        let status = controller.status();
        panel
            .session_details
            .set_is_visible(status.phase != SharingPhase::Disconnected);
        let fingerprint = format!("{:?}", status);
        if *panel.fingerprint.borrow() != fingerprint {
            *panel.fingerprint.borrow_mut() = fingerprint;
            let phase = match status.phase {
                SharingPhase::Connecting => "Connecting",
                SharingPhase::AwaitingReview => "Review incoming document",
                SharingPhase::Connected => "Connected",
                SharingPhase::Reconnecting => "Reconnecting",
                SharingPhase::Conflict => "Review conflicting edits",
                SharingPhase::Disconnected => "Disconnected",
            };
            let role = status
                .role
                .map(|role| format!(" · {}", role_label(role)))
                .unwrap_or_default();
            let edits = if status.pending_operations == 0 {
                String::new()
            } else {
                format!(" · {} pending edits", status.pending_operations)
            };
            panel.status.set_text(Some(&format!(
                "{phase}{role}{edits}\n{}{}",
                status.message,
                status
                    .backup_path
                    .as_ref()
                    .map(|path| format!("\nLocal backup: {}", path.display()))
                    .unwrap_or_default()
            )));
            let disconnected = status.phase == SharingPhase::Disconnected;
            for button in &panel.start_buttons {
                button.set_is_enabled(disconnected);
            }
            for button in &panel.owner_buttons {
                button.set_is_enabled(
                    status.phase == SharingPhase::Connected && status.role == Some(Role::Owner),
                );
            }
            panel.invitation.set_text(status.invitation.as_deref());
            panel.members.children().clear();
            for member in &status.members {
                let member_row = row(&panel.members);
                label(
                    &member_row,
                    &format!(
                        "{} · {}{}",
                        member.display_name,
                        role_label(member.role),
                        if member.revoked { " · revoked" } else { "" }
                    ),
                );
                for (title, role, revoked) in [
                    ("Owner", Role::Owner, false),
                    ("Editor", Role::Editor, false),
                    ("Viewer", Role::Viewer, false),
                    (
                        if member.revoked { "Restore" } else { "Revoke" },
                        member.role,
                        !member.revoked,
                    ),
                ] {
                    let control = button(title);
                    control.set_is_enabled(
                        status.role == Some(Role::Owner) && status.phase == SharingPhase::Connected,
                    );
                    let controller = Rc::downgrade(&controller);
                    let actor = member.actor.clone();
                    let status_label = panel.status.downgrade();
                    control.click(move |_, _| {
                        if let Some(controller) = controller.upgrade() {
                            if let Err(error) = controller.set_member(actor.clone(), role, revoked)
                            {
                                if let Some(label) = status_label.upgrade() {
                                    label.set_text(Some(&error));
                                }
                            }
                        }
                    });
                    member_row.children().add(control.upcast::<Control>());
                }
            }
        }
        let pending = controller.pending_source();
        panel.review.set_is_visible(pending.is_some());
        panel.restore_draft.set_is_enabled(
            controller.pending_local_source().is_some()
                && status.role.is_some_and(|role| role != Role::Viewer),
        );
        if *panel.review_source.borrow() != pending {
            *panel.review_source.borrow_mut() = pending.clone();
            if let Some(source) = pending {
                panel.remote.set_text(Some(&source));
                let session = self.session.borrow();
                panel.review_revision.set(session.document().revision());
                panel.resolved.set_text(Some(
                    controller
                        .pending_local_source()
                        .as_deref()
                        .unwrap_or(session.document().source()),
                ));
            }
        }
    }
}

fn role_label(role: Role) -> &'static str {
    match role {
        Role::Owner => "Owner",
        Role::Editor => "Editor",
        Role::Viewer => "Viewer",
    }
}

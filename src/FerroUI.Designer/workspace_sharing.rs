//! Native sharing controls. Transport, authentication and persistence belong to SharingController.
use super::*;
use crate::sharing::{SharingController, SharingOptions, SharingPhase, SharingStatus};
use ferroui_controls::{Border, ScrollViewer};
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
}
fn label(panel: &StackPanel, text: &str) -> Ref<TextBlock> {
    let label = TextBlock::new();
    label.set_text(Some(text));
    label.set_foreground(Some(SolidColorBrush::from_uint32(0xffdce5f6).into()));
    label.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
    panel.children().add(label.clone().upcast::<Control>());
    label
}
fn field(panel: &StackPanel, title: &str, text: &str) -> Ref<TextBox> {
    label(panel, title);
    let field = TextBox::new();
    field.set_text(Some(text));
    panel.children().add(field.clone().upcast::<Control>());
    field
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
            body.set_spacing(8.0);
            label(&body, "SHARE DESIGNER DOCUMENT").set_font_size(19.0);
            let status = label(&body, "Disconnected");
            let display_name = field(&body, "Display name", &options.display_name);
            let directory = field(
                &body,
                "Session storage directory",
                &options.directory.display().to_string(),
            );
            let bind = field(
                &body,
                "Host address (empty uses loopback with an available port)",
                "",
            );
            let public_url = field(&body, "Public HTTPS endpoint (optional)", "");
            let join = field(&body, "Invitation to join", "");
            join.set_accepts_return(true);
            join.set_max_height(90.0);
            let commands = row(&body);
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
            let controls = row(&body);
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
                &body,
                "Checkpoint adopts the current source as a new room baseline after synchronizing pending edits.",
            );
            let invitation = field(
                &body,
                "Created invitation — select and copy to a collaborator",
                "",
            );
            invitation.set_is_read_only(true);
            invitation.set_accepts_return(true);
            invitation.set_max_height(90.0);
            label(&body, "ROOM MEMBERS");
            let members = StackPanel::new();
            members.set_spacing(5.0);
            body.children().add(members.clone().upcast::<Control>());
            let review = StackPanel::new();
            review.set_spacing(6.0);
            review.set_is_visible(false);
            body.children().add(review.clone().upcast::<Control>());
            label(&review, "REVIEW INCOMING DOCUMENT").set_font_size(15.0);
            label(
                &review,
                "Review the remote snapshot and your local source. Acceptance checks the captured local revision and creates a durable backup.",
            );
            let remote = field(&review, "Remote source", "");
            remote.set_is_read_only(true);
            remote.set_accepts_return(true);
            remote.set_height(140.0);
            remote.set_font_family(ferroui_base::media::FontFamily::new("monospace"));
            let resolved = field(
                &review,
                "Resolved source — starts with your local or resumed draft",
                "",
            );
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
            action(&body, self, "Close panel", |workspace| {
                workspace
                    .root
                    .get_control::<ContentControl>("SharingHost")
                    .set_is_visible(false);
                Ok(())
            });
            let scroll = ScrollViewer::new();
            scroll.set_max_height(650.0);
            scroll.set_content(Some(Control::boxed(body)));
            let frame = Border::new();
            frame.set_width(760.0);
            frame.set_padding(Thickness::uniform(18.0));
            frame.set_background(Some(SolidColorBrush::from_uint32(0xff252d3d).into()));
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
            for button in &panel.owner_buttons {
                button.set_is_enabled(false);
            }
            for button in &panel.start_buttons {
                button.set_is_enabled(true);
            }
            return;
        };
        let status = controller.status();
        let fingerprint = format!("{:?}", status);
        if *panel.fingerprint.borrow() != fingerprint {
            *panel.fingerprint.borrow_mut() = fingerprint;
            panel.status.set_text(Some(&format!(
                "{:?} · {:?} · {} pending operations\n{}{}",
                status.phase,
                status.role,
                status.pending_operations,
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
                        "{} · {:?}{}",
                        member.display_name,
                        member.role,
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

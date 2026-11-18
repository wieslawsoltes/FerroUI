use super::DesignerWorkspace;
use crate::WorkspaceViewMode;
use ferroui_controls::{Button, Control, MenuFlyout, MenuItem};
use std::rc::Rc;

impl DesignerWorkspace {
    pub(super) fn connect_preview_checks(self: &Rc<Self>) {
        let button = self.root.get_control::<Button>("PreviewChecks");
        let flyout = MenuFlyout::new();
        flyout.flyout_presenter_classes().add("designer-chrome");
        let weak = Rc::downgrade(self);
        let weak_flyout = flyout.downgrade();
        flyout
            .opening(move |_| {
                let (Some(workspace), Some(flyout)) = (weak.upgrade(), weak_flyout.upgrade())
                else {
                    return;
                };
                let items = flyout.items();
                items.clear();
                let session = workspace.session.borrow();
                let findings = if workspace.pending_source.get() {
                    Err("Wait for the current preview".into())
                } else {
                    session.preview_checks()
                };
                let findings = match findings {
                    Ok(findings) if findings.is_empty() => {
                        items.add(Some(Control::boxed(preview_check_item(
                            "No findings in this preview".into(),
                            false,
                        ))));
                        return;
                    }
                    Ok(findings) => findings,
                    Err(message) => {
                        items.add(Some(Control::boxed(preview_check_item(message, false))));
                        return;
                    }
                };
                let generation = session.generation();
                for finding in findings {
                    let control = finding.name.as_deref().unwrap_or(&finding.control);
                    let item = preview_check_item(format!("{control} · {}", finding.message), true);
                    let workspace = weak.clone();
                    let node = finding.node;
                    item.click(move |_, _| {
                        let Some(workspace) = workspace.upgrade() else {
                            return;
                        };
                        if workspace.pending_source.get()
                            || workspace.session.borrow().generation() != generation
                        {
                            workspace
                                .status
                                .set_text(Some("The preview changed. Reopen Checks."));
                            return;
                        }
                        workspace.session.borrow_mut().select([node]);
                        workspace.runtime_selection.set(None);
                        if workspace.view_mode() == WorkspaceViewMode::Design {
                            workspace.set_view_mode(WorkspaceViewMode::Split);
                        }
                        workspace.reveal_source(node);
                        workspace.refresh(true);
                    });
                    items.add(Some(Control::boxed(item)));
                }
            })
            .detach();
        button.set_flyout(Some(flyout.upcast()));
    }
}

fn preview_check_item(label: String, enabled: bool) -> ferroui_base::Ref<MenuItem> {
    let item = MenuItem::new();
    item.set_header(Some(Rc::new(label)));
    item.set_is_enabled(enabled);
    item
}

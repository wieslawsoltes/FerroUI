use super::DesignerWorkspace;
use crate::WorkspaceViewMode;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::TextWrapping;
use ferroui_controls::{
    Button, Control, Flyout, PlacementMode, ScrollViewer, StackPanel, TextBlock,
};
use std::rc::Rc;

impl DesignerWorkspace {
    pub(super) fn connect_preview_checks(self: &Rc<Self>) {
        let button = self.root.get_control::<Button>("PreviewChecks");
        let flyout = Flyout::new();
        flyout.set_placement(PlacementMode::Top);
        flyout.popup().set_should_use_overlay_layer(true);
        flyout.flyout_presenter_classes().add("designer-chrome");
        let weak = Rc::downgrade(self);
        let weak_flyout = flyout.downgrade();
        flyout
            .opening(move |_| {
                let (Some(workspace), Some(flyout)) = (weak.upgrade(), weak_flyout.upgrade())
                else {
                    return;
                };
                let body = StackPanel::new();
                body.set_width(416.0);
                body.set_spacing(5.0);
                let scroll = ScrollViewer::new();
                scroll.set_max_height(320.0);
                scroll.set_content(Some(Control::boxed(body.clone())));
                flyout.set_content(Some(Control::boxed(scroll)));
                let session = workspace.session.borrow();
                let findings = if workspace.pending_source.get() {
                    Err("Wait for the current preview".into())
                } else {
                    session.preview_checks()
                };
                let findings = match findings {
                    Ok(findings) if findings.is_empty() => {
                        body.children().add(
                            check_text("No findings in this preview".into()).upcast::<Control>(),
                        );
                        return;
                    }
                    Ok(findings) => findings,
                    Err(message) => {
                        body.children().add(check_text(message).upcast::<Control>());
                        return;
                    }
                };
                let generation = session.generation();
                for finding in findings {
                    let control = finding.name.as_deref().unwrap_or(&finding.control);
                    let item = Button::new();
                    item.classes().add("nav");
                    item.set_horizontal_alignment(HorizontalAlignment::Stretch);
                    item.set_horizontal_content_alignment(HorizontalAlignment::Left);
                    item.set_content(Some(Control::boxed(check_text(format!(
                        "{control} · {}",
                        finding.message
                    )))));
                    let workspace = weak.clone();
                    let close = weak_flyout.clone();
                    let node = finding.node;
                    let source = finding.source;
                    item.click(move |_, _| {
                        let Some(workspace) = workspace.upgrade() else {
                            return;
                        };
                        if let Some(flyout) = close.upgrade() {
                            flyout.hide();
                        }
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
                        workspace.refresh(true);
                        if let Some(span) = source {
                            workspace.select_source_span(span);
                        } else {
                            workspace.reveal_source(node);
                        }
                    });
                    body.children().add(item.upcast::<Control>());
                }
            })
            .detach();
        button.set_flyout(Some(flyout.upcast()));
    }
}

fn check_text(label: String) -> ferroui_base::Ref<TextBlock> {
    let text = TextBlock::new();
    text.set_text(Some(&label));
    text.set_text_wrapping(TextWrapping::Wrap);
    text
}

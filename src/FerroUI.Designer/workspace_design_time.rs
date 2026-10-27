use super::DesignerWorkspace;
use crate::design_time::{DesignTimeEditor, DesignTimeEditorModel, DesignTimeSnapshot};
use ferroui_controls::{Button, Control, Flyout};
use std::rc::Rc;

impl DesignerWorkspace {
    pub(super) fn connect_design_time_commands(self: &Rc<Self>) {
        let button = self.root.get_control::<Button>("DesignData");
        let flyout = Flyout::new();
        flyout.flyout_presenter_classes().add("designer-chrome");
        button.set_flyout(Some(flyout.clone().upcast()));
        let weak = Rc::downgrade(self);
        let weak_flyout = flyout.downgrade();
        flyout.opening(move |_| {
            let (Some(workspace), Some(flyout)) = (weak.upgrade(), weak_flyout.upgrade()) else {
                return;
            };
            if workspace.pending_source.get() {
                workspace.apply_source();
            }
            let session = workspace.session.borrow();
            let node = session
                .selection()
                .first()
                .copied()
                .unwrap_or(session.document().ast().root());
            let properties = match session.design_time_properties(node) {
                Ok(properties) => properties,
                Err(error) => {
                    drop(session);
                    workspace.report(Err(error));
                    return;
                }
            };
            let sample_count = properties
                .iter()
                .find(|property| property.name == "ItemsSource" && !property.object)
                .and_then(|property| {
                    crate::design_time_samples::sample_count(
                        session.document(),
                        node,
                        &property.value,
                    )
                    .ok()
                    .flatten()
                });
            let title = session.document().ast().node(node).map_or_else(
                || "Selection".into(),
                |node| node.xaml_name().unwrap_or(node.local_name()).to_owned(),
            );
            let model = DesignTimeEditorModel {
                node,
                title,
                properties,
                enabled: session.design_time_enabled(),
                writable: !session.is_read_only(),
                sample_count,
            };
            let snapshot = DesignTimeSnapshot::capture(&session);
            drop(session);
            let weak = Rc::downgrade(&workspace);
            let accept_flyout = flyout.downgrade();
            let cancel_flyout = flyout.downgrade();
            let editor = DesignTimeEditor::new(
                model,
                move |enabled, edits| {
                    let workspace = weak.upgrade().ok_or("The designer was closed.")?;
                    if workspace.pending_source.get() {
                        return Err(
                            "Apply or repair the pending source before editing design data.".into(),
                        );
                    }
                    {
                        let mut session = workspace.session.borrow_mut();
                        if !snapshot.is_current(&session) {
                            return Err(
                                "The document or selection changed. Reopen Design Data.".into()
                            );
                        }
                        if !edits.is_empty() {
                            snapshot.apply(&mut session, &edits)?;
                        }
                        session.set_design_time_enabled(enabled)?;
                    }
                    workspace.refresh(true);
                    if let Some(flyout) = accept_flyout.upgrade() {
                        flyout.hide();
                    }
                    Ok(())
                },
                move || {
                    if let Some(flyout) = cancel_flyout.upgrade() {
                        flyout.hide();
                    }
                },
            );
            flyout.set_content(Some(Control::boxed(editor.control())));
        });
    }
}

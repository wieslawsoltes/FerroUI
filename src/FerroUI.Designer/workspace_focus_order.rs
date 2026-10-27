//! Workspace host for the reusable staged focus editor.
use super::*;
use crate::{FocusOrderCatalog, FocusOrderEditor, FocusOrderSnapshot};
use ferroui_controls::Flyout;

impl DesignerWorkspace {
    pub(super) fn connect_focus_order_commands(self: &Rc<Self>) {
        let button = self.root.get_control::<Button>("FocusOrder");
        let flyout = Flyout::new();
        flyout.flyout_presenter_classes().add("designer-chrome");
        let editor = FocusOrderEditor::new();
        let accepted = Rc::new(RefCell::new(None::<FocusOrderSnapshot>));
        let weak = Rc::downgrade(self);
        let snapshot = accepted.clone();
        editor.on_apply(move |commands| {
            let workspace = weak.upgrade().ok_or("The workspace was closed")?;
            if workspace.pending_source.get() {
                return Err(
                    "Source editing is pending; reopen Focus order after the preview updates"
                        .into(),
                );
            }
            let snapshot = snapshot
                .borrow()
                .clone()
                .ok_or("There is no accepted focus model")?;
            workspace
                .session
                .borrow_mut()
                .commit_focus_order(&snapshot, &commands)?;
            workspace.refresh(true);
            Ok(())
        });
        let weak = flyout.downgrade();
        editor.on_close(move || {
            if let Some(flyout) = weak.upgrade() {
                flyout.hide();
            }
        });
        let weak = Rc::downgrade(self);
        let weak_flyout = flyout.downgrade();
        let editor_opening = editor.clone();
        flyout.opening(move |_| {
            let Some(workspace) = weak.upgrade() else {
                return;
            };
            if workspace.pending_source.get() {
                workspace.apply_source();
            }
            let session = workspace.session.borrow();
            match session.focus_order_snapshot() {
                Ok(snapshot) => {
                    editor_opening.configure(snapshot.catalog.clone(), !session.is_read_only());
                    accepted.replace(Some(snapshot));
                    workspace.surface.set_focus_order_visible(true);
                }
                Err(error) => {
                    accepted.replace(None);
                    editor_opening.configure(
                        FocusOrderCatalog {
                            scopes: Vec::new(),
                            labels: Vec::new(),
                        },
                        false,
                    );
                    editor_opening.show_error(&error);
                }
            }
            if let Some(flyout) = weak_flyout.upgrade() {
                flyout.set_content(Some(Control::boxed(editor_opening.control())));
            }
        });
        let weak = Rc::downgrade(self);
        flyout.closed(move || {
            editor.retire();
            if let Some(workspace) = weak.upgrade() {
                workspace.surface.set_focus_order_visible(false);
            }
        });
        button.set_flyout(Some(flyout.upcast()));
    }

    pub fn show_focus_order_editor(self: &Rc<Self>) -> Result<(), String> {
        let button = self.root.get_control::<Button>("FocusOrder");
        let flyout = button
            .flyout()
            .ok_or("The keyboard-navigation editor is not connected")?;
        flyout.show_at(&button);
        Ok(())
    }
}

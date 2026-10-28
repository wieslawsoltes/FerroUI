//! Inspector commands and workspace-local appearance clipboard.
use super::*;
use crate::{PropertyClipboard, PropertyTransferEditor};
use ferroui_controls::{Flyout, MenuItem};

#[cfg(all(test, feature = "render-smoke"))]
#[path = "render_selection_workflows_tests.rs"]
mod render_tests;

#[derive(Default)]
pub(super) struct PropertyTransferUi {
    clipboard: RefCell<Option<PropertyClipboard>>,
    editor: RefCell<Option<Rc<PropertyTransferEditor>>>,
    flyout: RefCell<Option<Ref<Flyout>>>,
}

impl Drop for PropertyTransferUi {
    fn drop(&mut self) {
        if let Some(editor) = self.editor.get_mut().take() {
            editor.retire();
        }
    }
}

impl DesignerWorkspace {
    /// Capture authored appearance without changing the operating-system text clipboard.
    /// The retained clipboard is shared by this workspace's document tabs.
    pub fn copy_appearance_properties(self: &Rc<Self>) -> Result<(), String> {
        self.check_property_transfer_source()?;
        let clipboard = self.session.borrow().copy_properties()?;
        if let Some(editor) = self.property_transfer.editor.borrow().as_ref() {
            editor.retire();
        }
        if let Some(flyout) = self.property_transfer.flyout.borrow().as_ref() {
            flyout.hide();
        }
        self.status.set_text(Some(&format!(
            "Copied {} authored appearance properties from {} · {} context-dependent values skipped",
            clipboard.entries().len(), clipboard.source_label(), clipboard.skipped().len(),
        )));
        *self.property_transfer.clipboard.borrow_mut() = Some(clipboard);
        self.refresh_property_transfer_commands();
        Ok(())
    }

    /// Current immutable clipboard, available to embedders for their own paste UI.
    pub fn appearance_clipboard(&self) -> Option<PropertyClipboard> {
        self.property_transfer.clipboard.borrow().clone()
    }

    /// Most recently opened staged picker. Retained controls reject writes after dismissal.
    pub fn property_transfer_editor(&self) -> Option<Rc<PropertyTransferEditor>> {
        self.property_transfer.editor.borrow().clone()
    }

    pub fn show_property_paste_editor(self: &Rc<Self>) -> Result<(), String> {
        self.check_property_transfer_source()?;
        let clipboard = self
            .appearance_clipboard()
            .ok_or("Copy appearance properties from an authored control first")?;
        let accepted = self
            .session
            .borrow()
            .prepare_property_paste(&clipboard, &clipboard.property_names())?;
        let writable = !self.session.borrow().is_read_only();
        if let Some(editor) = self.property_transfer.editor.borrow().as_ref() {
            editor.retire();
        }
        if let Some(flyout) = self.property_transfer.flyout.borrow().as_ref() {
            flyout.hide();
        }
        let editor = PropertyTransferEditor::new(&clipboard, &accepted, writable);
        let flyout = Flyout::new();
        flyout.flyout_presenter_classes().add("designer-chrome");
        flyout
            .flyout_presenter_classes()
            .add("designer-property-transfer-flyout");
        flyout.set_content(Some(Control::boxed(editor.control())));
        let weak = Rc::downgrade(self);
        editor.on_apply(move |properties| {
            let workspace = weak.upgrade().ok_or("The workspace was closed")?;
            workspace.check_property_transfer_source()?;
            {
                let mut session = workspace.session.borrow_mut();
                if !accepted.is_current(&session) {
                    return Err(
                        "The document, selection or preview scope changed; reopen Paste properties"
                            .into(),
                    );
                }
                let preview = session.prepare_property_paste(&clipboard, &properties)?;
                session.commit_property_paste(&preview)?;
            }
            workspace.refresh(true);
            Ok(())
        });
        let weak_flyout = flyout.downgrade();
        editor.on_close(move || {
            if let Some(flyout) = weak_flyout.upgrade() {
                flyout.hide();
            }
        });
        let weak_editor = Rc::downgrade(&editor);
        flyout.closed(move || {
            if let Some(editor) = weak_editor.upgrade() {
                editor.retire();
            }
        });
        *self.property_transfer.editor.borrow_mut() = Some(editor);
        *self.property_transfer.flyout.borrow_mut() = Some(flyout.clone());
        let anchor = self.root.get_control::<Button>("PropertyTransferOptions");
        flyout.show_at(&anchor);
        Ok(())
    }

    fn check_property_transfer_source(&self) -> Result<(), String> {
        if self.pending_source.get() {
            return Err(
                "Apply or cancel the source draft before copying or pasting appearance properties"
                    .into(),
            );
        }
        if self.runtime_selection.get().is_some() {
            return Err("Select authored controls to copy or paste appearance properties".into());
        }
        Ok(())
    }

    pub(super) fn connect_property_transfer_commands(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.root
            .get_control::<MenuItem>("PropertyCopyAppearance")
            .click(move |_, _| {
                if let Some(workspace) = weak.upgrade() {
                    workspace.report(workspace.copy_appearance_properties());
                }
            });
        let weak = Rc::downgrade(self);
        self.root
            .get_control::<MenuItem>("PropertyPasteAppearance")
            .click(move |_, _| {
                if let Some(workspace) = weak.upgrade() {
                    workspace.report(workspace.show_property_paste_editor());
                }
            });
        let weak = Rc::downgrade(self);
        self.root
            .add_handler(InputElement::key_down_event(), move |_, event| {
                let Some(workspace) = weak.upgrade() else {
                    return;
                };
                if workspace.text_input_focused()
                    || event.key_modifiers.contains(KeyModifiers::SHIFT)
                    || !event.key_modifiers.contains(KeyModifiers::ALT)
                    || !event
                        .key_modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::META)
                {
                    return;
                }
                let result = match event.key {
                    Key::C => workspace.copy_appearance_properties(),
                    Key::V => workspace.show_property_paste_editor(),
                    _ => return,
                };
                workspace.report(result);
                event.set_handled(true);
            });
    }

    pub(super) fn refresh_property_transfer_commands(&self) {
        let session = self.session.borrow();
        let available = !self.pending_source.get()
            && self.runtime_selection.get().is_none()
            && matches!(session.preview_status(), PreviewStatus::Ready { revision } if *revision == session.document().revision());
        self.root
            .get_control::<MenuItem>("PropertyCopyAppearance")
            .set_is_enabled(
                available
                    && session.selection().len() == 1
                    && session
                        .selection()
                        .iter()
                        .all(|node| session.preview.visual(*node).is_some()),
            );
        self.root
            .get_control::<MenuItem>("PropertyPasteAppearance")
            .set_is_enabled(
                available
                    && self.property_transfer.clipboard.borrow().is_some()
                    && !session.selection().is_empty()
                    && session.selection().iter().all(|node| {
                        session.ensure_layer_manipulable(*node).is_ok()
                            && session.preview.visual(*node).is_some()
                    }),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_controls::testing::{TestServices, UnitTestApplication};

    const SOURCE: &str = r#"<StackPanel xmlns='https://github.com/ferroui'><Border Name='Source' Background='Red' CornerRadius='8'/><Border Name='Target' Background='Blue'/></StackPanel>"#;

    fn select(workspace: &Rc<DesignerWorkspace>, name: &str) -> NodeId {
        let session = workspace.session();
        let node = session.borrow().document().ast().find_name(name).unwrap();
        session.borrow_mut().select([node]);
        workspace.refresh(true);
        node
    }

    #[test]
    fn native_property_picker_stages_selective_paste_and_retires_after_apply_cancel_or_source_change(
    ) {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let workspace = DesignerWorkspace::new(SOURCE).unwrap();
        select(&workspace, "Source");
        workspace.copy_appearance_properties().unwrap();
        let target = select(&workspace, "Target");
        workspace.show_property_paste_editor().unwrap();
        let editor = workspace.property_transfer_editor().unwrap();
        editor.select_property("CornerRadius", false).unwrap();
        assert_eq!(workspace.session.borrow().document().source(), SOURCE);
        editor.apply().unwrap();
        assert!(editor.apply().is_err());
        let session = workspace.session.borrow();
        let node = session.document().ast().node(target).unwrap();
        assert_eq!(node.attribute("Background"), Some("Red"));
        assert_eq!(node.attribute("CornerRadius"), None);
        drop(session);
        workspace.session.borrow_mut().undo().unwrap();
        workspace.refresh(true);
        workspace.show_property_paste_editor().unwrap();
        let cancelled = workspace.property_transfer_editor().unwrap();
        cancelled.cancel();
        assert!(cancelled.apply().is_err());
        assert_eq!(workspace.session.borrow().document().source(), SOURCE);
        workspace.show_property_paste_editor().unwrap();
        let stale = workspace.property_transfer_editor().unwrap();
        select(&workspace, "Source");
        assert!(stale.apply().is_err());
        assert_eq!(workspace.session.borrow().document().source(), SOURCE);
    }

    #[test]
    fn pending_source_and_runtime_only_selection_never_overwrite_clipboard_or_authored_targets() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let workspace = DesignerWorkspace::new(SOURCE).unwrap();
        select(&workspace, "Source");
        workspace.copy_appearance_properties().unwrap();
        workspace.pending_source.set(true);
        assert!(workspace.copy_appearance_properties().is_err());
        assert!(workspace.show_property_paste_editor().is_err());
        assert_eq!(
            workspace.appearance_clipboard().unwrap().source_label(),
            "Border · Source"
        );
        workspace.pending_source.set(false);
        workspace.runtime_selection.set(Some(123));
        assert!(workspace.copy_appearance_properties().is_err());
        assert!(workspace.show_property_paste_editor().is_err());
        assert_eq!(workspace.session.borrow().document().source(), SOURCE);
    }
}

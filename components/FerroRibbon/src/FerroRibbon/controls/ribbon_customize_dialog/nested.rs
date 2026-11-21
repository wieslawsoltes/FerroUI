//! Source RunNestedDialog/Rename/ImportExport workflows, with native scoped
//! overlays and callback ownership rather than Task/delegate emulation.
use super::*;
use ferroribbon_core::state::RibbonStateSerializer;
use ferroui_base::{
    WeakRef,
    media::{FontFamily, TextWrapping},
    threading::{DispatcherPriority, DispatcherTimer},
};
use std::sync::Mutex;
static INVALID_IMPORT_MESSAGE: Mutex<Option<String>> = Mutex::new(None);
enum Next {
    Resume,
    InvalidImport,
}
#[derive(Clone, Copy)]
enum NestedKind {
    Rename,
    ImportExport,
    InvalidImport,
}
type NestedBody = Box<dyn FnOnce(&RibbonCustomizeDialog, bool) -> DialogResult<Next>>;
type WaitingBody = (NestedKind, Ref<Control>, NestedBody);
pub(super) struct NestedWait {
    parent: WeakRef<RibbonCustomizeDialog>,
    root: WeakRef<TopLevel>,
    body: RefCell<Option<WaitingBody>>,
    arming: Cell<bool>,
    closed: Cell<bool>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    timer: RefCell<Option<ferroui_base::reactive::Subscription>>,
}
impl NestedWait {
    fn start(&self) -> DialogResult<()> {
        let Some((kind, content, body)) = self.body.take() else {
            return Ok(());
        };
        if let Some(subscription) = self.subscription.take() {
            subscription.dispose();
        }
        if let Some(timer) = self.timer.take() {
            timer.dispose();
        }
        let Some(parent) = self.parent.upgrade() else {
            return Ok(());
        };
        parent
            .pending_nested
            .borrow_mut()
            .retain(|wait| !std::ptr::eq(&**wait, self));
        let Some(root) = self.root.upgrade() else {
            return parent.expected_nested_error(RibbonCustomizeDialogError::RootNotVisible);
        };
        parent.open_nested(root, kind, content, body)
    }
}
impl Drop for NestedWait {
    fn drop(&mut self) {
        if let Some(subscription) = self.subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(timer) = self.timer.get_mut().take() {
            timer.dispose();
        }
    }
}

impl RibbonCustomizeDialog {
    pub fn invalid_import_message() -> Option<String> {
        INVALID_IMPORT_MESSAGE
            .lock()
            .expect("customization import message lock")
            .clone()
    }
    pub fn set_invalid_import_message(value: Option<String>) -> Option<String> {
        std::mem::replace(
            &mut *INVALID_IMPORT_MESSAGE
                .lock()
                .expect("customization import message lock"),
            value,
        )
    }
    /// Source intentionally tolerates busy/closing native dialogs. Rust callers
    /// can inspect that typed outcome instead of an unobservable swallowed fault.
    pub fn last_nested_error(&self) -> Option<RibbonCustomizeDialogError> {
        self.nested_error.borrow().clone()
    }
    pub(super) fn rename(&self) -> DialogResult<()> {
        self.sync_editor_selection();
        let row = self.editor.borrow().selected();
        let Some(row) = row.filter(|r| r.kind != RibbonCustomizationRowKind::Command) else {
            return Ok(());
        };
        let strings = RibbonStrings::current();
        let label = utf8(row.label.clone(), "StructureRow.Label")?.ok_or_else(|| {
            RibbonCustomizeDialogError::State("Selected structure row has no label".into())
        })?;
        let suffix = utf8(strings.custom_group_suffix(), "CustomGroupSuffix")?.unwrap_or_default();
        let text_box = TextBox::new();
        text_box.set_text(Some(&label.replace(&format!(" {suffix}"), "")));
        let box_copy = text_box.clone();
        self.run_nested(
            NestedKind::Rename,
            text_box.upcast(),
            Box::new(move |dialog, primary| {
                if primary
                    && let Some(label) = box_copy.text()
                    && !ferroribbon_core::text::trim(&label.encode_utf16().collect::<Vec<_>>())
                        .is_empty()
                {
                    dialog.sync_editor_selection();
                    {
                        let mut editor = dialog.editor.borrow_mut();
                        editor.set_tabs(snapshot_tabs(&dialog.ribbon));
                        editor.set_custom_suffix(RibbonStrings::current().custom_group_suffix());
                        editor.rename_row(&row, label.into())?;
                    }
                    dialog.render_edited_structure()?;
                }
                Ok(Next::Resume)
            }),
        )
    }
    pub(super) fn import_export(&self) -> DialogResult<()> {
        let serialized = RibbonStateSerializer::serialize_default(Some(&self.working_state()))
            .map_err(|e| RibbonCustomizeDialogError::State(e.to_string()))?;
        let box_text = utf8(Some(serialized), "ImportExport.Text")?;
        let text_box = TextBox::new();
        text_box.set_text(box_text.as_deref());
        text_box.set_accepts_return(true);
        text_box.set_text_wrapping(TextWrapping::Wrap);
        text_box.set_height(360.);
        text_box.set_width(560.);
        text_box.set_font_family(FontFamily::new("Consolas,Menlo,monospace"));
        let box_copy = text_box.clone();
        self.run_nested(
            NestedKind::ImportExport,
            text_box.upcast(),
            Box::new(move |dialog, primary| {
                if !primary {
                    return Ok(Next::Resume);
                }
                let text = box_copy.text().map(RibbonString::from);
                let imported = RibbonStateSerializer::deserialize(text.as_ref())
                    .map_err(|e| RibbonCustomizeDialogError::State(e.to_string()))?;
                let Some(imported) = imported else {
                    return Ok(Next::InvalidImport);
                };
                dialog.editor.borrow().import_fields(&imported);
                dialog.qat_below.set_is_checked(Some(
                    dialog.working_state().quick_access_position()
                        == RibbonQuickAccessPosition::BelowRibbon,
                ));
                dialog.fill_structure()?;
                dialog.fill_qat()?;
                Ok(Next::Resume)
            }),
        )
    }
    fn run_nested(
        &self,
        kind: NestedKind,
        content: Ref<Control>,
        body: NestedBody,
    ) -> DialogResult<()> {
        let root = self
            .scope
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .and_then(|s| s.owner())
            .ok_or(RibbonCustomizeDialogError::RootNotVisible)?;
        self.nested_error.replace(None);
        let wait = Rc::new(NestedWait {
            parent: self.to_ref().downgrade(),
            root: root.downgrade(),
            body: RefCell::new(Some((kind, content, body))),
            arming: Cell::new(true),
            closed: Cell::new(false),
            subscription: RefCell::default(),
            timer: RefCell::default(),
        });
        let weak = Rc::downgrade(&wait);
        wait.subscription.replace(Some(self.dialog_closed(move |_| {
            if let Some(wait) = weak.upgrade() {
                wait.closed.set(true);
                if !wait.arming.get() {
                    wait.start().expect("nested dialog after deferred close");
                }
            }
        })));
        self.pending_nested.borrow_mut().push(wait.clone());
        self.close(false)?;
        wait.arming.set(false);
        if wait.closed.get() {
            wait.start()
        } else {
            let weak = Rc::downgrade(&wait);
            wait.timer.replace(Some(DispatcherTimer::run_once(
                move || {
                    if let Some(wait) = weak.upgrade() {
                        wait.start().expect("nested dialog after source timeout");
                    }
                },
                std::time::Duration::from_secs(1),
                DispatcherPriority::NORMAL,
            )));
            Ok(())
        }
    }
    fn expected_nested_error(&self, error: RibbonCustomizeDialogError) -> DialogResult<()> {
        match error {
            RibbonCustomizeDialogError::AlreadyOpen
            | RibbonCustomizeDialogError::RootNotVisible => {
                self.nested_error.replace(Some(error));
                Ok(())
            }
            error => Err(error),
        }
    }
    fn resume_after_nested(&self, root: Option<Ref<TopLevel>>) -> DialogResult<()> {
        if let Some(root) = root
            && let Err(error) = self.show(root)
        {
            return self.expected_nested_error(error);
        }
        Ok(())
    }
    fn open_nested(
        &self,
        root: Ref<TopLevel>,
        kind: NestedKind,
        content: Ref<Control>,
        body: NestedBody,
    ) -> DialogResult<()> {
        // Source creates the nested dialog after Hide/Closed. A Closed handler
        // may replace the current localization before these strings are read.
        let strings = RibbonStrings::current();
        let title = match kind {
            NestedKind::Rename => Some(
                utf8(strings.rename(), "Rename")?
                    .ok_or(RibbonCustomizeDialogError::MissingLocalizedText("Rename"))?
                    .trim_end_matches('.')
                    .to_string(),
            ),
            NestedKind::ImportExport | NestedKind::InvalidImport => {
                utf8(strings.import_export(), "ImportExport")?
            }
        };
        let primary = !matches!(kind, NestedKind::InvalidImport);
        let host = RibbonDialogHost::new();
        host.set_title(boxed(title));
        host.set_content(Some(Control::boxed(content)));
        host.set_primary_button_text(if primary {
            utf8(strings.ok(), "Ok")?
        } else {
            None
        });
        host.set_close_button_text(utf8(
            if primary {
                strings.cancel()
            } else {
                strings.ok()
            },
            "Nested.Close",
        )?);
        host.set_default_button(if matches!(kind, NestedKind::Rename) {
            crate::compat::ContentDialogButton::Primary
        } else {
            crate::compat::ContentDialogButton::None
        });
        RibbonTheme::set_theme(Some(&host), self.actual_theme_variant())
            .expect("nested dialog theme");
        let weak_root = root.downgrade();
        let resume_root = weak_root.clone();
        let scope_slot: Rc<RefCell<Option<Weak<RibbonDialogSession>>>> =
            Rc::new(RefCell::new(None));
        let callback_slot = scope_slot.clone();
        let parent = self.to_ref();
        let body = RefCell::new(Some(body));
        let completion_host = host.downgrade();
        let result = RibbonDialogSession::new(root, host, move |primary| {
            let scope = callback_slot.borrow().as_ref().and_then(Weak::upgrade);
            let Some(scope) = scope else {
                return;
            };
            if !scope.dismiss() {
                return;
            }
            let body = body.take().expect("nested dialog completes once");
            let parent = parent.clone();
            let weak_root = weak_root.clone();
            let host = completion_host.upgrade().expect("live nested dialog host");
            host.after_dialog_closed(move || {
                match body(
                    &parent,
                    primary == crate::compat::ContentDialogButton::Primary,
                )
                .expect("customization nested dialog action")
                {
                    Next::Resume => parent
                        .resume_after_nested(weak_root.upgrade())
                        .expect("customization dialog re-show"),
                    Next::InvalidImport => {
                        if let Some(root) = weak_root.upgrade() {
                            let message = Self::invalid_import_message()
                                .map(|message| Ok(Some(message)))
                                .unwrap_or_else(|| {
                                    utf8(RibbonStrings::current().invalid_import(), "InvalidImport")
                                })
                                .expect("invalid import message");
                            let label = TextBlock::new();
                            label.set_text(message.as_deref());
                            label.set_text_wrapping(TextWrapping::Wrap);
                            parent
                                .open_nested(
                                    root,
                                    NestedKind::InvalidImport,
                                    label.upcast(),
                                    Box::new(|_, _| Ok(Next::Resume)),
                                )
                                .expect("invalid import dialog");
                        }
                    }
                }
            });
        });
        match result {
            Ok(scope) => {
                scope_slot.replace(Some(Rc::downgrade(&scope)));
                scope.show();
                Ok(())
            }
            Err(error) => {
                let error: RibbonCustomizeDialogError = error.into();
                self.expected_nested_error(error)?;
                self.resume_after_nested(resume_root.upgrade())
            }
        }
    }
}

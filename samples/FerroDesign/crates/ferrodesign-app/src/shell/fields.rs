//! Native inspector drafts participate in the same lifecycle save boundary as
//! on-canvas text. Controls own their bindings; the shell retains only weak
//! references, so rebuilding a modal cannot keep a stale field or selection alive.
use super::*;
use ferroui_base::interactivity::RoutingStrategies;
use std::rc::Weak;

pub(super) struct InspectorDraft {
    field: WeakRef<TextBox>,
    committed: Rc<RefCell<String>>,
    confirmed: RefCell<String>,
    commit: Rc<dyn Fn()>,
}

impl InspectorDraft {
    fn live_field(&self) -> Option<Ref<TextBox>> {
        self.field
            .upgrade()
            .filter(|field| field.is_attached_to_visual_tree() && field.is_effectively_visible())
    }

    fn is_dirty(&self) -> bool {
        let Some(field) = self.live_field() else {
            return false;
        };
        let value = if field.is_composing_text() {
            self.confirmed.borrow().clone()
        } else {
            field.text().unwrap_or_default()
        };
        value != *self.committed.borrow()
    }

    fn restore_confirmed_composition(&self) {
        if let Some(field) = self.live_field().filter(|field| field.is_composing_text()) {
            // A native IME can temporarily remove the selected text while its
            // preedit remains outside Text. Restore the last confirmed value
            // before focus detaches the IME and raises ordinary LostFocus.
            let confirmed = self.confirmed.borrow().clone();
            field.set_text(Some(&confirmed));
        }
    }
}

impl EditorShell {
    pub(super) fn reject_inspector_field(&self, name: &str, message: &str) -> bool {
        self.field(name)
            .set_border_brush(Some(icons::brush("#C8372B")));
        self.toast(message);
        false
    }

    pub(super) fn bind_inspector_draft(
        self: &Rc<Self>,
        field: &Ref<TextBox>,
        committed: Rc<RefCell<String>>,
        commit: Rc<dyn Fn()>,
        error: Option<&Ref<TextBlock>>,
    ) {
        let confirmed = committed.borrow().clone();
        let draft = Rc::new(InspectorDraft {
            field: field.downgrade(),
            confirmed: RefCell::new(confirmed),
            committed,
            commit,
        });
        self.inspector_drafts
            .borrow_mut()
            .push(Rc::downgrade(&draft));
        let weak_shell = Rc::downgrade(self);
        let editing = draft.clone();
        field.text_changing(move |_, _| {
            let (Some(shell), Some(field)) = (weak_shell.upgrade(), editing.field.upgrade()) else {
                return;
            };
            if field.is_composing_text() {
                return;
            }
            let value = field.text().unwrap_or_default();
            *editing.confirmed.borrow_mut() = value.clone();
            if shell.updating.get() {
                *editing.committed.borrow_mut() = value;
            }
            shell.sync_document_draft_dirty();
        });
        let weak_shell = Rc::downgrade(self);
        let blur = draft.clone();
        field.add_handler(InputElement::lost_focus_event(), move |_, _| {
            // Register after the field's native commit handler. An invalid
            // value stays dirty and keeps the host's existing unload guard.
            if let Some(shell) = weak_shell.upgrade() {
                if let Some(field) = blur
                    .field
                    .upgrade()
                    .filter(|field| !field.is_composing_text())
                {
                    *blur.confirmed.borrow_mut() = field.text().unwrap_or_default();
                }
                shell.sync_document_draft_dirty();
            }
        });
        let weak_shell = Rc::downgrade(self);
        let canceled = draft.clone();
        let error = error.map(|error| error.downgrade());
        field.add_handler_with(
            InputElement::key_down_event(),
            move |_, event| {
                let (Some(shell), Some(field)) = (weak_shell.upgrade(), canceled.field.upgrade())
                else {
                    return;
                };
                if event.key != Key::Escape || field.is_composing_text() {
                    return;
                }
                let value = canceled.committed.borrow().clone();
                field.set_text(Some(&value));
                field.set_border_brush(Some(icons::brush("#E5E5E5")));
                if let Some(error) = error.as_ref().and_then(WeakRef::upgrade) {
                    error.set_is_visible(false);
                }
                shell.sync_document_draft_dirty();
                event.set_handled(true);
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let weak_shell = Rc::downgrade(self);
        // Effects handles numeric keys during tunneling. Observe handled keys
        // after those commands, without taking their keyboard ownership.
        field.add_handler_with(
            InputElement::key_down_event(),
            move |_, _| {
                let _keep_binding_alive = &draft;
                if let Some(shell) = weak_shell.upgrade() {
                    shell.sync_document_draft_dirty();
                }
            },
            RoutingStrategies::BUBBLE,
            true,
        );
    }

    pub(super) fn wire_inspector_field(self: &Rc<Self>, name: &'static str) {
        let field = self.field(name);
        let committed = Rc::new(RefCell::new(field.text().unwrap_or_default()));
        let weak_shell = Rc::downgrade(self);
        let weak_field = field.downgrade();
        let value = committed.clone();
        let submit: Rc<dyn Fn(bool) -> bool> = Rc::new(move |explicit| {
            let (Some(shell), Some(field)) = (weak_shell.upgrade(), weak_field.upgrade()) else {
                return false;
            };
            if shell.updating.get() || field.is_composing_text() {
                return false;
            }
            // Enter is an explicit literal override for radius, even if its
            // displayed number already equals the bound variable. Passive
            // blur/flush must not detach an unchanged binding or add history.
            if !(explicit && name == "RadiusField")
                && field.text().unwrap_or_default() == *value.borrow()
            {
                return true;
            }
            let accepted = shell.commit_field(name);
            if accepted {
                // Update before moving focus: the resulting LostFocus event
                // sees a clean field and cannot submit this command twice.
                *value.borrow_mut() = field.text().unwrap_or_default();
            }
            shell.sync_document_draft_dirty();
            accepted
        });
        let passive = submit.clone();
        let commit: Rc<dyn Fn()> = Rc::new(move || {
            passive(false);
        });
        let blur = commit.clone();
        field.add_handler(InputElement::lost_focus_event(), move |_, _| blur());
        let enter = submit;
        let weak_shell = Rc::downgrade(self);
        let weak_field = field.downgrade();
        field.add_handler(InputElement::key_down_event(), move |_, event| {
            if event.key != Key::Enter || name == "TextField" {
                return;
            }
            if weak_field
                .upgrade()
                .is_some_and(|field| field.is_composing_text())
            {
                return;
            }
            if enter(true) {
                if let Some(shell) = weak_shell.upgrade() {
                    shell.canvas.focus();
                }
            }
            // A rejected Enter retains the TextBox and its draft. Escape then
            // reaches bind_inspector_draft's native restoration handler.
            event.set_handled(true);
        });
        self.bind_inspector_draft(&field, committed, commit, None);
    }

    fn live_inspector_drafts(&self) -> Vec<Rc<InspectorDraft>> {
        let mut bindings = self.inspector_drafts.borrow_mut();
        let mut live = Vec::with_capacity(bindings.len());
        bindings.retain(|binding: &Weak<InspectorDraft>| {
            if let Some(binding) = binding.upgrade() {
                if binding.live_field().is_some() {
                    live.push(binding);
                }
                true
            } else {
                false
            }
        });
        live
    }

    pub(super) fn sync_document_draft_dirty(&self) {
        let dirty = self.inline_draft_dirty.get()
            || self.canvas.network_bucket_state().1
            || self.canvas.stroke_width_active()
            || self.canvas.corner_radius_active()
            || self.canvas.linear_gradient_active()
            || self
                .live_inspector_drafts()
                .iter()
                .any(|draft| draft.is_dirty());
        platform::set_document_draft_dirty(dirty);
    }

    pub(super) fn set_inline_draft_dirty(&self, dirty: bool) {
        self.inline_draft_dirty.set(dirty);
        self.sync_document_draft_dirty();
    }

    pub(super) fn flush_inspector_drafts(&self) -> bool {
        let drafts = self.live_inspector_drafts();
        for draft in &drafts {
            draft.restore_confirmed_composition();
        }
        // Let FerroUI run the actual native focus-loss handlers before taking
        // the persistence snapshot. Do not synthesize keyboard events.
        self.canvas.focus();
        for draft in &drafts {
            if draft.is_dirty() {
                (draft.commit)();
            }
        }
        self.sync_document_draft_dirty();
        !self
            .live_inspector_drafts()
            .iter()
            .any(|draft| draft.is_dirty())
    }
}

#[cfg(all(test, not(target_os = "emscripten")))]
#[path = "fields_tests.rs"]
mod tests;

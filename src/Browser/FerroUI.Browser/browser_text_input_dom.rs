use super::BrowserTextInputMethod;
use crate::browser_text_input_edit::{edit_from_dom, utf16_length};
use crate::browser_text_input_projection::DomTextProjection;
use ferroui_base::input::text_input::{
    TextInputMethodClient, TextInputMethodConnection, TextInputMethodEdit, TextInputMethodError,
    TextInputMethodState, TextSelection,
};
use std::cell::Cell;
use std::rc::{Rc, Weak};

#[derive(Clone)]
struct Snapshot {
    revision: u32,
    generation: u64,
    events: u64,
    intrinsic: Option<u64>,
    client: Weak<dyn TextInputMethodClient>,
    connection: Option<Weak<TextInputMethodConnection>>,
    exact: bool,
    state: TextInputMethodState,
    projection: DomTextProjection,
}

impl Snapshot {
    fn same_state(&self, other: &Self) -> bool {
        self.generation == other.generation
            && self.events == other.events
            && self.intrinsic == other.intrinsic
            && self.exact == other.exact
            && Weak::ptr_eq(&self.client, &other.client)
            && match (&self.connection, &other.connection) {
                (Some(left), Some(right)) => Weak::ptr_eq(left, right),
                (None, None) => true,
                _ => false,
            }
            && self.state == other.state
    }
}

struct PendingEdit {
    token: u32,
    snapshot: Snapshot,
    selection: TextSelection,
    input_type: String,
}

#[derive(Clone)]
struct Composition {
    token: u32,
    generation: u64,
    exact: bool,
    range: TextSelection,
    original: TextInputMethodState,
    original_dom: String,
}

#[derive(Default)]
pub(super) struct DomState {
    serial: u32,
    snapshot: Option<Snapshot>,
    pending: Option<PendingEdit>,
    composition: Option<Composition>,
}

impl DomState {
    fn next_revision(&mut self) -> u32 {
        self.serial = self.serial.wrapping_add(1).max(1);
        self.serial
    }
}

struct FlagGuard<'a>(&'a Cell<bool>);
impl Drop for FlagGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl BrowserTextInputMethod {
    pub(super) fn invalidate_dom(&self) -> u32 {
        self.client_revision
            .set(self.client_revision.get().wrapping_add(1));
        self.dom_dirty.set(true);
        let mut dom = self.dom.borrow_mut();
        dom.snapshot = None;
        dom.pending = None;
        dom.composition = None;
        dom.next_revision()
    }

    fn same_binding(
        &self,
        generation: u64,
        client: &Rc<dyn TextInputMethodClient>,
        connection: &Option<Rc<TextInputMethodConnection>>,
    ) -> bool {
        self.generation.get() == generation
            && self
                .client()
                .is_some_and(|current| Rc::ptr_eq(&current, client))
            && match (&*self.connection.borrow(), connection) {
                (Some(current), Some(connection)) => Rc::ptr_eq(current, connection),
                (None, None) => true,
                _ => false,
            }
    }

    fn capture_snapshot(&self) -> Result<Snapshot, TextInputMethodError> {
        let generation = self.generation.get();
        let events = self.client_revision.get();
        let client = self.client().ok_or(TextInputMethodError::Unavailable)?;
        let connection = self.connection.borrow().clone();
        let intrinsic = client.input_method_state_revision();
        if !self.same_binding(generation, &client, &connection) {
            return Err(TextInputMethodError::StaleConnection);
        }
        let exact = connection
            .as_ref()
            .is_some_and(|connection| connection.supports_exact_edits());
        if !self.same_binding(generation, &client, &connection) {
            return Err(TextInputMethodError::StaleConnection);
        }
        let state = if exact {
            connection.as_ref().unwrap().edit_state()?
        } else {
            let text = client.surrounding_text();
            if !self.same_binding(generation, &client, &connection) {
                return Err(TextInputMethodError::StaleConnection);
            }
            TextInputMethodState {
                text,
                selection: client.selection(),
            }
        };
        if !self.same_binding(generation, &client, &connection) {
            return Err(TextInputMethodError::StaleConnection);
        }
        let current_intrinsic = client.input_method_state_revision();
        if !self.same_binding(generation, &client, &connection)
            || self.client_revision.get() != events
            || intrinsic != current_intrinsic
        {
            return Err(TextInputMethodError::StateChanged);
        }
        let projection = DomTextProjection::new(&state)?;
        Ok(Snapshot {
            revision: 0,
            generation,
            events,
            intrinsic,
            client: Rc::downgrade(&client),
            connection: connection.as_ref().map(Rc::downgrade),
            exact,
            state,
            projection,
        })
    }

    fn snapshot_is_current(&self, snapshot: &Snapshot) -> bool {
        self.capture_snapshot()
            .is_ok_and(|current| snapshot.same_state(&current))
    }

    /// Native composition owns its DOM preview. Outside composition, each
    /// mirror is an immutable snapshot with an opaque revision, including
    /// selection-only changes and focus/reset updates whose text is unchanged.
    pub(super) fn synchronize_dom(&self) {
        self.dom_dirty.set(true);
        if self.is_composing.get() || self.mirroring_dom.replace(true) {
            return;
        }
        let _guard = FlagGuard(&self.mirroring_dom);
        loop {
            self.dom_dirty.set(false);
            let generation = self.generation.get();
            let snapshot = self.capture_snapshot();
            if self.is_composing.get() {
                return;
            }
            if generation != self.generation.get() || self.dom_dirty.get() {
                continue;
            }
            let revision = self.dom.borrow_mut().next_revision();
            let snapshot = snapshot.ok().map(|mut snapshot| {
                snapshot.revision = revision;
                snapshot
            });
            {
                let mut dom = self.dom.borrow_mut();
                dom.snapshot = snapshot.clone();
                dom.pending = None;
            }
            if let Some(snapshot) = snapshot {
                self.page.set_surrounding_text(
                    &snapshot.projection.state.text,
                    snapshot.projection.state.selection.start,
                    snapshot.projection.state.selection.end,
                    revision,
                );
            } else {
                self.page.set_surrounding_text("", 0, 0, revision);
            }
            if !self.dom_dirty.get() && self.generation.get() == generation {
                break;
            }
        }
    }

    pub(super) fn apply_input_options(&self) {
        if self.applying_options.replace(true) {
            return;
        }
        let _guard = FlagGuard(&self.applying_options);
        loop {
            let revision = self.options_revision.get();
            let generation = self.generation.get();
            let Some(mut options) = self.options.borrow().clone() else {
                return;
            };
            let connection = self.connection.borrow().clone();
            options.suggestions &=
                connection.is_some_and(|connection| connection.supports_exact_edits());
            if revision != self.options_revision.get() || generation != self.generation.get() {
                continue;
            }
            if self.applied_options.borrow().as_ref() == Some(&options) {
                return;
            }
            self.applied_options.replace(Some(options.clone()));
            // Attribute-only updates preserve the focused textarea and its
            // native composition. Never retain a borrow across this callback.
            self.page.set_input_options(&options);
            if revision == self.options_revision.get() && generation == self.generation.get() {
                break;
            }
        }
    }

    pub(crate) fn on_dom_before_input(
        &self,
        revision: u32,
        input_type: &str,
        value: &str,
        anchor: i32,
        active: i32,
    ) -> u32 {
        let snapshot = self.dom.borrow().snapshot.clone();
        let selection = TextSelection::new(anchor, active);
        let accepted = snapshot.filter(|snapshot| {
            !self.is_composing.get()
                && snapshot.exact
                && snapshot.projection.selection_is_exact
                && revision != 0
                && snapshot.revision == revision
                && snapshot.projection.state.text == value
                && self.snapshot_is_current(snapshot)
                && snapshot.projection.selection_to_model(selection).is_ok()
        });
        let Some(snapshot) = accepted else {
            self.dom.borrow_mut().pending = None;
            self.synchronize_dom();
            return 0;
        };
        let mut dom = self.dom.borrow_mut();
        let token = dom.next_revision();
        dom.pending = Some(PendingEdit {
            token,
            snapshot,
            selection,
            input_type: input_type.to_owned(),
        });
        token
    }

    /// Consumes one actual DOM value change through the manager's exact edit
    /// route. Failure always restores the current owner's canonical state.
    pub(crate) fn on_dom_input(
        &self,
        revision: u32,
        token: u32,
        input_type: &str,
        value: &str,
        anchor: i32,
        active: i32,
    ) -> bool {
        let result = (|| {
            if self.is_composing.get() || revision == 0 {
                return Err(TextInputMethodError::StaleConnection);
            }
            let pending = self.dom.borrow_mut().pending.take();
            let (snapshot, selection) = if token != 0 {
                let pending = pending
                    .filter(|pending| {
                        pending.token == token
                            && pending.snapshot.revision == revision
                            && pending.input_type == input_type
                    })
                    .ok_or(TextInputMethodError::StaleConnection)?;
                (pending.snapshot, Some(pending.selection))
            } else {
                if pending.is_some() {
                    return Err(TextInputMethodError::StaleConnection);
                }
                let snapshot = self
                    .dom
                    .borrow()
                    .snapshot
                    .clone()
                    .filter(|snapshot| snapshot.revision == revision)
                    .ok_or(TextInputMethodError::StaleConnection)?;
                (snapshot, None)
            };
            if !snapshot.exact
                || !snapshot.projection.selection_is_exact
                || !self.snapshot_is_current(&snapshot)
            {
                return Err(TextInputMethodError::StateChanged);
            }
            let connection = snapshot
                .connection
                .as_ref()
                .and_then(Weak::upgrade)
                .ok_or(TextInputMethodError::StaleConnection)?;
            let after = TextInputMethodState {
                text: value.to_owned(),
                selection: TextSelection::new(anchor, active),
            };
            match edit_from_dom(&snapshot.projection.state, selection, &after, input_type)? {
                Some(edit) => {
                    let edit = snapshot
                        .projection
                        .edit_to_model(&snapshot.state.text, &edit)?;
                    connection.apply_edit(
                        &edit,
                        self.input_handler
                            .upgrade()
                            .map_or(0, |handler| handler.timestamp()),
                    )
                }
                None => {
                    let selection = snapshot.projection.selection_to_model(after.selection)?;
                    if selection != snapshot.state.selection {
                        connection.select_exact(selection)?;
                    }
                    Ok(true)
                }
            }
        })();
        self.synchronize_dom();
        result.unwrap_or(false)
    }

    pub(crate) fn on_dom_composition_start(
        &self,
        revision: u32,
        value: &str,
        anchor: i32,
        active: i32,
    ) -> u32 {
        let snapshot = self.dom.borrow().snapshot.clone();
        let selection = TextSelection::new(anchor, active);
        let Some(mut snapshot) = snapshot.filter(|snapshot| {
            snapshot.revision == revision
                && revision != 0
                && snapshot.projection.selection_is_exact
                && snapshot.projection.state.text == value
                && self.snapshot_is_current(snapshot)
        }) else {
            self.synchronize_dom();
            return 0;
        };
        let Ok(selection) = snapshot.projection.selection_to_model(selection) else {
            self.synchronize_dom();
            return 0;
        };
        if selection != snapshot.state.selection {
            let selected = if snapshot.exact {
                snapshot
                    .connection
                    .as_ref()
                    .and_then(Weak::upgrade)
                    .is_some_and(|connection| connection.select_exact(selection).is_ok())
            } else if let Some(client) = snapshot.client.upgrade() {
                client.set_selection(selection);
                true
            } else {
                false
            };
            let current = self.capture_snapshot();
            if !selected || self.generation.get() != snapshot.generation {
                self.synchronize_dom();
                return 0;
            }
            let Ok(current) = current else {
                self.synchronize_dom();
                return 0;
            };
            snapshot = current;
        }
        self.dom.borrow_mut().pending = None;
        if !self.on_composition_start() {
            return 0;
        }
        let generation = self.generation.get();
        let mut dom = self.dom.borrow_mut();
        let token = dom.next_revision();
        dom.composition = Some(Composition {
            token,
            generation,
            exact: snapshot.exact,
            range: TextSelection::new(
                snapshot
                    .state
                    .selection
                    .start
                    .min(snapshot.state.selection.end),
                snapshot
                    .state
                    .selection
                    .start
                    .max(snapshot.state.selection.end),
            ),
            original: snapshot.state,
            original_dom: snapshot.projection.state.text,
        });
        token
    }

    pub(crate) fn on_dom_composition_update(&self, token: u32, data: &str) {
        let composition = self.dom.borrow().composition.clone();
        let Some(composition) = composition.filter(|composition| {
            composition.token == token && composition.generation == self.generation.get()
        }) else {
            return;
        };
        if !composition.exact {
            self.on_composition_update(Some(data));
            return;
        }
        let data = data.replace("\r\n", "\n").replace('\r', "\n");
        let Some(end) = utf16_length(&data)
            .ok()
            .and_then(|length| composition.range.start.checked_add(length))
        else {
            return;
        };
        let edit = TextInputMethodEdit::new(composition.range, data, TextSelection::new(end, end));
        let session = self.composition_session.borrow().clone();
        if session.is_some_and(|session| session.apply_edit(&edit).is_ok() && session.is_current())
            && self.generation.get() == composition.generation
        {
            // Editing publishes application callbacks. Retain the new span only
            // if those callbacks left this exact native composition installed.
            if let Some(current) = self.dom.borrow_mut().composition.as_mut() {
                if current.token == token && current.generation == composition.generation {
                    current.range = TextSelection::new(composition.range.start, end);
                }
            }
        }
    }

    pub(crate) fn on_dom_composition_end(
        &self,
        token: u32,
        data: &str,
        value: &str,
        anchor: i32,
        active: i32,
    ) {
        let composition = self.dom.borrow().composition.clone();
        let Some(composition) = composition.filter(|composition| {
            composition.token == token && composition.generation == self.generation.get()
        }) else {
            self.synchronize_dom();
            return;
        };
        self.dom.borrow_mut().composition = None;
        // Escape restores the original DOM value and reports empty data. An
        // empty accepted replacement instead removes the selected text.
        let cancelled = data.is_empty() && value == composition.original_dom;
        let data = data.replace("\r\n", "\n").replace('\r', "\n");
        // Validate the entire final value and both directed UTF-16 endpoints,
        // including LF-only models. The committed range belongs to the original
        // model; the native value and selection belong to its LF projection.
        let final_selection = (!cancelled)
            .then(|| {
                let selection = composition.original.selection;
                let edit = TextInputMethodEdit::new(
                    TextSelection::new(
                        selection.start.min(selection.end),
                        selection.start.max(selection.end),
                    ),
                    &data,
                    TextSelection::default(),
                );
                edit.apply_to(&composition.original.text)
                    .and_then(|text| {
                        DomTextProjection::new(&TextInputMethodState {
                            text,
                            selection: TextSelection::default(),
                        })
                    })
                    .and_then(|projection| {
                        if projection.state.text != value {
                            return Err(TextInputMethodError::InvalidRange);
                        }
                        projection.selection_to_model(TextSelection::new(anchor, active))
                    })
            })
            .and_then(Result::ok);
        if composition.exact && data.is_empty() && final_selection.is_some() {
            // Legacy commit("") means cancellation. Complete the exact preview
            // to accept a deletion, with its final selection in the undo unit.
            let session = self.composition_session.borrow().clone();
            if let Some(session) = session {
                let edit =
                    TextInputMethodEdit::new(composition.range, "", final_selection.unwrap());
                let applied = session.apply_edit(&edit);
                if self.generation.get() == composition.generation {
                    self.is_composing.set(false);
                    if applied.is_ok() {
                        let result = session.complete(
                            self.input_handler
                                .upgrade()
                                .map_or(0, |handler| handler.timestamp()),
                        );
                        if result.is_ok() && self.generation.get() == composition.generation {
                            self.composition_session.take();
                        }
                    }
                }
            }
        } else {
            self.on_composition_end(final_selection.is_some().then_some(data.as_str()));
        }
        if self.generation.get() == composition.generation {
            self.is_composing.set(false);
            // The browser ended this native session and cannot retry a rejected
            // commit. Roll back its retained preview before mirroring the model.
            let rejected = self.composition_session.borrow().clone();
            if let Some(session) = rejected {
                let _ = session.cancel();
            }
            if self.generation.get() == composition.generation {
                self.composition_session.take();
                if final_selection.is_some() {
                    if let Ok(snapshot) = self.capture_snapshot() {
                        let selection = TextSelection::new(anchor, active);
                        if snapshot.exact && snapshot.projection.state.text == value {
                            if let Some(connection) =
                                snapshot.connection.as_ref().and_then(Weak::upgrade)
                            {
                                if let Ok(selection) =
                                    snapshot.projection.selection_to_model(selection)
                                {
                                    if snapshot.state.selection != selection {
                                        let _ = connection.select_exact(selection);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        self.synchronize_dom();
    }
}

#[cfg(test)]
#[path = "browser_text_input_dom_tests.rs"]
mod tests;

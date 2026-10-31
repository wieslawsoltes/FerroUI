use crate::browser_input_handler::BrowserInputHandler;
use crate::browser_text_input_options::BrowserTextInputOptions;
use crate::interop::{input_helper, JsObject};
use ferroui_base::input::text_input::{
    ITextInputMethodImpl, TextInputMethodClient, TextInputMethodComposition, TextInputMethodConnection,
    TextInputOptions, TextSelection,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::Rect;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The calls the text input method makes into the page: the hidden input
/// element of the view and the element of the view itself. Replaced by a
/// recorder in the tests.
///
/// Not in the original: a view has a second hidden element, the textarea of a
/// client of the exact-edit connection (`docs/porting/browser-platform.md`,
/// section 14). [`use_text_input_element`](Self::use_text_input_element)
/// chooses the element that the hide, show, focus, bounds and selection calls
/// act on; every other client keeps the input element of the original.
pub(crate) trait ITextInputPage {
    fn hide_input_element(&self);
    fn show_input_element(&self);
    fn focus_input_element(&self);
    fn focus_container_element(&self);
    fn clear_input_element(&self);
    fn set_surrounding_text(&self, text: &str, start: i32, end: i32);
    fn set_selection(&self, start: i32, end: i32);
    fn set_options(&self, options: &TextInputOptions);
    fn set_bounds(&self, x: i32, y: i32, width: i32, height: i32, caret: i32);
    fn selection_start(&self) -> i32;
    fn selection_end(&self) -> i32;
    /// Whether the view has the textarea of connection clients.
    fn has_text_input_element(&self) -> bool;
    /// Makes the textarea (`true`) or the input element (`false`) the element
    /// of the client, hiding the other one.
    fn use_text_input_element(&self, text_input: bool);
    /// Empties the textarea and gives it the mirror revision `revision`.
    fn clear_text_input_element(&self, revision: u32);
    /// Mirrors a snapshot of a connection client into the textarea.
    fn set_text_input_state(&self, text: &str, anchor: i32, active: i32, revision: u32);
    /// Sets the input hints of the textarea as attributes.
    fn set_input_options(&self, options: &BrowserTextInputOptions);
}

struct TextInputPage {
    input_element: JsObject,
    container_element: JsObject,
    /// The textarea of connection clients, `None` in a page whose script does not create one.
    text_input_element: Option<JsObject>,
    uses_text_input_element: Cell<bool>,
}

impl TextInputPage {
    /// The element of the current client.
    fn element(&self) -> &JsObject {
        match &self.text_input_element {
            Some(text_input_element) if self.uses_text_input_element.get() => text_input_element,
            _ => &self.input_element,
        }
    }
}

impl ITextInputPage for TextInputPage {
    fn hide_input_element(&self) {
        input_helper::hide_element(self.element());
    }

    fn show_input_element(&self) {
        input_helper::show_element(self.element());
    }

    fn focus_input_element(&self) {
        input_helper::focus_element(self.element());
    }

    fn focus_container_element(&self) {
        input_helper::focus_element(&self.container_element);
    }

    fn clear_input_element(&self) {
        input_helper::clear_input_element(&self.input_element);
    }

    fn set_surrounding_text(&self, text: &str, start: i32, end: i32) {
        input_helper::set_surrounding_text(&self.input_element, text, start, end);
    }

    fn set_selection(&self, start: i32, end: i32) {
        input_helper::set_selection(&self.input_element, start, end);
    }

    fn set_options(&self, options: &TextInputOptions) {
        let options = BrowserInputOptions::from_options(options);
        input_helper::set_options(
            &self.input_element, options.input_type, options.input_mode, options.enter_key_hint,
            options.capitalization, options.suggestions.map_or(-1, i32::from), options.language, options.multiline, options.sensitive,
        );
    }

    fn set_bounds(&self, x: i32, y: i32, width: i32, height: i32, caret: i32) {
        input_helper::set_bounds(self.element(), x, y, width, height, caret);
    }

    fn selection_start(&self) -> i32 {
        input_helper::get_selection_start(self.element())
    }

    fn selection_end(&self) -> i32 {
        input_helper::get_selection_end(self.element())
    }

    fn has_text_input_element(&self) -> bool {
        self.text_input_element.is_some()
    }

    fn use_text_input_element(&self, text_input: bool) {
        let text_input = text_input && self.text_input_element.is_some();
        if self.uses_text_input_element.get() != text_input {
            input_helper::hide_element(self.element());
            self.uses_text_input_element.set(text_input);
        }
    }

    fn clear_text_input_element(&self, revision: u32) {
        if let Some(text_input_element) = &self.text_input_element {
            input_helper::clear_text_input_element(text_input_element, revision);
        }
    }

    fn set_text_input_state(&self, text: &str, anchor: i32, active: i32, revision: u32) {
        if let Some(text_input_element) = &self.text_input_element {
            input_helper::set_text_input_state(text_input_element, text, anchor, active, revision);
        }
    }

    fn set_input_options(&self, options: &BrowserTextInputOptions) {
        if let Some(text_input_element) = &self.text_input_element {
            input_helper::set_input_options(
                text_input_element,
                options.input_mode,
                options.enter_key_hint,
                options.capitalization,
                options.suggestions,
                options.sensitive,
                &options.language,
            );
        }
    }
}

// HTML input modes retain text/password inputs, which support UTF16 selection APIs.
// Email/number input *types* would reject the selection operations used by the bridge.
#[derive(Debug, PartialEq)]
struct BrowserInputOptions<'a> {
    input_type: &'static str,
    input_mode: &'static str,
    enter_key_hint: &'static str,
    capitalization: &'static str,
    suggestions: Option<bool>,
    language: &'a str,
    multiline: bool,
    sensitive: bool,
}
impl<'a> BrowserInputOptions<'a> {
    fn from_options(options: &'a TextInputOptions) -> Self {
        use ferroui_base::input::text_input::{TextInputContentType as Content, TextInputReturnKeyType as Return};
        let password = matches!(options.content_type, Content::Password | Content::Pin);
        let private = options.is_sensitive || password;
        Self {
            input_type: if password { "password" } else { "text" },
            sensitive: private,
            // A textarea cannot provide a secure password input. Privacy takes precedence.
            multiline: options.multiline && !password,
            input_mode: match options.content_type {
                Content::Digits | Content::Pin => "numeric",
                Content::Number => "decimal",
                Content::Email => "email",
                Content::Url => "url",
                Content::Search => "search",
                Content::Normal | Content::Alpha | Content::Name | Content::Password | Content::Social => "text",
            },
            enter_key_hint: match options.return_key_type {
                Return::Default => "",
                Return::Return => "enter",
                Return::Done => "done",
                Return::Go => "go",
                Return::Send => "send",
                Return::Search => "search",
                Return::Next => "next",
                Return::Previous => "previous",
            },
            capitalization: if private {
                "none"
            } else if options.uppercase {
                "characters"
            } else if options.lowercase {
                "none"
            } else if options.auto_capitalization {
                "sentences"
            } else if options.content_type == Content::Name {
                "words"
            } else {
                "none"
            },
            suggestions: if private { Some(false) } else { options.show_suggestions },
            language: options.locale_hints.as_ref().and_then(|values| values.first()).map_or("", String::as_str),
        }
    }
}

/// The text input method of a view: the hidden input element of the view
/// stands in for the text the client edits, so that the input method of
/// the browser has an element to compose into.
pub struct BrowserTextInputMethod {
    weak_self: Weak<BrowserTextInputMethod>,
    page: Box<dyn ITextInputPage>,
    input_handler: Weak<BrowserInputHandler>,
    client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
    /// The subscriptions to the events of the current client.
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    is_composing: Cell<bool>,
    disposed: Cell<bool>,
    client_generation: Cell<u64>,
    text_revision: Cell<u64>,
    /// Set while the page reports that the keyboard focus left the view.
    focus_is_leaving_view: Cell<bool>,
    // Not in the original: the state of a client of the exact-edit connection (decision D5 of
    // docs/porting/winui-rich-text.md; browser-platform.md, section 14). `connection` is `None`
    // for every other client, which keeps the path of the original.
    connection: RefCell<Option<Rc<TextInputMethodConnection>>>,
    composition_session: RefCell<Option<TextInputMethodComposition>>,
    /// Advanced by every change of client, reset and composition start, so that a callback
    /// into application code that replaced the client ends the older operation.
    generation: Cell<u64>,
    /// The last options given to `set_options`; applied to the textarea of a connection client.
    options: RefCell<Option<TextInputOptions>>,
    applied_options: RefCell<Option<BrowserTextInputOptions>>,
    options_revision: Cell<u64>,
    applying_options: Cell<bool>,
    dom: RefCell<dom::DomState>,
    client_revision: Cell<u64>,
    mirroring_dom: Cell<bool>,
    dom_dirty: Cell<bool>,
}

impl BrowserTextInputMethod {
    /// Creates the text input method of the view in `container_element`,
    /// whose hidden input element is `input_element`.
    ///
    /// # Panics
    /// Panics when one of the elements is null or undefined.
    pub(crate) fn new(
        input_handler: Weak<BrowserInputHandler>,
        container_element: JsObject,
        input_element: JsObject,
    ) -> Rc<Self> {
        if input_element.is_null() || input_element.is_undefined() {
            panic!("Value cannot be null. (Parameter 'inputElement')");
        }
        if container_element.is_null() || container_element.is_undefined() {
            panic!("Value cannot be null. (Parameter 'containerElement')");
        }

        let text_input_element = input_helper::get_text_input_element(&input_element);
        let text_input_element =
            (!text_input_element.is_null() && !text_input_element.is_undefined()).then_some(text_input_element);
        Self::with_page(
            input_handler,
            Box::new(TextInputPage {
                input_element,
                container_element,
                text_input_element,
                uses_text_input_element: Cell::new(false),
            }),
        )
    }

    pub(crate) fn with_page(input_handler: Weak<BrowserInputHandler>, page: Box<dyn ITextInputPage>) -> Rc<Self> {
        Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            page,
            input_handler,
            client: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
            is_composing: Cell::new(false),
            disposed: Cell::new(false),
            client_generation: Cell::new(0),
            text_revision: Cell::new(0),
            focus_is_leaving_view: Cell::new(false),
            connection: RefCell::new(None),
            composition_session: RefCell::new(None),
            generation: Cell::new(0),
            options: RefCell::new(None),
            applied_options: RefCell::new(None),
            options_revision: Cell::new(0),
            applying_options: Cell::new(false),
            dom: RefCell::new(dom::DomState::default()),
            client_revision: Cell::new(0),
            mirroring_dom: Cell::new(false),
            dom_dirty: Cell::new(false),
        })
    }

    fn uses_connection(&self) -> bool {
        self.connection.borrow().is_some()
    }

    fn next_generation(&self) -> u64 {
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        generation
    }

    fn timestamp(&self) -> u64 {
        self.input_handler.upgrade().map_or(0, |handler| handler.timestamp())
    }

    /// Whether the input method of the browser is composing text.
    pub fn is_composing(&self) -> bool {
        self.is_composing.get()
    }

    fn client(&self) -> Option<Rc<dyn TextInputMethodClient>> {
        self.client.borrow().clone()
    }

    fn current(&self, generation: u64) -> bool {
        !self.disposed.get() && self.client_generation.get() == generation
    }

    fn handler(&self, generation: u64, action: fn(&BrowserTextInputMethod)) -> Rc<dyn Fn()> {
        let this = self.weak_self.clone();
        Rc::new(move || {
            if let Some(this) = this.upgrade() {
                if this.current(generation) {
                    action(&this);
                }
            }
        })
    }

    /// Runs `action` as the reaction to the keyboard focus of the page
    /// having left the view: a client that goes away meanwhile does not
    /// bring the focus back.
    pub(crate) fn while_focus_leaves_view(&self, action: impl FnOnce()) {
        let previous = self.focus_is_leaving_view.replace(true);
        action();
        self.focus_is_leaving_view.set(previous);
    }

    /// Gives the focus of the page to the hidden input element, unless the
    /// focus is just leaving the view.
    // Differs from the original, which focuses unconditionally: a client that updates its
    // cursor while it loses the focus would take the focus back from the element of the page
    // the user went to.
    fn focus_input_element(&self) {
        if !self.disposed.get() && !self.focus_is_leaving_view.get() {
            self.page.focus_input_element();
        }
    }

    fn hide_ime(&self) {
        self.page.hide_input_element();
        // Differs from the original, which always gives the focus back to the element of the
        // view. There the framework never learns that the page moved the focus elsewhere; here
        // it does, drops its focused element and with it the client, and focusing the view then
        // would take the focus back from the element of the page the user just went to.
        if !self.disposed.get() && !self.focus_is_leaving_view.get() {
            self.page.focus_container_element();
        }
    }

    fn input_pane_activation_requested(&self) {
        if self.client().is_some() {
            self.show_ime();
        }
    }

    fn show_ime(&self) {
        self.page.show_input_element();
        self.focus_input_element();
    }

    fn surrounding_text_changed(&self) {
        if self.disposed.get() { return; }
        if self.uses_connection() {
            // A connection client is mirrored as revisioned snapshots (selection changes too).
            self.client_revision.set(self.client_revision.get().wrapping_add(1));
            self.synchronize_dom();
            return;
        }
        let generation = self.client_generation.get();
        let revision = self.text_revision.get().wrapping_add(1);
        self.text_revision.set(revision);
        if let Some(client) = self.client() {
            let surrounding_text = client.surrounding_text();
            if !self.current(generation) || self.text_revision.get() != revision {
                return;
            }
            let selection = client.selection();
            if self.current(generation) && self.text_revision.get() == revision {
                self.page.set_surrounding_text(&surrounding_text, selection.start, selection.end);
            }
        }
    }

    // A completed edit can publish surrounding text before its final selection.
    // Mirror selection independently without copying text, resizing or taking focus.
    fn selection_changed(&self) {
        if self.disposed.get() { return; }
        if self.uses_connection() { self.surrounding_text_changed(); return; }
        if !self.is_composing.get() {
            if let Some(client) = self.client() {
                let generation = self.client_generation.get();
                let revision = self.text_revision.get();
                let selection = client.selection();
                if self.current(generation) && self.text_revision.get() == revision && !self.is_composing.get() {
                    self.page.set_selection(selection.start, selection.end);
                }
            }
        }
    }

    /// The hidden input element is about to change: `input_type` is the
    /// kind of the change and `start` and `end` the range it targets.
    pub fn on_before_input(&self, input_type: &str, mut start: i32, mut end: i32) {
        if self.disposed.get() || self.uses_connection() {
            return;
        }
        if input_type != "deleteByComposition" {
            if input_type == "deleteContentBackward" {
                start = self.page.selection_start();
                end = self.page.selection_end();
            } else {
                start = -1;
                end = -1;
            }
        }

        if start != -1 && end != -1 {
            if let Some(client) = self.client() {
                client.set_selection(TextSelection::new(start, end));
            }
        }
    }

    /// Inserts text that arrived without a keydown. The native client's
    /// selection is authoritative: the hidden input may still contain the
    /// surrounding text from before a native selection change.
    pub fn on_text_input(&self, data: Option<&str>) -> bool {
        if self.is_composing() || self.client().is_none() || self.uses_connection() {
            return false;
        }
        let Some(data) = data.filter(|data| !data.is_empty()) else {
            return false;
        };
        let handled = self.input_handler.upgrade().is_some_and(|input_handler| input_handler.raw_text_event(data));
        // A TextBox raises its text notification before it finishes moving the
        // caret. Mirror again after dispatch so the DOM also gets the final
        // selection (and any filtering performed by the native control).
        self.surrounding_text_changed();
        handled
    }

    /// A native focus change or reset ends the old client's composition even
    /// when the browser delivers its compositionend later, or not at all.
    fn cancel_composition(&self) {
        if self.is_composing.replace(false) {
            if let Some(client) = self.client() {
                client.set_preedit_text(None);
            }
        }
    }

    /// The input method of the browser started composing.
    pub fn on_composition_start(&self) {
        if self.uses_connection() {
            return;
        }
        let Some(client) = self.client() else {
            return;
        };

        // Set before calling the client: a synchronous reset/detach must win.
        self.is_composing.set(true);
        client.set_preedit_text(None);
    }

    /// The text being composed changed.
    pub fn on_composition_update(&self, data: Option<&str>) {
        if !self.is_composing() || self.uses_connection() {
            return;
        }
        let Some(client) = self.client() else {
            return;
        };

        client.set_preedit_text(data);
    }

    /// The input method of the browser finished composing; `data` is the
    /// text it committed.
    pub fn on_composition_end(&self, data: Option<&str>) {
        if self.uses_connection() {
            return;
        }
        // Always leave composing before looking up the client. A delayed end
        // after reset/client replacement belongs to the canceled composition.
        if !self.is_composing.replace(false) {
            return;
        }
        let Some(client) = self.client() else {
            return;
        };

        let generation = self.client_generation.get();
        let revision = self.text_revision.get();
        client.set_preedit_text(None);
        if !self.current(generation) || self.text_revision.get() != revision || self.is_composing.get() {
            return;
        }

        // Clearing preedit can raise native callbacks that move focus. Never
        // deliver the old client's commit to the newly focused control.
        if !self.client().is_some_and(|current| Rc::ptr_eq(&current, &client)) {
            return;
        }
        if let Some(data) = data {
            if let Some(input_handler) = self.input_handler.upgrade() {
                input_handler.raw_text_event(data);
            }
        }
        if self.current(generation) {
            self.selection_changed();
        }
    }

    /// Retires the native editor without moving page focus to a closing view.
    pub(crate) fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        self.client_generation.set(self.client_generation.get().wrapping_add(1));
        self.next_generation();
        let was_connection = self.uses_connection();
        let dom_revision = self.invalidate_dom();
        self.composition_session.take();
        self.connection.take();
        let composing = self.is_composing.replace(false);
        let old_client = self.client.take();
        let subscriptions = ClientSubscriptions(self.subscriptions.take());
        self.page.clear_input_element();
        if was_connection {
            self.page.clear_text_input_element(dom_revision);
        }
        self.page.hide_input_element();
        drop(subscriptions);
        if composing {
            if let Some(client) = old_client.as_ref() {
                client.set_preedit_text(None);
            }
        }
        drop(old_client);
    }
}

// Pending installations and attached subscriptions use the same RAII cleanup.
// Client callbacks run with no RefCell borrow held, including during unwinding.
#[derive(Default)]
struct ClientSubscriptions(Vec<Rc<dyn IDisposable>>);

impl Drop for ClientSubscriptions {
    fn drop(&mut self) {
        for subscription in self.0.drain(..) {
            subscription.dispose();
        }
    }
}

impl Drop for BrowserTextInputMethod {
    fn drop(&mut self) {
        self.dispose();
    }
}

impl BrowserTextInputMethod {
    /// The textarea of a connection client started composing: a composition session of the
    /// connection, which keeps the selected text until the first update.
    fn begin_connection_composition(&self) -> bool {
        let generation = self.next_generation();
        if self.client().is_none() {
            return false;
        }
        let Some(connection) = self.connection.borrow().clone() else {
            return false;
        };
        self.is_composing.set(true);
        self.composition_session.take();
        let result = connection.begin_composition();
        if self.generation.get() != generation {
            return false;
        }
        let Ok(session) = result else {
            self.is_composing.set(false);
            self.synchronize_dom();
            return false;
        };
        *self.composition_session.borrow_mut() = Some(session);
        self.generation.get() == generation && self.is_composing.get()
    }

    /// The preview of a connection client's composition changed.
    fn update_connection_composition(&self, data: &str) {
        if self.client().is_none() {
            return;
        }
        let session = self.composition_session.borrow().clone();
        if let Some(session) = session {
            let _ = session.update(data, None);
        }
    }

    /// The composition of a connection client ended: `data` is committed, `None` cancels.
    fn end_connection_composition(&self, data: Option<&str>) {
        let generation = self.generation.get();
        if self.client().is_none() {
            return;
        }
        self.is_composing.set(false);
        let session = self.composition_session.borrow().clone();
        if let Some(session) = session {
            // Commit the accepted marked replacement directly. Clearing it
            // first would destroy formatting and split one edit into two.
            let result = match data {
                Some(data) => session.commit(data, self.timestamp()),
                None => session.cancel(),
            };
            if result.is_ok() && self.generation.get() == generation {
                self.composition_session.take();
            }
        }
    }

    /// `set_client` when the old or the new client is a client of the connection.
    fn set_connection_client(
        &self,
        client: Option<Rc<dyn TextInputMethodClient>>,
        connection: Option<Rc<TextInputMethodConnection>>,
        generation: u64,
    ) {
        let was_connection = self.uses_connection();
        let dom_revision = self.invalidate_dom();
        if was_connection {
            self.is_composing.set(false);
            self.composition_session.take();
            self.connection.take();
            if let Some(previous) = self.client() {
                previous.set_preedit_text(None);
            }
        } else {
            self.cancel_composition();
        }
        if self.generation.get() != generation {
            return;
        }
        *self.connection.borrow_mut() = connection.clone();
        if self.client().is_some() {
            let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
            for subscription in subscriptions {
                subscription.dispose();
            }
        }
        if self.generation.get() != generation {
            return;
        }

        if let Some(client) = &client {
            let mut subscriptions = vec![
                client.surrounding_text_changed(self.handler(self.client_generation.get(), Self::surrounding_text_changed)),
                client.input_pane_activation_requested(self.handler(self.client_generation.get(), Self::input_pane_activation_requested)),
            ];
            if connection.is_some() {
                subscriptions.push(client.selection_changed(self.handler(self.client_generation.get(), Self::surrounding_text_changed)));
            } else {
                subscriptions.push(client.selection_changed(self.handler(self.client_generation.get(), Self::selection_changed)));
            }
            if self.generation.get() != generation {
                for subscription in subscriptions {
                    subscription.dispose();
                }
                return;
            }
            *self.subscriptions.borrow_mut() = subscriptions;
        }

        self.page.clear_text_input_element(dom_revision);
        self.page.use_text_input_element(connection.is_some());
        // Retire the legacy peer even when switching to the connection textarea.
        // A hidden password or multiline editor must not retain its old value.
        self.page.clear_input_element();
        if self.generation.get() != generation {
            return;
        }

        let old_client = self.client.replace(client);
        drop(old_client);

        if let Some(client) = self.client() {
            self.show_ime();
            if self.generation.get() != generation {
                return;
            }
            if connection.is_some() {
                self.synchronize_dom();
            } else {
                let surrounding_text = client.surrounding_text();
                let selection = client.selection();
                self.page.set_surrounding_text(&surrounding_text, selection.start, selection.end);
            }
        } else {
            self.hide_ime();
        }
        self.apply_input_options();
    }
}

impl ITextInputMethodImpl for BrowserTextInputMethod {
    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
        if self.disposed.get() {
            return;
        }
        let generation = self.client_generation.get().wrapping_add(1);
        self.client_generation.set(generation);
        let connection_generation = self.next_generation();
        let connection = client.as_ref()
            .filter(|_| self.page.has_text_input_element())
            .and_then(|client| client.input_method_editing().and_then(|_| client.input_method_connection()));
        if self.generation.get() != connection_generation || !self.current(generation) { return; }
        if self.uses_connection() || connection.is_some() {
            self.set_connection_client(client, connection, connection_generation);
            return;
        }
        let composing = self.is_composing.replace(false);
        let old_client = self.client.take();
        let old_subscriptions = ClientSubscriptions(self.subscriptions.take());
        self.page.clear_input_element();
        drop(old_subscriptions);
        if composing {
            if let Some(client) = old_client.as_ref() {
                client.set_preedit_text(None);
            }
        }
        drop(old_client);
        if !self.current(generation) {
            return;
        }

        self.client.replace(client.clone());
        if let Some(client) = client {
            let mut subscriptions = ClientSubscriptions::default();
            subscriptions.0.push(client.surrounding_text_changed(self.handler(generation, Self::surrounding_text_changed)));
            if !self.current(generation) { return; }
            subscriptions.0.push(client.selection_changed(self.handler(generation, Self::selection_changed)));
            if !self.current(generation) { return; }
            subscriptions.0.push(client.input_pane_activation_requested(self.handler(generation, Self::input_pane_activation_requested)));
            if !self.current(generation) { return; }
            *self.subscriptions.borrow_mut() = std::mem::take(&mut subscriptions.0);
            self.show_ime();
            if self.current(generation) {
                self.surrounding_text_changed();
            }
        } else {
            self.hide_ime();
        }
    }

    fn set_cursor_rect(&self, rect: Rect) {
        if self.disposed.get() { return; }
        let generation = self.client_generation.get();
        self.focus_input_element();
        if !self.current(generation) { return; }
        let caret = self.client().map_or(0, |client| client.selection().end);
        if !self.current(generation) { return; }
        self.page.set_bounds(rect.x as i32, rect.y as i32, rect.width as i32, rect.height as i32, caret);
        self.focus_input_element();
    }

    fn set_options(&self, options: &TextInputOptions) {
        if self.disposed.get() { return; }
        self.options.replace(Some(options.clone()));
        self.options_revision.set(self.options_revision.get().wrapping_add(1));
        if self.uses_connection() { self.apply_input_options(); }
        else { self.page.set_options(options); }
    }

    fn reset(&self) {
        if self.disposed.get() { return; }
        if self.uses_connection() { self.reset_connection_client(); return; }
        self.text_revision.set(self.text_revision.get().wrapping_add(1));
        let composing = self.is_composing.replace(false);
        let client = self.client();
        self.page.clear_input_element();
        self.page.set_surrounding_text("", 0, 0);
        if composing {
            if let Some(client) = client {
                client.set_preedit_text(None);
            }
        }
    }
}

impl BrowserTextInputMethod {
    fn reset_connection_client(&self) {
        let generation = self.next_generation();
        let dom_revision = self.invalidate_dom();
        self.is_composing.set(false);
        self.composition_session.take();
        if let Some(client) = self.client() {
            client.set_preedit_text(None);
        }
        if self.generation.get() != generation {
            return;
        }
        self.page.clear_text_input_element(dom_revision);
        if self.generation.get() == generation {
            self.synchronize_dom();
        }
    }
}

#[path = "browser_text_input_dom.rs"]
mod dom;

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::input::text_input::TextInputMethodClientEvents;
    use ferroui_base::{Ref, Visual};

    type Calls = Rc<RefCell<Vec<String>>>;

    struct RecordingPage {
        calls: Calls,
        selection: Rc<Cell<(i32, i32)>>,
    }

    impl RecordingPage {
        fn log(&self, call: impl Into<String>) {
            self.calls.borrow_mut().push(call.into());
        }
    }

    impl ITextInputPage for RecordingPage {
        fn hide_input_element(&self) {
            self.log("hide");
        }

        fn show_input_element(&self) {
            self.log("show");
        }

        fn focus_input_element(&self) {
            self.log("focus input");
        }

        fn focus_container_element(&self) {
            self.log("focus container");
        }

        fn clear_input_element(&self) {
            self.log("clear");
        }

        fn set_surrounding_text(&self, text: &str, start: i32, end: i32) {
            self.log(format!("text '{text}' {start} {end}"));
        }

        fn set_selection(&self, start: i32, end: i32) {
            self.log(format!("selection {start} {end}"));
            self.selection.set((start, end));
        }

        fn set_options(&self, options: &TextInputOptions) {
            self.log(format!("options {options:?}"));
        }

        fn set_bounds(&self, x: i32, y: i32, width: i32, height: i32, caret: i32) {
            self.log(format!("bounds {x} {y} {width} {height} caret {caret}"));
        }

        fn selection_start(&self) -> i32 {
            self.selection.get().0
        }

        fn selection_end(&self) -> i32 {
            self.selection.get().1
        }

        fn has_text_input_element(&self) -> bool {
            true
        }

        fn use_text_input_element(&self, text_input: bool) {
            self.log(format!("use {}", if text_input { "textarea" } else { "input" }));
        }

        fn clear_text_input_element(&self, _revision: u32) {
            self.log("clear textarea");
        }

        fn set_text_input_state(&self, text: &str, anchor: i32, active: i32, _revision: u32) {
            self.log(format!("textarea '{text}' {anchor} {active}"));
        }

        fn set_input_options(&self, options: &BrowserTextInputOptions) {
            self.log(format!("options {} {}", options.input_mode, options.enter_key_hint));
        }
    }

    struct TestClient {
        events: TextInputMethodClientEvents,
        text: RefCell<String>,
        text_reads: Cell<usize>,
        selection: Cell<TextSelection>,
        preedit: RefCell<Vec<Option<String>>>,
        preedit_hook: RefCell<Option<Box<dyn FnOnce()>>>,
        text_hook: RefCell<Option<Box<dyn FnOnce()>>>,
    }

    impl TestClient {
        fn new(text: &str, start: i32, end: i32) -> Rc<Self> {
            Rc::new(Self {
                events: TextInputMethodClientEvents::new(),
                text: RefCell::new(text.to_string()),
                text_reads: Cell::new(0),
                selection: Cell::new(TextSelection::new(start, end)),
                preedit: RefCell::new(Vec::new()),
                preedit_hook: RefCell::new(None),
                text_hook: RefCell::new(None),
            })
        }
    }

    impl TextInputMethodClient for TestClient {
        fn events(&self) -> &TextInputMethodClientEvents {
            &self.events
        }

        fn text_view_visual(&self) -> Option<Ref<Visual>> {
            None
        }

        fn supports_preedit(&self) -> bool {
            true
        }

        fn supports_surrounding_text(&self) -> bool {
            true
        }

        fn surrounding_text(&self) -> String {
            self.text_reads.set(self.text_reads.get() + 1);
            let text = self.text.borrow().clone();
            let hook = self.text_hook.take();
            if let Some(hook) = hook { hook(); }
            text
        }

        fn cursor_rectangle(&self) -> Rect {
            Rect::default()
        }

        fn selection(&self) -> TextSelection {
            self.selection.get()
        }

        fn set_selection(&self, value: TextSelection) {
            self.selection.set(value);
        }

        fn set_preedit_text(&self, preedit_text: Option<&str>) {
            self.preedit.borrow_mut().push(preedit_text.map(str::to_string));
            let hook = self.preedit_hook.take();
            if let Some(hook) = hook { hook(); }
        }
    }

    struct Fixture {
        method: Rc<BrowserTextInputMethod>,
        calls: Calls,
        selection: Rc<Cell<(i32, i32)>>,
    }

    impl Fixture {
        fn new() -> Self {
            let calls: Calls = Rc::new(RefCell::new(Vec::new()));
            let selection = Rc::new(Cell::new((0, 0)));
            let page = RecordingPage { calls: calls.clone(), selection: selection.clone() };
            let method = BrowserTextInputMethod::with_page(Weak::new(), Box::new(page));
            Self { method, calls, selection }
        }

        fn take_calls(&self) -> Vec<String> {
            std::mem::take(&mut *self.calls.borrow_mut())
        }

        fn set_client(&self, client: &Rc<TestClient>) {
            let client: Rc<dyn TextInputMethodClient> = client.clone();
            self.method.set_client(Some(client));
        }
    }

    #[test]
    fn a_client_shows_the_input_element_and_mirrors_its_text() {
        let fixture = Fixture::new();
        let client = TestClient::new("hello", 1, 3);

        fixture.set_client(&client);

        assert_eq!(vec!["clear", "show", "focus input", "text 'hello' 1 3"], fixture.take_calls());
    }

    #[test]
    fn multiline_source_snapshots_keep_utf16_offsets_and_original_delimiters() {
        let fixture = Fixture::new();
        let text = "a\r\n😀\rb\nc";
        fixture.set_client(&TestClient::new(text, 5, 9));
        assert_eq!(vec!["clear".to_owned(), "show".to_owned(), "focus input".to_owned(),
            format!("text '{text}' 5 9")], fixture.take_calls());
    }

    #[test]
    fn without_a_client_the_input_element_is_hidden_and_the_view_focused() {
        let fixture = Fixture::new();
        fixture.set_client(&TestClient::new("hello", 1, 3));
        fixture.take_calls();

        fixture.method.set_client(None);

        assert_eq!(vec!["clear", "hide", "focus container"], fixture.take_calls());
    }

    #[test]
    fn a_client_that_goes_away_because_the_focus_left_the_view_does_not_take_the_focus_back() {
        let fixture = Fixture::new();
        fixture.set_client(&TestClient::new("hello", 1, 3));
        fixture.take_calls();

        fixture.method.while_focus_leaves_view(|| {
            // The client moves its cursor as it loses the focus, and then goes away.
            fixture.method.set_cursor_rect(Rect::new(1.0, 2.0, 3.0, 4.0));
            fixture.method.set_client(None);
        });
        let calls = fixture.take_calls();
        assert!(!calls.iter().any(|call| call.starts_with("focus")), "{calls:?}");
        assert_eq!(Some(&"hide".to_string()), calls.last());

        // Afterwards the usual behaviour is back.
        fixture.set_client(&TestClient::new("hello", 1, 3));
        fixture.take_calls();
        fixture.method.set_client(None);
        assert_eq!(vec!["clear", "hide", "focus container"], fixture.take_calls());
    }

    #[test]
    fn a_change_of_the_surrounding_text_of_the_client_is_mirrored() {
        let fixture = Fixture::new();
        let client = TestClient::new("hello", 1, 3);
        fixture.set_client(&client);
        fixture.take_calls();

        *client.text.borrow_mut() = "hello world".to_string();
        client.selection.set(TextSelection::new(11, 11));
        client.raise_surrounding_text_changed();

        assert_eq!(vec!["text 'hello world' 11 11"], fixture.take_calls());
    }

    #[test]
    fn selection_after_text_replacement_reaches_the_page_without_rewriting_text() {
        let fixture = Fixture::new();
        let client = TestClient::new("Qw", 2, 2);
        fixture.set_client(&client);
        fixture.take_calls();

        *client.text.borrow_mut() = "Qwe".to_string();
        client.raise_surrounding_text_changed();
        client.selection.set(TextSelection::new(3, 3));
        client.raise_selection_changed();

        assert_eq!(vec!["text 'Qwe' 2 2", "selection 3 3"], fixture.take_calls());
    }

    #[test]
    fn selection_only_changes_never_read_or_rewrite_the_text_or_focus_the_page() {
        let fixture = Fixture::new();
        let client = TestClient::new("a😀bc", 0, 0);
        fixture.set_client(&client);
        fixture.take_calls();
        let reads = client.text_reads.get();

        for (start, end) in [(1, 3), (5, 1), (0, 0), (5, 5)] {
            client.selection.set(TextSelection::new(start, end));
            client.raise_selection_changed();
            assert_eq!(vec![format!("selection {start} {end}")], fixture.take_calls());
            assert_eq!((start, end), fixture.selection.get());
            assert_eq!(reads, client.text_reads.get());
        }
    }

    #[test]
    fn selection_updates_leave_browser_composition_alone_until_it_ends() {
        let fixture = Fixture::new();
        let client = TestClient::new("kana", 4, 4);
        fixture.set_client(&client);
        fixture.take_calls();
        fixture.method.on_composition_start();
        client.selection.set(TextSelection::new(1, 3));
        client.raise_selection_changed();
        assert!(fixture.take_calls().is_empty());
        fixture.method.on_composition_update(Some("ka"));
        assert!(fixture.take_calls().is_empty());
        fixture.method.on_composition_end(None);
        assert_eq!(vec!["selection 1 3"], fixture.take_calls());
        assert!(!fixture.method.is_composing());
    }

    #[test]
    fn a_selection_subscription_does_not_retain_the_input_method() {
        let fixture = Fixture::new();
        let client = TestClient::new("hello", 0, 0);
        fixture.set_client(&client);
        let weak = Rc::downgrade(&fixture.method);
        let calls = fixture.calls.clone();
        fixture.take_calls();
        drop(fixture);
        assert!(weak.upgrade().is_none());
        assert_eq!(vec!["clear", "hide"], std::mem::take(&mut *calls.borrow_mut()));
        client.raise_selection_changed();
        assert!(calls.borrow().is_empty());
    }

    #[test]
    fn a_request_for_the_input_pane_shows_the_input_element_again() {
        let fixture = Fixture::new();
        let client = TestClient::new("", 0, 0);
        fixture.set_client(&client);
        fixture.take_calls();

        client.raise_input_pane_activation_requested();

        assert_eq!(vec!["show", "focus input"], fixture.take_calls());
    }

    #[test]
    fn the_events_of_a_replaced_client_are_no_longer_followed() {
        let fixture = Fixture::new();
        let first = TestClient::new("first", 0, 0);
        let second = TestClient::new("second", 2, 2);
        fixture.set_client(&first);
        fixture.set_client(&second);
        fixture.take_calls();

        first.raise_selection_changed();
        first.raise_surrounding_text_changed();
        first.raise_input_pane_activation_requested();
        assert!(fixture.take_calls().is_empty());

        second.raise_surrounding_text_changed();
        assert_eq!(vec!["text 'second' 2 2"], fixture.take_calls());

        fixture.method.set_client(None);
        fixture.take_calls();
        second.raise_surrounding_text_changed();
        second.raise_input_pane_activation_requested();
        second.raise_selection_changed();
        assert!(fixture.take_calls().is_empty());
    }

    #[test]
    fn the_cursor_rectangle_moves_the_input_element_to_the_caret_of_the_client() {
        let fixture = Fixture::new();

        fixture.method.set_cursor_rect(Rect::new(10.9, 20.2, 1.0, 16.7));
        assert_eq!(vec!["focus input", "bounds 10 20 1 16 caret 0", "focus input"], fixture.take_calls());

        fixture.set_client(&TestClient::new("hello", 1, 3));
        fixture.take_calls();
        fixture.method.set_cursor_rect(Rect::new(10.9, 20.2, 1.0, 16.7));
        assert_eq!(vec!["focus input", "bounds 10 20 1 16 caret 3", "focus input"], fixture.take_calls());
    }

    #[test]
    fn input_modes_cover_all_native_content_types_without_unsupported_html_input_types() {
        use ferroui_base::input::text_input::TextInputContentType as C;
        for (content_type, input_mode) in [
            (C::Normal, "text"), (C::Alpha, "text"), (C::Digits, "numeric"),
            (C::Pin, "numeric"), (C::Number, "decimal"), (C::Email, "email"),
            (C::Url, "url"), (C::Name, "text"), (C::Password, "text"),
            (C::Social, "text"), (C::Search, "search"),
        ] {
            let options = TextInputOptions { content_type, ..Default::default() };
            let mapped = BrowserInputOptions::from_options(&options);
            assert_eq!(mapped.input_mode, input_mode, "{content_type:?}");
            assert_eq!(mapped.input_type, if matches!(content_type, C::Password | C::Pin) { "password" } else { "text" });
        }
    }

    #[test]
    fn every_native_return_label_maps_to_a_real_html_hint() {
        use ferroui_base::input::text_input::TextInputReturnKeyType as R;
        for (return_key_type, hint) in [
            (R::Default, ""), (R::Return, "enter"), (R::Done, "done"),
            (R::Go, "go"), (R::Send, "send"), (R::Search, "search"),
            (R::Next, "next"), (R::Previous, "previous"),
        ] {
            let options = TextInputOptions { return_key_type, ..Default::default() };
            assert_eq!(BrowserInputOptions::from_options(&options).enter_key_hint, hint);
        }
    }

    #[test]
    fn sensitive_text_password_and_pin_inputs_disable_capitalization_and_suggestions() {
        use ferroui_base::input::text_input::TextInputContentType as C;
        for (content_type, is_sensitive) in [(C::Normal, true), (C::Password, false), (C::Pin, false)] {
            let options = TextInputOptions { content_type, is_sensitive, uppercase: true,
                auto_capitalization: true, show_suggestions: Some(true), ..Default::default() };
            let mapped = BrowserInputOptions::from_options(&options);
            assert_eq!(mapped.input_type, if matches!(content_type, C::Password | C::Pin) { "password" } else { "text" });
            assert!(mapped.sensitive);
            assert_eq!(mapped.capitalization, "none");
            assert_eq!(mapped.suggestions, Some(false));
        }
    }

    #[test]
    fn nonprivate_options_preserve_casing_precedence_nullable_suggestions_and_first_locale() {
        use ferroui_base::input::text_input::TextInputContentType;
        let mut options = TextInputOptions { uppercase: true, lowercase: true,
            locale_hints: Some(Rc::from(["tr-TR".to_string(), "en-US".to_string()])), ..Default::default() };
        let mapped = BrowserInputOptions::from_options(&options);
        assert_eq!(mapped.capitalization, "characters");
        assert_eq!(mapped.language, "tr-TR");
        assert_eq!(mapped.suggestions, None);
        options.uppercase = false;
        assert_eq!(BrowserInputOptions::from_options(&options).capitalization, "none");
        options.lowercase = false;
        options.auto_capitalization = true;
        assert_eq!(BrowserInputOptions::from_options(&options).capitalization, "sentences");
        options.auto_capitalization = false;
        options.content_type = TextInputContentType::Name;
        assert_eq!(BrowserInputOptions::from_options(&options).capitalization, "words");
        for suggestions in [Some(false), Some(true)] {
            options.show_suggestions = suggestions;
            assert_eq!(BrowserInputOptions::from_options(&options).suggestions, suggestions);
        }
        options.locale_hints = None;
        assert_eq!(BrowserInputOptions::from_options(&options).language, "");
    }

    #[test]
    fn current_client_input_options_are_forwarded_to_the_page() {
        let fixture = Fixture::new();
        let options = TextInputOptions {
            is_sensitive: true,
            content_type: ferroui_base::input::text_input::TextInputContentType::Password,
            ..Default::default()
        };
        fixture.method.set_options(&options);
        assert_eq!(vec![format!("options {options:?}")], fixture.take_calls());
    }

    #[test]
    fn multiline_is_enabled_only_for_non_private_native_editors() {
        use ferroui_base::input::text_input::TextInputContentType as Content;
        for content in [Content::Normal, Content::Email, Content::Password, Content::Pin] {
            for sensitive in [false, true] {
                for multiline in [false, true] {
                    let options = TextInputOptions { content_type: content, is_sensitive: sensitive,
                        multiline, ..TextInputOptions::default_options() };
                    let actual = BrowserInputOptions::from_options(&options);
                    let private = matches!(content, Content::Password | Content::Pin);
                    assert_eq!(actual.multiline, multiline && !private);
                    assert_eq!(actual.input_type, if private { "password" } else { "text" });
                }
            }
        }
    }

    #[test]
    fn reset_empties_the_input_element() {
        let fixture = Fixture::new();

        fixture.method.reset();

        assert_eq!(vec!["clear", "text '' 0 0"], fixture.take_calls());
    }

    // A legacy client keeps its own DOM protocol while receiving native input
    // hints and selection notifications. It never uses the connection textarea.
    #[test]
    fn a_client_without_the_connection_receives_options_and_keeps_its_dom_protocol() {
        use ferroui_base::input::text_input::{TextInputContentType, TextInputReturnKeyType};
        let fixture = Fixture::new();
        let options = TextInputOptions {
            content_type: TextInputContentType::Digits,
            return_key_type: TextInputReturnKeyType::Search,
            show_suggestions: Some(true),
            ..Default::default()
        };

        fixture.method.set_options(&options);
        let client = TestClient::new("a\r\nb", 1, 3);
        fixture.set_client(&client);
        fixture.method.set_options(&options);
        client.set_selection(TextSelection::new(0, 0));
        fixture.method.on_composition_start();
        fixture.method.on_composition_update(Some("k"));
        fixture.method.on_composition_end(Some("k"));
        fixture.method.reset();
        fixture.method.set_client(None);

        assert_eq!(
            vec![format!("options {options:?}"), "clear".into(), "show".into(), "focus input".into(),
                "text 'a\r\nb' 1 3".into(), format!("options {options:?}"), "selection 0 0".into(),
                "clear".into(), "text '' 0 0".into(), "clear".into(), "hide".into(), "focus container".into()],
            fixture.take_calls()
        );
    }

    #[test]
    fn a_deletion_by_composition_selects_the_range_it_targets() {
        let fixture = Fixture::new();
        let client = TestClient::new("hello", 5, 5);
        fixture.set_client(&client);

        fixture.method.on_before_input("deleteByComposition", 2, 4);

        assert_eq!(TextSelection::new(2, 4), client.selection());
    }

    #[test]
    fn a_deletion_by_composition_without_a_range_leaves_the_selection() {
        let fixture = Fixture::new();
        let client = TestClient::new("hello", 5, 5);
        fixture.set_client(&client);

        fixture.method.on_before_input("deleteByComposition", -1, -1);
        fixture.method.on_before_input("deleteByComposition", 2, -1);

        assert_eq!(TextSelection::new(5, 5), client.selection());
    }

    #[test]
    fn a_backward_deletion_selects_the_selection_of_the_input_element() {
        let fixture = Fixture::new();
        let client = TestClient::new("hello", 5, 5);
        fixture.set_client(&client);
        fixture.selection.set((3, 4));

        fixture.method.on_before_input("deleteContentBackward", 0, 1);

        assert_eq!(TextSelection::new(3, 4), client.selection());
    }

    #[test]
    fn other_changes_of_the_input_element_leave_the_selection() {
        let fixture = Fixture::new();
        let client = TestClient::new("hello", 5, 5);
        fixture.set_client(&client);
        fixture.selection.set((3, 4));

        fixture.method.on_before_input("insertText", 0, 1);
        fixture.method.on_before_input("insertCompositionText", 2, 4);

        assert_eq!(TextSelection::new(5, 5), client.selection());
    }

    #[test]
    fn a_change_of_the_input_element_without_a_client_is_ignored() {
        let fixture = Fixture::new();

        fixture.method.on_before_input("deleteByComposition", 2, 4);
        fixture.method.on_before_input("deleteContentBackward", 0, 1);

        assert!(fixture.take_calls().is_empty());
    }

    #[test]
    fn a_composition_sets_the_preedit_text_and_clears_it_at_its_end() {
        let fixture = Fixture::new();
        let client = TestClient::new("", 0, 0);
        fixture.set_client(&client);
        assert!(!fixture.method.is_composing());

        fixture.method.on_composition_start();
        assert!(fixture.method.is_composing());

        fixture.method.on_composition_update(Some("k"));
        fixture.method.on_composition_update(Some("ka"));
        fixture.method.on_composition_update(None);
        assert!(fixture.method.is_composing());

        fixture.method.on_composition_end(Some("\u{304b}"));
        assert!(!fixture.method.is_composing());

        assert_eq!(
            vec![None, Some("k".to_string()), Some("ka".to_string()), None, None],
            *client.preedit.borrow()
        );
    }

    #[test]
    fn a_composition_without_a_client_changes_nothing() {
        let fixture = Fixture::new();

        fixture.method.on_composition_start();
        assert!(!fixture.method.is_composing());

        fixture.method.on_composition_update(Some("k"));
        fixture.method.on_composition_end(Some("k"));

        assert!(!fixture.method.is_composing());
        assert!(fixture.take_calls().is_empty());
    }

    #[test]
    fn losing_the_client_cancels_composition_before_its_delayed_end() {
        let fixture = Fixture::new();
        let client = TestClient::new("", 0, 0);
        fixture.set_client(&client);
        fixture.method.on_composition_start();
        fixture.method.on_composition_update(Some("ka"));

        fixture.method.set_client(None);
        fixture.method.on_composition_end(None);
        assert!(!fixture.method.is_composing());
        assert_eq!(vec![None, Some("ka".to_owned()), None], *client.preedit.borrow());

        fixture.set_client(&client);
        let before = client.preedit.borrow().clone();
        fixture.method.on_composition_end(None);
        assert!(!fixture.method.is_composing());
        assert_eq!(before, *client.preedit.borrow());
    }

    #[test]
    fn reset_cancels_preedit_and_ignores_the_late_composition_end() {
        let fixture = Fixture::new();
        let client = TestClient::new("first", 5, 5);
        fixture.set_client(&client);
        fixture.method.on_composition_start();
        fixture.method.on_composition_update(Some("ka"));
        fixture.method.reset();
        assert!(!fixture.method.is_composing());
        assert_eq!(vec![None, Some("ka".to_owned()), None], *client.preedit.borrow());
        fixture.take_calls();
        fixture.method.on_composition_update(Some("late"));
        fixture.method.on_composition_end(Some("late"));
        assert!(fixture.take_calls().is_empty());
        assert_eq!(vec![None, Some("ka".to_owned()), None], *client.preedit.borrow());
    }

    #[test]
    fn replacing_a_composing_client_never_sends_old_preedit_to_the_next_field() {
        let fixture = Fixture::new();
        let first = TestClient::new("first", 5, 5);
        let second = TestClient::new("second", 6, 6);
        fixture.set_client(&first);
        fixture.method.on_composition_start();
        fixture.method.on_composition_update(Some("ka"));
        fixture.set_client(&second);
        fixture.take_calls();
        assert!(!fixture.method.is_composing());
        fixture.method.on_composition_update(Some("late"));
        fixture.method.on_composition_end(Some("late"));
        assert!(second.preedit.borrow().is_empty());
        assert_eq!(vec![None, Some("ka".to_owned()), None], *first.preedit.borrow());
        assert!(fixture.take_calls().is_empty());
    }

    #[test]
    fn a_preedit_callback_that_detaches_during_start_leaves_no_composition() {
        let fixture = Fixture::new();
        let client = TestClient::new("first", 5, 5);
        fixture.set_client(&client);
        let method = fixture.method.clone();
        *client.preedit_hook.borrow_mut() = Some(Box::new(move || method.set_client(None)));
        fixture.method.on_composition_start();
        assert!(!fixture.method.is_composing());
        assert_eq!(vec![None, None], *client.preedit.borrow());
    }

    #[test]
    fn replacement_during_old_preedit_cleanup_wins_over_the_outer_attachment() {
        let fixture = Fixture::new();
        let first = TestClient::new("first", 5, 5);
        let second = TestClient::new("second", 6, 6);
        let third = TestClient::new("third", 5, 5);
        fixture.set_client(&first);
        fixture.method.on_composition_start();
        let method = fixture.method.clone();
        let next = third.clone();
        *first.preedit_hook.borrow_mut() = Some(Box::new(move || method.set_client(Some(next))));
        fixture.take_calls();
        fixture.set_client(&second);
        let current = fixture.method.client().unwrap();
        let third: Rc<dyn TextInputMethodClient> = third;
        assert!(Rc::ptr_eq(&current, &third));
        assert_eq!(vec!["clear", "clear", "show", "focus input", "text 'third' 5 5"], fixture.take_calls());
        assert_eq!(0, second.text_reads.get());
    }

    #[test]
    fn a_reentrant_text_snapshot_cannot_overwrite_the_replacement_field() {
        let fixture = Fixture::new();
        let first = TestClient::new("old private", 0, 0);
        let second = TestClient::new("public", 6, 6);
        fixture.set_client(&first);
        fixture.take_calls();
        let method = fixture.method.clone();
        *first.text_hook.borrow_mut() = Some(Box::new(move || method.set_client(Some(second))));
        first.raise_surrounding_text_changed();
        assert_eq!(vec!["clear", "show", "focus input", "text 'public' 6 6"], fixture.take_calls());
    }

    #[test]
    fn resetting_during_a_text_snapshot_does_not_restore_the_retired_value() {
        let fixture = Fixture::new();
        let client = TestClient::new("private", 0, 0);
        fixture.set_client(&client);
        fixture.take_calls();
        let method = fixture.method.clone();
        *client.text_hook.borrow_mut() = Some(Box::new(move || method.reset()));
        client.raise_surrounding_text_changed();
        assert_eq!(vec!["clear", "text '' 0 0"], fixture.take_calls());
    }

    #[test]
    fn disposal_clears_the_editor_preedit_and_client_without_focus_or_reactivation() {
        let fixture = Fixture::new();
        let client = TestClient::new("private", 7, 7);
        fixture.set_client(&client);
        fixture.method.on_composition_start();
        fixture.method.on_composition_update(Some("ka"));
        fixture.take_calls();
        fixture.method.dispose();
        assert_eq!(vec!["clear", "hide"], fixture.take_calls());
        assert!(!fixture.method.is_composing());
        assert!(fixture.method.client().is_none());
        assert!(fixture.method.subscriptions.borrow().is_empty());
        assert_eq!(vec![None, Some("ka".to_owned()), None], *client.preedit.borrow());
        fixture.method.dispose();
        fixture.set_client(&client);
        fixture.method.set_cursor_rect(Rect::default());
        fixture.method.set_options(&TextInputOptions::default_options());
        fixture.method.reset();
        fixture.method.on_before_input("deleteByComposition", 1, 2);
        fixture.method.on_composition_start();
        fixture.method.on_composition_update(Some("late"));
        fixture.method.on_composition_end(Some("late"));
        client.raise_selection_changed();
        client.raise_surrounding_text_changed();
        client.raise_input_pane_activation_requested();
        assert!(fixture.take_calls().is_empty());
        assert_eq!(TextSelection::new(7, 7), client.selection());
    }

    // A custom client exercises installation, explicit unsubscription and event
    // snapshots already being dispatched when a client is replaced.
    struct SubscriptionClient {
        inner: Rc<TestClient>,
        hook: RefCell<Option<Box<dyn FnOnce()>>>,
        active: Rc<Cell<usize>>,
        retired_handler: RefCell<Option<Rc<dyn Fn()>>>,
    }
    impl TextInputMethodClient for SubscriptionClient {
        fn events(&self) -> &TextInputMethodClientEvents { self.inner.events() }
        fn text_view_visual(&self) -> Option<Ref<Visual>> { None }
        fn supports_preedit(&self) -> bool { true }
        fn supports_surrounding_text(&self) -> bool { true }
        fn surrounding_text(&self) -> String { self.inner.surrounding_text() }
        fn cursor_rectangle(&self) -> Rect { Rect::default() }
        fn selection(&self) -> TextSelection { self.inner.selection() }
        fn set_selection(&self, value: TextSelection) { self.inner.set_selection(value); }
        fn surrounding_text_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
            self.retired_handler.replace(Some(handler.clone()));
            let handle = self.inner.surrounding_text_changed(handler);
            self.active.set(self.active.get() + 1);
            let hook = self.hook.take();
            if let Some(hook) = hook { hook(); }
            let active = self.active.clone();
            ferroui_base::reactive::Disposable::create(move || {
                handle.dispose();
                active.set(active.get() - 1);
            })
        }
    }

    fn subscription_client() -> Rc<SubscriptionClient> {
        Rc::new(SubscriptionClient { inner: TestClient::new("custom", 6, 6),
            hook: RefCell::new(None), active: Rc::new(Cell::new(0)),
            retired_handler: RefCell::new(None) })
    }

    #[test]
    fn an_aborted_subscription_installation_disposes_the_returned_handle() {
        let fixture = Fixture::new();
        let client = subscription_client();
        let replacement = TestClient::new("next", 4, 4);
        let method = fixture.method.clone();
        *client.hook.borrow_mut() = Some(Box::new(move || method.set_client(Some(replacement))));
        fixture.method.set_client(Some(client.clone()));
        assert_eq!(0, client.active.get());
        assert_eq!(vec!["clear", "clear", "show", "focus input", "text 'next' 4 4"], fixture.take_calls());
        client.retired_handler.borrow().as_ref().unwrap()();
        assert!(fixture.take_calls().is_empty());
    }

    #[test]
    fn drop_unsubscribes_even_when_the_custom_client_outlives_the_method() {
        let fixture = Fixture::new();
        let client = subscription_client();
        fixture.method.set_client(Some(client.clone()));
        assert_eq!(1, client.active.get());
        drop(fixture);
        assert_eq!(0, client.active.get());
    }

    #[test]
    fn a_retired_event_snapshot_cannot_read_or_reactivate_the_new_client() {
        let fixture = Fixture::new();
        let client = subscription_client();
        fixture.method.set_client(Some(client.clone()));
        let next = TestClient::new("public", 6, 6);
        fixture.set_client(&next);
        fixture.take_calls();
        let reads = next.text_reads.get();
        client.retired_handler.borrow().as_ref().unwrap()();
        assert_eq!(0, client.active.get());
        assert_eq!(reads, next.text_reads.get());
        assert!(fixture.take_calls().is_empty());
    }

    #[test]
    fn new_composition_and_selection_notifications_work_after_reset() {
        let fixture = Fixture::new();
        let client = TestClient::new("kana", 4, 4);
        fixture.set_client(&client);
        fixture.method.on_composition_start();
        fixture.method.reset();
        fixture.take_calls();
        fixture.method.on_composition_start();
        fixture.method.on_composition_update(Some("ka"));
        assert!(fixture.method.is_composing());
        fixture.method.on_composition_end(None);
        client.selection.set(TextSelection::new(2, 2));
        client.raise_selection_changed();
        assert!(!fixture.method.is_composing());
        assert_eq!(vec!["selection 4 4", "selection 2 2"], fixture.take_calls());
        assert_eq!(vec![None, None, None, Some("ka".to_owned()), None], *client.preedit.borrow());
    }
}

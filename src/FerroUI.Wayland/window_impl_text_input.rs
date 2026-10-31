//! The text input method of a window (the port of `WindowImpl.TextInput.cs`).
//!
//! UI-thread broker for text input. Combines two responsibilities so they
//! stay coherent on compositors with or without zwp_text_input_v3:
//! 1. Compose-key gating — flips the text input client flag of the surface on
//!    the Wayland thread so XKB-compose key sequences are processed if a text
//!    input client is attached.
//! 2. zwp_text_input_v3 IME — when the compositor exposes the manager,
//!    drives the worker-side facade (`WaylandTextInputV3`) and surfaces its
//!    callbacks back to the active text input client.
//!
//! All worker→UI callbacks (`IWaylandTextInputV3Events`) are marshalled by
//! the `WaylandTextInputV3EventsProxy` the broker registers with the surface,
//! so the methods below run on the UI thread despite being invoked by the
//! worker.

use crate::server::persistent::i_w_surface::IWSurface;
use crate::server::persistent::w_surface::WSurfaceId;
use crate::server::transient::i_wayland_text_input_v3_events::{IWaylandTextInputV3Events, WaylandTextInputV3EventsProxy};
use crate::window_impl::WindowImpl;
use crate::window_impl_base::get;
use ferroui_base::input::raw::{RawKeyEventArgs, RawKeyEventType, RawTextInputEventArgs};
use ferroui_base::input::text_input::{ITextInputMethodImpl, TextInputMethodClient, TextInputOptions, TextSelection};
use ferroui_base::input::{InputScopeNameValue, Key, KeyDeviceType, PhysicalKey, RawInputModifiers};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Rect;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

pub(crate) struct WaylandTextInputMethod {
    this: Weak<WaylandTextInputMethod>,
    owner: Weak<WindowImpl>,
    client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
    /// The subscriptions to the events of the client (the reference adds and removes
    /// handlers).
    client_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    has_preedit: Cell<bool>,
    /// The options that were set last. The framework sets the options of the focused element
    /// before it sets the client, and the worker forgets the state of a surface when its
    /// client changes: they are sent again after the client (the port's own; the reference
    /// loses them, DEVIATIONS.md).
    options: RefCell<Option<TextInputOptions>>,
    sink_proxy: RefCell<Option<WaylandTextInputV3EventsProxy>>,

    // Each callback the worker emits carries the session token most
    // recently set via set_text_input_active. set_client bumps client_epoch
    // and propagates it to the worker; worker→UI callbacks emitted under
    // a previous client carry the old token and are dropped by the guard
    // at the top of on_ime_update.
    client_epoch: Cell<i32>,

    // Tracks the surface we registered the sink against (the reference keeps the
    // proxy and compares references; a surface of the worker has a number). After
    // hide/show the surface is recreated; we need to re-register the
    // sink and re-apply the current text-input state.
    registered_against_surface: Cell<Option<WSurfaceId>>,
}

impl WaylandTextInputMethod {
    pub(crate) fn new(owner: Weak<WindowImpl>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            owner,
            client: RefCell::new(None),
            client_subscriptions: RefCell::new(Vec::new()),
            has_preedit: Cell::new(false),
            options: RefCell::new(None),
            sink_proxy: RefCell::new(None),
            client_epoch: Cell::new(0),
            registered_against_surface: Cell::new(None),
        })
    }

    fn current_client(&self) -> Option<Rc<dyn TextInputMethodClient>> {
        self.client.borrow().clone()
    }

    /// The first input scope of the owner of the client, for a client of the exact-edit
    /// connection; `None` for any other client, whose options are converted as upstream
    /// (not in the reference; `docs/porting/winui-rich-text.md`, decisions D5 and D8).
    fn connection_input_scope(&self) -> Option<InputScopeNameValue> {
        self.current_client()
            .and_then(|client| client.input_method_connection())
            .and_then(|connection| connection.effective_input_scope())
    }

    /// Called by the sink of the window after a new worker surface
    /// is created (Show following Hide). Re-registers the sink on the
    /// freshly created worker surface and re-applies the current
    /// text-input state so the IME keeps working across surface
    /// recreation.
    pub(crate) fn on_surface_created(&self) {
        if self.owner.upgrade().and_then(|owner| owner.surface_proxy()).is_none() {
            return;
        }
        // Force re-registration on the new surface even if we already
        // have a sink proxy — the worker-side sink lives on the surface and
        // is reset when the surface is recreated.
        self.registered_against_surface.set(None);

        let client = self.current_client();
        let has_client = client.is_some();
        let supports_preedit = client.as_ref().is_some_and(|client| client.supports_preedit());
        let supports_surrounding = client.as_ref().is_some_and(|client| client.supports_surrounding_text());
        self.apply_text_input_state_to_surface(has_client, supports_preedit, supports_surrounding);

        if has_client {
            self.push_surrounding();
        }
    }

    fn apply_text_input_state_to_surface(&self, has_client: bool, supports_preedit: bool, supports_surrounding: bool) {
        let Some(proxy) = self.owner.upgrade().and_then(|owner| owner.surface_proxy()) else {
            return;
        };

        if self.registered_against_surface.get() != Some(proxy.id()) {
            let sink_proxy = self
                .sink_proxy
                .borrow_mut()
                .get_or_insert_with(|| {
                    let this: Rc<dyn IWaylandTextInputV3Events> =
                        self.this.upgrade().expect("the text input method is alive while it is called");
                    WaylandTextInputV3EventsProxy::new(this, Dispatcher::ui_thread())
                })
                .clone();
            proxy.register_text_input_sink(sink_proxy);
            self.registered_against_surface.set(Some(proxy.id()));
        }

        proxy.set_text_input_active(has_client, supports_preedit, supports_surrounding, self.client_epoch.get());
        if has_client {
            if let Some(options) = &*self.options.borrow() {
                proxy.set_text_input_options_with_input_scope(options, self.connection_input_scope());
            }
        }
    }

    fn push_surrounding(&self) {
        let Some(client) = self.current_client().filter(|client| client.supports_surrounding_text()) else {
            return;
        };
        let text = client.surrounding_text();
        let selection = client.selection();
        if let Some(proxy) = self.owner.upgrade().and_then(|owner| owner.surface_proxy()) {
            proxy.set_text_input_surrounding_text(&text, selection.end, selection.start);
        }
    }

    fn unsubscribe_client(&self) {
        for subscription in self.client_subscriptions.borrow_mut().drain(..) {
            subscription.dispose();
        }
    }
}

impl ITextInputMethodImpl for WaylandTextInputMethod {
    fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
        let old_client = self.current_client();
        let same = match (&old_client, &client) {
            (Some(old), Some(new)) => Rc::ptr_eq(old, new),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }

        self.unsubscribe_client();

        // Bump epoch BEFORE swapping the client. The worker stamps every
        // subsequent IME callback with the token we send below; older
        // callbacks (for the previous client) carry the previous token
        // and are dropped at the top of on_ime_update.
        self.client_epoch.set(self.client_epoch.get().wrapping_add(1));

        // Abort any in-flight composition on the old client (if any).
        if let (Some(old_client), true) = (&old_client, self.has_preedit.get()) {
            old_client.set_preedit_text_with_cursor(None, None);
        }
        self.has_preedit.set(false);

        *self.client.borrow_mut() = client.clone();
        let has_client = client.is_some();
        let supports_preedit = client.as_ref().is_some_and(|client| client.supports_preedit());
        let supports_surrounding = client.as_ref().is_some_and(|client| client.supports_surrounding_text());

        self.apply_text_input_state_to_surface(has_client, supports_preedit, supports_surrounding);

        if let Some(client) = client {
            let this = self.this.clone();
            let on_changed: Rc<dyn Fn()> = Rc::new(move || {
                if let Some(this) = this.upgrade() {
                    this.push_surrounding();
                }
            });
            let mut subscriptions = self.client_subscriptions.borrow_mut();
            subscriptions.push(client.surrounding_text_changed(on_changed.clone()));
            subscriptions.push(client.selection_changed(on_changed));
            drop(subscriptions);
            self.push_surrounding();
        }
    }

    fn set_cursor_rect(&self, rect: Rect) {
        if let Some(proxy) = self.owner.upgrade().and_then(|owner| owner.surface_proxy()) {
            proxy.set_text_input_cursor_rect(rect);
        }
    }

    fn set_options(&self, options: &TextInputOptions) {
        *self.options.borrow_mut() = Some(options.clone());
        if let Some(proxy) = self.owner.upgrade().and_then(|owner| owner.surface_proxy()) {
            proxy.set_text_input_options_with_input_scope(options, self.connection_input_scope());
        }
    }

    fn reset(&self) {
        if let Some(proxy) = self.owner.upgrade().and_then(|owner| owner.surface_proxy()) {
            proxy.reset_text_input();
        }
    }
}

impl IWaylandTextInputV3Events for WaylandTextInputMethod {
    fn on_ime_update(
        &self,
        session_token: i32,
        delete_before_chars: i32,
        delete_after_chars: i32,
        commit_text: Option<String>,
        preedit_text: Option<String>,
        _preedit_cursor_begin_char: i32,
        preedit_cursor_end_char: i32,
    ) {
        // The session token rejects in-flight callbacks queued on the UI
        // dispatcher before set_client (or surface recreation) bumped the
        // epoch — those would otherwise apply to the wrong client.
        if session_token != self.client_epoch.get() {
            return;
        }
        let Some(client) = self.current_client() else {
            return;
        };
        let Some(owner) = self.owner.upgrade() else {
            return;
        };
        let Some(input_root) = owner.base().input_root() else {
            return;
        };

        let input = get(&owner.base().input);

        // Per spec, in this exact order:
        //   1. clear current preedit (implicit before commit/delete);
        //   2. apply delete_surrounding_text;
        //   3. insert commit text;
        //   4. apply new preedit.
        let has_delete = (delete_before_chars > 0 || delete_after_chars > 0) && client.supports_surrounding_text();
        let commit_text = commit_text.filter(|text| !text.is_empty());
        let has_commit = commit_text.is_some();
        let preedit_text = preedit_text.filter(|text| !text.is_empty());

        // Clear the existing preedit before we mutate the document with
        // delete or commit (it's about to be replaced anyway, and any
        // ongoing composition needs to be torn down first).
        if self.has_preedit.get() && (has_delete || has_commit) {
            client.set_preedit_text_with_cursor(None, None);
            self.has_preedit.set(false);
        }

        if let (true, Some(input)) = (has_delete, &input) {
            // Select the to-be-deleted range, then synthesize a Backspace
            // keypress. TextBox handles "Backspace with non-empty selection"
            // by deleting the selection — same pattern iOS uses for
            // deleting backward.
            let sel = client.selection();
            let anchor = sel.start.min(sel.end);
            let cursor = sel.start.max(sel.end);
            client.set_selection(TextSelection::new((anchor - delete_before_chars).max(0), cursor + delete_after_chars));

            for type_ in [RawKeyEventType::KeyDown, RawKeyEventType::KeyUp] {
                input(Rc::new(RawKeyEventArgs::new(
                    owner.base().keyboard(),
                    0,
                    input_root.clone(),
                    type_,
                    Key::Back,
                    RawInputModifiers::NONE,
                    PhysicalKey::Backspace,
                    Some("\u{8}".to_string()),
                    KeyDeviceType::Keyboard,
                )));
            }
        }

        if let (Some(commit_text), Some(input)) = (commit_text, &input) {
            input(Rc::new(RawTextInputEventArgs::new(owner.base().keyboard(), 0, input_root.clone(), commit_text)));
        }

        if let Some(preedit_text) = preedit_text {
            self.has_preedit.set(true);
            client.set_preedit_text_with_cursor(
                Some(&preedit_text),
                (preedit_cursor_end_char >= 0).then_some(preedit_cursor_end_char),
            );
        } else if self.has_preedit.get() {
            // No new preedit: clear any leftover composition.
            client.set_preedit_text_with_cursor(None, None);
            self.has_preedit.set(false);
        }
    }
}

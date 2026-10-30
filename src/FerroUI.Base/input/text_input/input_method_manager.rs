use super::{
    ITextInputMethodImpl, TextInputMethodClient, TextInputMethodClientRequestedEventArgs, TextInputMethodConnection,
    TextInputOptions, TransformTrackingHelper,
};
use crate::input::{InputElement, InputMethod};
use crate::interactivity::{Interactive, RoutedEventHandlerToken};
use crate::reactive::IDisposable;
use crate::{FerroProperty, Rect, Ref, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

fn same_client(a: &Rc<dyn TextInputMethodClient>, b: &Rc<dyn TextInputMethodClient>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

fn same_input_method(a: Option<&Rc<dyn ITextInputMethodImpl>>, b: Option<&Rc<dyn ITextInputMethodImpl>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
        (None, None) => true,
        _ => false,
    }
}

/// Whether a client opted in to the exact-edit connection: it implements the
/// atomic composition capability. Every other client keeps the upstream manager
/// behaviour (`docs/porting/winui-rich-text.md`, decision D5).
fn uses_connection(client: &dyn TextInputMethodClient) -> bool {
    client.input_method_editing().is_some()
}

fn is_input_options_property(property: &FerroProperty) -> bool {
    [
        TextInputOptions::content_type_property().as_property(),
        TextInputOptions::input_scope_names_property().as_property(),
        TextInputOptions::return_key_type_property().as_property(),
        TextInputOptions::multiline_property().as_property(),
        TextInputOptions::lowercase_property().as_property(),
        TextInputOptions::uppercase_property().as_property(),
        TextInputOptions::auto_capitalization_property().as_property(),
        TextInputOptions::is_sensitive_property().as_property(),
        TextInputOptions::show_suggestions_property().as_property(),
        TextInputOptions::locale_hints_property().as_property(),
    ]
    .iter()
    .any(|candidate| candidate.id() == property.id())
}

/// Connects the focused element to the input method of its root: finds
/// the text input method client of the element and keeps the input method
/// informed about the client, its options and the cursor rectangle.
///
/// A client that opts in to the exact-edit connection ([`uses_connection`])
/// also gets, as WinUI's text controls do (`docs/porting/DEVIATIONS.md`,
/// Input): a [`TextInputMethodConnection`] to its semantic owner
/// ([`TextInputMethodClient::input_method_owner`]), which may differ from the
/// focused element; the options of that owner, published again when one of
/// them changes; a new connection and a fresh query when it asks for a reset;
/// and no binding while the owner is detached or disabled. A legacy client
/// gets the upstream behaviour unchanged.
pub(crate) struct TextInputMethodManager {
    this: Weak<TextInputMethodManager>,
    im: RefCell<Option<Rc<dyn ITextInputMethodImpl>>>,
    focused_element: RefCell<Option<Ref<InputElement>>>,
    /// The element that supplies the options and the cursor transform: the
    /// focused element, or the owner a connection client names.
    owner: RefCell<Option<WeakRef<InputElement>>>,
    visual_root: RefCell<Option<(Ref<Interactive>, RoutedEventHandlerToken)>>,
    client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
    connection: RefCell<Option<Rc<TextInputMethodConnection>>>,
    /// Advanced on every change of the client or its connection, so that the
    /// option publication of a connection client never runs for a stale one.
    generation: Cell<u64>,
    client_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    transform_tracker: Rc<TransformTrackingHelper>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    disposed: Cell<bool>,
}

impl TextInputMethodManager {
    pub(crate) fn new() -> Rc<TextInputMethodManager> {
        let manager = Rc::new_cyclic(|this: &Weak<TextInputMethodManager>| TextInputMethodManager {
            this: this.clone(),
            im: RefCell::new(None),
            focused_element: RefCell::new(None),
            owner: RefCell::new(None),
            visual_root: RefCell::new(None),
            client: RefCell::new(None),
            connection: RefCell::new(None),
            generation: Cell::new(0),
            client_subscriptions: RefCell::new(Vec::new()),
            transform_tracker: TransformTrackingHelper::new(true),
            subscriptions: RefCell::new(Vec::new()),
            disposed: Cell::new(false),
        });

        let matrix_changed = {
            let weak = manager.this.clone();
            manager.transform_tracker.matrix_changed(move || {
                if let Some(this) = weak.upgrade() {
                    this.update_cursor_rect();
                }
            })
        };

        let is_enabled_changed = {
            let weak = manager.this.clone();
            InputMethod::is_input_method_enabled_property().changed().subscribe(move |e| {
                if let Some(this) = weak.upgrade() {
                    this.on_is_input_method_enabled_changed(e.sender());
                }
            })
        };

        manager.subscriptions.borrow_mut().extend([matrix_changed, is_enabled_changed]);
        manager
    }

    fn im(&self) -> Option<Rc<dyn ITextInputMethodImpl>> {
        self.im.borrow().clone()
    }

    fn client(&self) -> Option<Rc<dyn TextInputMethodClient>> {
        self.client.borrow().clone()
    }

    fn focused_element(&self) -> Option<Ref<InputElement>> {
        self.focused_element.borrow().clone()
    }

    /// The owner of the current client: the element its options and cursor
    /// transform come from. Without a client it is the focused element.
    fn owner(&self) -> Option<Ref<InputElement>> {
        match self.owner.borrow().as_ref() {
            Some(owner) => owner.upgrade(),
            None => self.focused_element(),
        }
    }

    fn is_current(&self, generation: u64) -> bool {
        !self.disposed.get() && self.generation.get() == generation
    }

    fn advance_generation(&self) -> u64 {
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        generation
    }

    fn is_current_client(&self, client: &Weak<dyn TextInputMethodClient>) -> bool {
        match (self.client(), client.upgrade()) {
            (Some(current), Some(client)) => same_client(&current, &client),
            _ => false,
        }
    }

    fn retire_connection(&self) {
        let connection = self.connection.take();
        if let Some(connection) = connection {
            connection.invalidate();
        }
        if let Some(client) = self.client() {
            client.events().set_connection(None);
        }
    }

    fn install_connection(&self) {
        let (Some(owner), Some(client)) = (self.owner(), self.client()) else {
            return;
        };
        if !uses_connection(&*client) {
            return;
        }
        let connection = TextInputMethodConnection::new(&owner, &client);
        client.events().set_connection(Some(&connection));
        *self.connection.borrow_mut() = Some(connection);
    }

    fn set_client(&self, value: Option<Rc<dyn TextInputMethodClient>>, owner: Option<Ref<InputElement>>) {
        let current = self.client();
        let unchanged = match (&current, &value) {
            (Some(current), Some(value)) => same_client(current, value),
            (None, None) => true,
            _ => false,
        };
        // Only a connection client has an owner of its own; a legacy client
        // follows the focused element, as upstream.
        let owner = owner.filter(|_| value.as_ref().is_some_and(|value| uses_connection(&**value)));
        let owner_unchanged = self.owner.borrow().as_ref().and_then(WeakRef::upgrade) == owner;
        if unchanged && owner_unchanged {
            return;
        }

        self.advance_generation();

        if current.is_some() {
            self.retire_connection();

            let subscriptions = std::mem::take(&mut *self.client_subscriptions.borrow_mut());
            for subscription in subscriptions {
                subscription.dispose();
            }

            drop(self.client.replace(None));
            drop(self.owner.replace(None));

            if let Some(im) = self.im() {
                im.reset();
            }
        }

        drop(self.client.replace(value.clone()));
        drop(self.owner.replace(owner.as_ref().map(Ref::downgrade)));

        if let Some(client) = value {
            let sender = Rc::downgrade(&client);

            let cursor_rectangle_changed = {
                let (weak, sender) = (self.this.clone(), sender.clone());
                client.cursor_rectangle_changed(Rc::new(move || {
                    if let Some(this) = weak.upgrade() {
                        this.on_cursor_rectangle_changed(&sender);
                    }
                }))
            };
            let text_view_visual_changed = {
                let weak = self.this.clone();
                client.text_view_visual_changed(Rc::new(move || {
                    if let Some(this) = weak.upgrade() {
                        this.on_text_view_visual_changed();
                    }
                }))
            };
            let reset_requested = {
                let (weak, sender) = (self.this.clone(), sender.clone());
                client.reset_requested(Rc::new(move || {
                    if let Some(this) = weak.upgrade() {
                        this.on_reset_requested(&sender);
                    }
                }))
            };

            self.client_subscriptions.borrow_mut().extend([
                cursor_rectangle_changed,
                text_view_visual_changed,
                reset_requested,
            ]);

            if uses_connection(&*client) {
                if let Some(owner) = &owner {
                    let weak = self.this.clone();
                    let weak_owner = owner.downgrade();
                    let options_changed = owner.property_changed(move |event| {
                        if !event.is_effective_value_change() || !is_input_options_property(event.property()) {
                            return;
                        }
                        if let Some(this) = weak.upgrade() {
                            if this.owner() == weak_owner.upgrade() && this.is_current_client(&sender) {
                                this.update_input_options(this.generation.get());
                            }
                        }
                    });
                    self.client_subscriptions.borrow_mut().push(options_changed);
                }
                self.install_connection();
            }

            self.populate_im_with_initial_values();
        } else {
            if let Some(im) = self.im() {
                im.set_client(None);
            }
            self.transform_tracker.set_visual(None);
        }
    }

    fn populate_im_with_initial_values(&self) {
        if self.client().is_some_and(|client| uses_connection(&*client)) {
            self.populate_connection_client(self.generation.get());
            return;
        }
        let im = self.im();

        if let Some(im) = &im {
            match self.owner() {
                Some(target) => im.set_options(&TextInputOptions::from_styled_element(&target)),
                None => im.set_options(&TextInputOptions::default_options()),
            }
        }

        let client = self.client();
        let text_view_visual = client.as_ref().and_then(|client| client.text_view_visual());
        self.transform_tracker.set_visual(text_view_visual.as_ref());

        if let Some(im) = &im {
            im.set_client(client);
        }

        self.update_cursor_rect();
    }

    /// `populate_im_with_initial_values` for a connection client: a backend call may
    /// run application code that binds another client, after which the remaining
    /// steps of this binding are not made.
    fn populate_connection_client(&self, generation: u64) {
        let Some(im) = self.im() else {
            return;
        };
        self.update_input_options(generation);
        if !self.is_current(generation) {
            return;
        }
        let client = self.client();
        let text_view_visual = client.as_ref().and_then(|client| client.text_view_visual());
        self.transform_tracker.set_visual(text_view_visual.as_ref());
        if !self.is_current(generation) {
            return;
        }
        im.set_client(client);
        if self.is_current(generation) {
            self.update_cursor_rect();
        }
    }

    /// Publishes the options of the owner of a connection client again, after
    /// one of them changed. A backend may run application code from
    /// `set_options`; when that code changes the options once more, the latest
    /// ones are published after the older call returns, and nothing is
    /// published once the client or its connection has been replaced.
    fn update_input_options(&self, mut generation: u64) {
        loop {
            if !self.is_current(generation) {
                return;
            }
            let (Some(im), Some(owner)) = (self.im(), self.owner()) else {
                return;
            };
            let options = TextInputOptions::from_styled_element(&owner);
            let scopes = TextInputOptions::get_input_scope_names(&owner);
            if !self.is_current(generation) {
                return;
            }
            im.set_options(&options);

            // A backend may finish applying the old value after a callback in it
            // published newer options or bound another client. Publish the current
            // owner's values once more after that older call returns.
            generation = self.generation.get();
            if !self.is_current(generation)
                || self.client().is_none()
                || !same_input_method(Some(&im), self.im().as_ref())
            {
                return;
            }
            let Some(owner) = self.owner() else {
                return;
            };
            if TextInputOptions::from_styled_element(&owner) == options
                && TextInputOptions::get_input_scope_names(&owner) == scopes
            {
                return;
            }
        }
    }

    fn on_reset_requested(&self, sender: &Weak<dyn TextInputMethodClient>) {
        let Some(im) = self.im() else {
            return;
        };
        if !self.is_current_client(sender) {
            return;
        }
        let Some(client) = sender.upgrade() else {
            return;
        };
        if !uses_connection(&*client) {
            im.reset();
            self.populate_im_with_initial_values();
            return;
        }

        // A connection client is queried again first: a reset may follow a
        // change of the editing UI that hands the input to another client.
        self.try_find_and_apply_client();
        if !self.is_current_client(sender) {
            return;
        }
        let generation = self.advance_generation();
        self.retire_connection();
        if let Some(im) = self.im() {
            im.reset();
        }
        if !self.is_current(generation) {
            return;
        }
        self.install_connection();
        self.populate_im_with_initial_values();
    }

    fn on_is_input_method_enabled_changed(&self, sender: &crate::FerroObject) {
        let is_tracked = [self.focused_element(), self.owner()].into_iter().flatten().any(|element| {
            let element: &crate::FerroObject = &element;
            std::ptr::eq(element, sender)
        });

        if is_tracked {
            self.try_find_and_apply_client();
        }
    }

    fn on_text_view_visual_changed(&self) {
        let text_view_visual = self.client().and_then(|client| client.text_view_visual());
        self.transform_tracker.set_visual(text_view_visual.as_ref());
    }

    fn update_cursor_rect(&self) {
        let (Some(im), Some(client), Some(v)) = (self.im(), self.client(), self.owner()) else {
            return;
        };
        let Some(root) = v.visual_root() else {
            return;
        };

        match v.transform_to_visual(&root) {
            None => im.set_cursor_rect(Rect::default()),
            Some(transform) => im.set_cursor_rect(client.cursor_rectangle().transform_to_aabb(transform)),
        }
    }

    fn on_cursor_rectangle_changed(&self, sender: &Weak<dyn TextInputMethodClient>) {
        if self.is_current_client(sender) {
            self.update_cursor_rect();
        }
    }

    /// Tells the manager which element has keyboard focus.
    pub(crate) fn set_focused_element(&self, element: Option<Ref<InputElement>>) {
        if *self.focused_element.borrow() == element {
            return;
        }

        if let Some((visual_root, token)) = self.visual_root.borrow_mut().take() {
            InputMethod::remove_text_input_method_client_requery_requested_handler(&visual_root, token);
        }

        drop(self.focused_element.replace(element.clone()));

        let visual_root =
            element.as_ref().and_then(|element| element.visual_root()).and_then(|root| root.downcast::<Interactive>().ok());

        if let Some(visual_root) = visual_root {
            let weak = self.this.clone();
            let token = InputMethod::add_text_input_method_client_requery_requested_handler(&visual_root, move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.text_input_method_client_requery_requested();
                }
            });
            *self.visual_root.borrow_mut() = Some((visual_root, token));
        }

        // Focus inside the editing UI of a connection client (its flyout, possibly in a
        // popup root without an input method) keeps the input method of the owner's
        // root; try_find_and_apply_client then keeps the binding. A legacy client
        // follows the focused root, as upstream.
        if !element.as_ref().is_some_and(|element| self.connection_client_retains(element)) {
            let input_method =
                element.as_ref().and_then(|element| element.get_input_root()).and_then(|root| root.input_method());

            self.replace_input_method(input_method);
        }

        self.try_find_and_apply_client();
    }

    /// Whether the bound connection client keeps the input method while `focused`
    /// has keyboard focus.
    fn connection_client_retains(&self, focused: &InputElement) -> bool {
        let (Some(client), Some(_)) = (self.client(), self.owner()) else {
            return false;
        };
        uses_connection(&*client) && self.im().is_some() && client.retains_input_method_focus(focused)
    }

    fn replace_input_method(&self, input_method: Option<Rc<dyn ITextInputMethodImpl>>) {
        let im = self.im();
        if !same_input_method(im.as_ref(), input_method.as_ref()) {
            if let Some(im) = &im {
                im.set_client(None);
            }
        }

        drop(self.im.replace(input_method));
    }

    fn text_input_method_client_requery_requested(&self) {
        if self.im.borrow().is_some() {
            self.try_find_and_apply_client();
        }
    }

    fn try_find_and_apply_client(&self) {
        if self.disposed.get() {
            return;
        }
        let focused = self.focused_element();
        let Some(focused) = focused.filter(|focused| {
            self.im.borrow().is_some() && InputMethod::get_is_input_method_enabled(focused)
        }) else {
            self.set_client(None, None);
            return;
        };

        let client_query = TextInputMethodClientRequestedEventArgs::new();
        client_query.set_routed_event(Some(InputElement::text_input_method_client_requested_event()));

        focused.raise_event(&client_query);

        let mut client = client_query.client();
        let mut owner = client.as_ref().map(|client| client.input_method_owner().unwrap_or_else(|| focused.clone()));

        // Focus inside the active editing UI of the current client (its context
        // menu, a flyout) keeps that client; only a client that says so does.
        if client.is_none() {
            if let (Some(current), Some(current_owner)) = (self.client(), self.owner()) {
                if current.retains_input_method_focus(&focused) {
                    client = Some(current);
                    owner = Some(current_owner);
                }
            }
        }

        if let (Some(candidate), Some(candidate_owner)) = (&client, &owner) {
            if uses_connection(&**candidate) {
                // A connection client is bound only while its owner can take
                // input, and through the input method of the owner's root.
                let available = candidate_owner.is_attached_to_visual_tree()
                    && candidate_owner.is_effectively_enabled()
                    && InputMethod::get_is_input_method_enabled(candidate_owner);
                let input_method = candidate_owner.get_input_root().and_then(|root| root.input_method());
                match input_method.filter(|_| available) {
                    Some(input_method) => {
                        if !same_input_method(self.im().as_ref(), Some(&input_method)) {
                            self.set_client(None, None);
                            self.replace_input_method(Some(input_method));
                        }
                    }
                    None => {
                        client = None;
                        owner = None;
                    }
                }
            }
        }

        self.set_client(client, owner);
    }

    /// Releases the subscriptions of the manager.
    pub(crate) fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }

        self.retire_connection();
        let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
        let subscriptions = std::mem::take(&mut *self.client_subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
        self.transform_tracker.dispose();
    }
}

impl Drop for TextInputMethodManager {
    fn drop(&mut self) {
        self.dispose();
    }
}

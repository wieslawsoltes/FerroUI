use super::{
    ITextInputMethodImpl, TextInputMethodClient, TextInputMethodClientRequestedEventArgs,
    TextInputMethodConnection, TextInputOptions, TransformTrackingHelper,
};
use crate::input::{InputElement, InputMethod};
use crate::interactivity::{Interactive, RoutedEventHandlerToken};
use crate::reactive::IDisposable;
use crate::{FerroProperty, Rect, Ref, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

fn same_client(
    a: Option<&Rc<dyn TextInputMethodClient>>,
    b: Option<&Rc<dyn TextInputMethodClient>>,
) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
        (None, None) => true,
        _ => false,
    }
}

fn same_input_method(
    a: Option<&Rc<dyn ITextInputMethodImpl>>,
    b: Option<&Rc<dyn ITextInputMethodImpl>>,
) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
        (None, None) => true,
        _ => false,
    }
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

/// Connects a semantic text owner to its root's input method. Keyboard focus may
/// enter the owner's active editing UI without replacing that connection. Owner,
/// client, and transport changes retire captured platform sessions first.
pub(crate) struct TextInputMethodManager {
    this: Weak<TextInputMethodManager>,
    im: RefCell<Option<Rc<dyn ITextInputMethodImpl>>>,
    focused_element: RefCell<Option<Ref<InputElement>>>,
    owner: RefCell<Option<WeakRef<InputElement>>>,
    visual_root: RefCell<Option<(Ref<Interactive>, RoutedEventHandlerToken)>>,
    client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
    connection: RefCell<Option<Rc<TextInputMethodConnection>>>,
    generation: Cell<u64>,
    focus_generation: Cell<u64>,
    cursor_generation: Cell<u64>,
    client_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    transform_tracker: Rc<TransformTrackingHelper>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    disposed: Cell<bool>,
}

impl TextInputMethodManager {
    pub(crate) fn new() -> Rc<Self> {
        let manager = Rc::new_cyclic(|this| Self {
            this: this.clone(),
            im: RefCell::new(None),
            focused_element: RefCell::new(None),
            owner: RefCell::new(None),
            visual_root: RefCell::new(None),
            client: RefCell::new(None),
            connection: RefCell::new(None),
            generation: Cell::new(0),
            focus_generation: Cell::new(0),
            cursor_generation: Cell::new(0),
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
            InputMethod::is_input_method_enabled_property()
                .changed()
                .subscribe(move |e| {
                    if let Some(this) = weak.upgrade() {
                        this.on_is_input_method_enabled_changed(e.sender());
                    }
                })
        };
        manager
            .subscriptions
            .borrow_mut()
            .extend([matrix_changed, is_enabled_changed]);
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
    fn owner(&self) -> Option<Ref<InputElement>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    fn current(&self, generation: u64) -> bool {
        !self.disposed.get() && self.generation.get() == generation
    }

    fn is_current_client(&self, client: &Weak<dyn TextInputMethodClient>) -> bool {
        client
            .upgrade()
            .is_some_and(|client| same_client(self.client().as_ref(), Some(&client)))
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
        if let (Some(owner), Some(client)) = (self.owner(), self.client()) {
            let connection = TextInputMethodConnection::new(&owner, &client);
            client.events().set_connection(Some(&connection));
            *self.connection.borrow_mut() = Some(connection);
        }
    }

    fn set_binding(
        &self,
        owner: Option<Ref<InputElement>>,
        client: Option<Rc<dyn TextInputMethodClient>>,
        im: Option<Rc<dyn ITextInputMethodImpl>>,
    ) {
        if self.disposed.get() {
            return;
        }
        let previous_im = self.im();
        let same_im = same_input_method(previous_im.as_ref(), im.as_ref());
        if self.owner() == owner && same_client(self.client().as_ref(), client.as_ref()) && same_im
        {
            return;
        }
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        self.retire_connection();
        let previous_client = self.client.take();
        self.owner.take();
        self.im.take();
        let subscriptions = std::mem::take(&mut *self.client_subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
        self.transform_tracker.set_visual(None);
        if !self.current(generation) {
            return;
        }
        if let Some(previous_im) = previous_im {
            if !same_im {
                previous_im.set_client(None);
            } else if previous_client.is_some() {
                previous_im.reset();
            }
        }
        if !self.current(generation) {
            return;
        }
        *self.im.borrow_mut() = im;
        *self.owner.borrow_mut() = owner.as_ref().map(Ref::downgrade);
        *self.client.borrow_mut() = client.clone();
        self.install_connection();
        if let Some(client) = client {
            let sender = Rc::downgrade(&client);
            let cursor = {
                let (weak, sender) = (self.this.clone(), sender.clone());
                client.cursor_rectangle_changed(Rc::new(move || {
                    if let Some(this) = weak.upgrade() {
                        if this.is_current_client(&sender) {
                            this.update_cursor_rect();
                        }
                    }
                }))
            };
            let view = {
                let (weak, sender) = (self.this.clone(), sender.clone());
                client.text_view_visual_changed(Rc::new(move || {
                    if let Some(this) = weak.upgrade() {
                        if this.is_current_client(&sender) {
                            this.on_text_view_visual_changed();
                        }
                    }
                }))
            };
            let reset = {
                let (weak, sender) = (self.this.clone(), sender.clone());
                client.reset_requested(Rc::new(move || {
                    if let Some(this) = weak.upgrade() {
                        this.on_reset_requested(&sender);
                    }
                }))
            };
            let mut subscriptions = vec![cursor, view, reset];
            if let Some(owner) = owner {
                let weak = self.this.clone();
                let weak_owner = owner.downgrade();
                subscriptions.push(owner.property_changed(move |event| {
                    if event.is_effective_value_change()
                        && is_input_options_property(event.property())
                    {
                        if let Some(this) = weak.upgrade() {
                            // Reset retires a connection without reinstalling these
                            // subscriptions. Check semantic identity here, then fence
                            // this publication with the current binding generation.
                            if this.owner() == weak_owner.upgrade()
                                && this.is_current_client(&sender)
                            {
                                this.update_input_options(this.generation.get());
                            }
                        }
                    }
                }));
            }
            if !self.current(generation) {
                for subscription in subscriptions {
                    subscription.dispose();
                }
                return;
            }
            self.client_subscriptions.borrow_mut().extend(subscriptions);
            self.populate_im_with_initial_values(generation);
        } else if same_im {
            if let Some(im) = self.im() {
                im.set_client(None);
            }
        }
    }

    fn populate_im_with_initial_values(&self, generation: u64) {
        let Some(im) = self.im() else {
            return;
        };
        self.update_input_options(generation);
        if !self.current(generation) {
            return;
        }
        let client = self.client();
        let visual = client.as_ref().and_then(|client| client.text_view_visual());
        self.transform_tracker.set_visual(visual.as_ref());
        if !self.current(generation) {
            return;
        }
        im.set_client(client);
        if self.current(generation) {
            self.update_cursor_rect();
        }
    }

    fn input_options(&self) -> TextInputOptions {
        self.owner()
            .map_or_else(TextInputOptions::default_options, |owner| {
                TextInputOptions::from_styled_element(&owner)
            })
    }

    fn update_input_options(&self, mut generation: u64) {
        loop {
            if !self.current(generation) {
                return;
            }
            let Some(im) = self.im() else {
                return;
            };
            let options = self.input_options();
            // Property getters and the backend can call application code. Never
            // retain manager borrows or install a captured client after that code
            // changes the binding. Option changes alone do not reset composition.
            if !self.current(generation) {
                return;
            }
            im.set_options(&options);

            generation = self.generation.get();
            if !self.current(generation)
                || self.client().is_none()
                || !same_input_method(Some(&im), self.im().as_ref())
            {
                return;
            }
            let latest = self.input_options();
            if self.current(generation) && latest == options {
                return;
            }
            // A native setter may finish applying the old value after a
            // callback updates options or replaces its client. Reapply the
            // current owner's values only after that older setter returns.
            generation = self.generation.get();
        }
    }

    fn on_reset_requested(&self, sender: &Weak<dyn TextInputMethodClient>) {
        if !self.is_current_client(sender) {
            return;
        }
        self.try_find_and_apply_client();
        if !self.is_current_client(sender) {
            return;
        }
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        self.retire_connection();
        if let Some(im) = self.im() {
            im.reset();
        }
        if !self.current(generation) {
            return;
        }
        self.install_connection();
        self.populate_im_with_initial_values(generation);
    }

    fn on_is_input_method_enabled_changed(&self, sender: &crate::FerroObject) {
        if [self.owner(), self.focused_element()]
            .into_iter()
            .flatten()
            .any(|element| {
                let element: &crate::FerroObject = &element;
                std::ptr::eq(element, sender)
            })
        {
            self.try_find_and_apply_client();
        }
    }

    fn on_text_view_visual_changed(&self) {
        self.try_find_and_apply_client();
        let generation = self.generation.get();
        let focus_generation = self.focus_generation.get();
        let client = self.client();
        let visual = client.as_ref().and_then(|client| client.text_view_visual());
        if self.current(generation)
            && self.focus_generation.get() == focus_generation
            && same_client(self.client().as_ref(), client.as_ref())
        {
            self.transform_tracker.set_visual(visual.as_ref());
        }
    }

    fn update_cursor_rect(&self) {
        let generation = self.generation.get();
        if !self.current(generation) {
            return;
        }
        let cursor_generation = self.cursor_generation.get().wrapping_add(1);
        self.cursor_generation.set(cursor_generation);
        let (Some(im), Some(client), Some(owner)) = (self.im(), self.client(), self.owner()) else {
            return;
        };
        let current = || {
            self.current(generation)
                && self.cursor_generation.get() == cursor_generation
                && self.owner().as_ref() == Some(&owner)
                && same_client(self.client().as_ref(), Some(&client))
                && same_input_method(self.im().as_ref(), Some(&im))
        };
        let Some(root) = owner.visual_root() else {
            self.try_find_and_apply_client();
            return;
        };
        if !current() {
            return;
        }
        // Resolving a caret can invoke layout/application callbacks. They may
        // install another owner or publish newer geometry for this same client.
        // Capture the accepted owner transform only after that query returns.
        let local = client.cursor_rectangle();
        if !current() || owner.visual_root().as_ref() != Some(&root) {
            return;
        }
        let rect = owner
            .transform_to_visual(&root)
            .map_or_else(Rect::default, |transform| {
                local.transform_to_aabb(transform)
            });
        if current() && owner.visual_root().as_ref() == Some(&root) {
            im.set_cursor_rect(rect);
        }
    }

    /// Records the keyboard's accepted destination before LostFocus/GotFocus.
    /// A view change in those callbacks must not requery the previous popup
    /// child. Resolving the binding normally remains a post-event operation.
    pub(crate) fn prepare_focus_change(&self, element: Option<Ref<InputElement>>) {
        if self.disposed.get() || *self.focused_element.borrow() == element {
            return;
        }
        self.focus_generation
            .set(self.focus_generation.get().wrapping_add(1));
        let previous = self.visual_root.take();
        if let Some((root, token)) = previous {
            InputMethod::remove_text_input_method_client_requery_requested_handler(&root, token);
        }
        *self.focused_element.borrow_mut() = element.clone();
        let root = element
            .and_then(|element| element.visual_root())
            .and_then(|root| root.downcast::<Interactive>().ok());
        if let Some(root) = root {
            let weak = self.this.clone();
            let token = InputMethod::add_text_input_method_client_requery_requested_handler(
                &root,
                move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        this.try_find_and_apply_client();
                    }
                },
            );
            *self.visual_root.borrow_mut() = Some((root, token));
        }
    }

    pub(crate) fn set_focused_element(&self, element: Option<Ref<InputElement>>) {
        self.prepare_focus_change(element);
        self.try_find_and_apply_client();
    }

    fn try_find_and_apply_client(&self) {
        if self.disposed.get() {
            return;
        }
        let generation = self.generation.get();
        let focus_generation = self.focus_generation.get();
        let current =
            || self.current(generation) && self.focus_generation.get() == focus_generation;
        let Some(focused) = self.focused_element() else {
            self.set_binding(None, None, None);
            return;
        };
        if !InputMethod::get_is_input_method_enabled(&focused) {
            let im = focused
                .get_input_root()
                .and_then(|root| root.input_method());
            if current() {
                self.set_binding(None, None, im);
            }
            return;
        }
        let query = TextInputMethodClientRequestedEventArgs::new();
        query.set_routed_event(Some(
            InputElement::text_input_method_client_requested_event(),
        ));
        focused.raise_event(&query);
        if !current() {
            return;
        }
        let mut client = query.client();
        let mut owner = client.as_ref().map(|client| {
            client
                .input_method_owner()
                .unwrap_or_else(|| focused.clone())
        });
        if !current() {
            return;
        }
        if client.is_none() {
            if let (Some(current), Some(current_owner)) = (self.client(), self.owner()) {
                if current.retains_input_method_focus(&focused) {
                    client = Some(current);
                    owner = Some(current_owner);
                }
            }
        }
        if !current() {
            return;
        }
        let owner = owner.filter(|owner| {
            owner.is_attached_to_visual_tree()
                && owner.is_effectively_enabled()
                && InputMethod::get_is_input_method_enabled(owner)
        });
        let im = owner
            .as_ref()
            .and_then(|owner| owner.get_input_root())
            .and_then(|root| root.input_method());
        if !current() {
            return;
        }
        if owner.is_some() && im.is_some() {
            self.set_binding(owner, client, im);
        } else {
            // Retain a focused root without a client to preserve the normal
            // reset/detach ordering when that root later offers an editor.
            let im = focused
                .get_input_root()
                .and_then(|root| root.input_method());
            if current() {
                self.set_binding(None, None, im);
            }
        }
    }

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
        let root = self.visual_root.take();
        if let Some((root, token)) = root {
            InputMethod::remove_text_input_method_client_requery_requested_handler(&root, token);
        }
        self.transform_tracker.dispose();
        self.client.take();
        self.owner.take();
        let im = self.im.take();
        if let Some(im) = im {
            im.set_client(None);
        }
    }
}

impl Drop for TextInputMethodManager {
    fn drop(&mut self) {
        self.dispose();
    }
}

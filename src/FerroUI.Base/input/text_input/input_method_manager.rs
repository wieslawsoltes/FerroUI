use super::{
    ITextInputMethodImpl, TextInputMethodClient, TextInputMethodClientRequestedEventArgs, TextInputOptions,
    TransformTrackingHelper,
};
use crate::input::{InputElement, InputMethod};
use crate::interactivity::{Interactive, RoutedEventHandlerToken};
use crate::reactive::IDisposable;
use crate::{Rect, Ref};
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

/// Connects the focused element to the input method of its root: finds
/// the text input method client of the element and keeps the input method
/// informed about the client, its options and the cursor rectangle.
pub(crate) struct TextInputMethodManager {
    this: Weak<TextInputMethodManager>,
    im: RefCell<Option<Rc<dyn ITextInputMethodImpl>>>,
    focused_element: RefCell<Option<Ref<InputElement>>>,
    visual_root: RefCell<Option<(Ref<Interactive>, RoutedEventHandlerToken)>>,
    client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
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
            visual_root: RefCell::new(None),
            client: RefCell::new(None),
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

    fn is_current_client(&self, client: &Weak<dyn TextInputMethodClient>) -> bool {
        match (self.client(), client.upgrade()) {
            (Some(current), Some(client)) => same_client(&current, &client),
            _ => false,
        }
    }

    fn set_client(&self, value: Option<Rc<dyn TextInputMethodClient>>) {
        let current = self.client();
        let unchanged = match (&current, &value) {
            (Some(current), Some(value)) => same_client(current, value),
            (None, None) => true,
            _ => false,
        };
        if unchanged {
            return;
        }

        if current.is_some() {
            let subscriptions = std::mem::take(&mut *self.client_subscriptions.borrow_mut());
            for subscription in subscriptions {
                subscription.dispose();
            }

            drop(self.client.replace(None));

            if let Some(im) = self.im() {
                im.reset();
            }
        }

        drop(self.client.replace(value.clone()));

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
                let (weak, sender) = (self.this.clone(), sender);
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

            self.populate_im_with_initial_values();
        } else {
            if let Some(im) = self.im() {
                im.set_client(None);
            }
            self.transform_tracker.set_visual(None);
        }
    }

    fn populate_im_with_initial_values(&self) {
        let im = self.im();

        if let Some(im) = &im {
            match self.focused_element() {
                Some(target) => im.set_options(&TextInputOptions::from_styled_element(&target)),
                None => im.set_options(&TextInputOptions::default_options()),
            }
        }

        let client = self.client();
        let text_view_visual = client.as_ref().map(|client| client.text_view_visual());
        self.transform_tracker.set_visual(text_view_visual.as_ref());

        if let Some(im) = &im {
            im.set_client(client);
        }

        self.update_cursor_rect();
    }

    fn on_reset_requested(&self, sender: &Weak<dyn TextInputMethodClient>) {
        if let Some(im) = self.im() {
            if self.is_current_client(sender) {
                im.reset();
                self.populate_im_with_initial_values();
            }
        }
    }

    fn on_is_input_method_enabled_changed(&self, sender: &crate::FerroObject) {
        let is_focused_element = self
            .focused_element
            .borrow()
            .as_ref()
            .is_some_and(|focused| {
                let focused: &crate::FerroObject = focused;
                std::ptr::eq(focused, sender)
            });

        if is_focused_element {
            self.try_find_and_apply_client();
        }
    }

    fn on_text_view_visual_changed(&self) {
        let text_view_visual = self.client().map(|client| client.text_view_visual());
        self.transform_tracker.set_visual(text_view_visual.as_ref());
    }

    fn update_cursor_rect(&self) {
        let (Some(im), Some(client), Some(v)) = (self.im(), self.client(), self.focused_element()) else {
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

        let input_method =
            element.as_ref().and_then(|element| element.get_input_root()).and_then(|root| root.input_method());

        let im = self.im();
        if !same_input_method(im.as_ref(), input_method.as_ref()) {
            if let Some(im) = &im {
                im.set_client(None);
            }
        }

        drop(self.im.replace(input_method));

        self.try_find_and_apply_client();
    }

    fn text_input_method_client_requery_requested(&self) {
        if self.im.borrow().is_some() {
            self.try_find_and_apply_client();
        }
    }

    fn try_find_and_apply_client(&self) {
        let focused = self.focused_element();
        let Some(focused) = focused.filter(|focused| {
            self.im.borrow().is_some() && InputMethod::get_is_input_method_enabled(focused)
        }) else {
            self.set_client(None);
            return;
        };

        let client_query = TextInputMethodClientRequestedEventArgs::new();
        client_query.set_routed_event(Some(InputElement::text_input_method_client_requested_event()));

        focused.raise_event(&client_query);
        self.set_client(client_query.client());
    }

    /// Releases the subscriptions of the manager.
    pub(crate) fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }

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

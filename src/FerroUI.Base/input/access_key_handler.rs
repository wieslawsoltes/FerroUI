use super::{
    FocusManager, IAccessKeyHandler, IMainMenu, InputElement, Key, KeyEventArgs, KeyModifiers, PointerPressedEventArgs,
};
use crate::interactivity::{RoutedEvent, RoutedEventArgs, RoutingStrategies};
use crate::reactive::IDisposable;
use crate::{
    ferro_property, ferro_routed_event, ferro_routed_event_args, AttachedProperty, FerroProperty, Ref,
    StyledPropertyOptions, Visual, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

type Element = Ref<InputElement>;

/// The inputs to an access key pressed event: raised on an element to ask
/// which element an access key should target.
#[derive(Clone)]
pub struct AccessKeyPressedEventArgs {
    base: RoutedEventArgs,
    target: Rc<RefCell<Option<Element>>>,
    key: String,
}

ferro_routed_event_args!(AccessKeyPressedEventArgs: RoutedEventArgs);

impl AccessKeyPressedEventArgs {
    /// Creates args for the given (normalized) access key.
    pub fn new(key: &str) -> Self {
        Self {
            base: RoutedEventArgs::with_event(AccessKeyHandler::access_key_pressed_event()),
            target: Rc::new(RefCell::new(None)),
            key: key.to_string(),
        }
    }

    /// The target for the access key.
    pub fn target(&self) -> Option<Element> {
        self.target.borrow().clone()
    }

    /// Sets the target for the access key.
    pub fn set_target(&self, value: Option<Element>) {
        let old = self.target.replace(value);
        drop(old);
    }

    /// The access key which was pressed.
    pub fn key(&self) -> &str {
        &self.key
    }
}

/// Information pertaining to when the access key associated with an element
/// is pressed.
#[derive(Clone)]
pub struct AccessKeyEventArgs {
    base: RoutedEventArgs,
    key: String,
    is_multiple: bool,
}

ferro_routed_event_args!(AccessKeyEventArgs: RoutedEventArgs);

impl AccessKeyEventArgs {
    /// Creates args for an access key. `is_multiple` tells whether the key
    /// is not unique among the registered elements.
    pub fn new(key: &str, is_multiple: bool) -> Self {
        Self {
            base: RoutedEventArgs::with_event(AccessKeyHandler::access_key_event()),
            key: key.to_string(),
            is_multiple,
        }
    }

    /// The key that was pressed which caused this event to fire.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Whether the key pressed is not unique among the registered elements.
    pub fn is_multiple(&self) -> bool {
        self.is_multiple
    }
}

/// An access key registered for an element.
pub struct AccessKeyRegistration {
    target: WeakRef<InputElement>,
    key: String,
}

impl AccessKeyRegistration {
    /// Creates a registration.
    pub fn new(key: String, target: WeakRef<InputElement>) -> Self {
        Self { target, key }
    }

    /// The (normalized) access key.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The registered element, if it is still alive.
    pub fn get_input_element(&self) -> Option<Element> {
        self.target.upgrade()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProcessKeyResult {
    NoMatch,
    MoreMatches,
    LastMatch,
}

/// Handles access keys for a window.
pub struct AccessKeyHandler {
    this: Weak<AccessKeyHandler>,
    /// The registered access keys.
    registrations: RefCell<Vec<AccessKeyRegistration>>,
    /// The window to which the handler belongs.
    owner: RefCell<Option<WeakRef<InputElement>>>,
    /// Whether access keys are currently being shown.
    showing_access_keys: Cell<bool>,
    /// Whether to ignore the Alt KeyUp event.
    ignore_alt_up: Cell<bool>,
    /// Whether the AltKey is down.
    alt_is_down: Cell<bool>,
    /// Element to restore following AltKey taking focus.
    restore_focus_element: RefCell<Option<WeakRef<InputElement>>>,
    /// The window's main menu.
    main_menu: RefCell<Option<Rc<dyn IMainMenu>>>,
    main_menu_closed: RefCell<Option<Rc<dyn IDisposable>>>,
}

crate::ferro_static_type!(AccessKeyHandler);

impl AccessKeyHandler {
    ferro_routed_event!(
        /// Defines the AccessKey event: raised on the element an access key
        /// targets.
        pub fn access_key_event() -> RoutedEvent<AccessKeyEventArgs> {
            RoutedEvent::register::<AccessKeyHandler, _>("AccessKey", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the AccessKeyPressed event: raised to find the target of
        /// an access key.
        pub fn access_key_pressed_event() -> RoutedEvent<AccessKeyPressedEventArgs> {
            RoutedEvent::register::<AccessKeyHandler, _>("AccessKeyPressed", RoutingStrategies::BUBBLE)
        }
    );

}

crate::ferro_properties! { impl AccessKeyHandler {
    ferro_property!(
        /// Defines the ShowAccessKey attached property: whether access key
        /// markers are shown in a window.
        pub fn show_access_key_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<AccessKeyHandler, Visual, _>(
                "ShowAccessKey",
                StyledPropertyOptions::new(false).inherits(true),
            )
        }
    );
} }

impl AccessKeyHandler {
    /// Creates an access key handler without an owner.
    pub fn new() -> Rc<AccessKeyHandler> {
        Rc::new_cyclic(|this| AccessKeyHandler {
            this: this.clone(),
            registrations: RefCell::new(Vec::new()),
            owner: RefCell::new(None),
            showing_access_keys: Cell::new(false),
            ignore_alt_up: Cell::new(false),
            alt_is_down: Cell::new(false),
            restore_focus_element: RefCell::new(None),
            main_menu: RefCell::new(None),
            main_menu_closed: RefCell::new(None),
        })
    }

    /// The number of registered access keys, including registrations of
    /// elements that are no longer alive and have not been purged yet.
    pub fn registration_count(&self) -> usize {
        self.registrations.borrow().len()
    }

    fn owner(&self) -> Option<Element> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// The registered access keys with the elements they target (the ones
    /// that are still alive). For handlers built on this one.
    pub fn registrations(&self) -> Vec<(String, Element)> {
        self.registrations
            .borrow()
            .iter()
            .filter_map(|registration| {
                registration.get_input_element().map(|element| (registration.key().to_owned(), element))
            })
            .collect()
    }

    /// Records the owner of the handler: the first part of
    /// [`IAccessKeyHandler::set_owner`].
    ///
    /// This method can only be called once.
    fn set_owner_without_handlers(&self, owner: &Element) {
        if self.owner.borrow().is_some() {
            panic!("AccessKeyHandler owner has already been set.");
        }

        *self.owner.borrow_mut() = Some(owner.downgrade());
    }

    /// Sets the owner of the handler as [`IAccessKeyHandler::set_owner`] does, with the key
    /// and pointer handlers attached, and then runs `on_set_owner`: the `OnSetOwner` virtual
    /// of the class, which does nothing in this class and which a handler built on this one
    /// overrides (the handler of a menu item attaches its text input handler there).
    ///
    /// This method can only be called once.
    pub fn set_owner_with(&self, owner: &Element, on_set_owner: impl FnOnce(&Element)) {
        IAccessKeyHandler::set_owner(self, owner);

        on_set_owner(owner);
    }

    fn set_show_access_keys(target: &Visual, value: bool) {
        target.set_value(Self::show_access_key_property(), value)
    }

    fn source_of(e: &RoutedEventArgs) -> Option<Element> {
        e.source().and_then(|source| source.downcast::<InputElement>().ok())
    }

    /// Called when a key is pressed in the owner window, before the event
    /// reaches the focused element.
    pub fn on_preview_key_down(&self, e: &KeyEventArgs) {
        let Some(owner) = self.owner() else { return };
        let source = Self::source_of(e);

        // If the event did not originate from within the owner, ignore all
        // keyboard events.
        let is_focus_within_owner = Self::is_focus_within_owner(&owner, source.as_ref());
        if !is_focus_within_owner {
            return;
        }

        if e.key == Key::LeftAlt || e.key == Key::RightAlt {
            self.alt_is_down.set(true);

            let main_menu = self.main_menu();
            if !main_menu.as_ref().is_some_and(|menu| menu.is_open()) {
                let focus_manager = source.as_ref().and_then(|source| FocusManager::get_focus_manager(source));

                // Save the currently focused input element.
                if let Some(focused_element) = focus_manager.and_then(|manager| manager.get_focused_element()) {
                    *self.restore_focus_element.borrow_mut() = Some(focused_element.downgrade());
                }

                // When Alt is pressed without a main menu, or with a closed
                // main menu, show access key markers in the window (i.e.
                // "_File").
                self.showing_access_keys.set(is_focus_within_owner);
                Self::set_show_access_keys(&owner, is_focus_within_owner);
            } else {
                // If the Alt key is pressed and the main menu is open, close
                // the main menu.
                self.close_menu(&owner);
                self.ignore_alt_up.set(true);

                let restore_element = self.restore_focus_element.borrow_mut().take().and_then(|weak| weak.upgrade());
                if let Some(restore_element) = restore_element {
                    restore_element.focus();
                }
            }
        } else if self.alt_is_down.get() {
            self.ignore_alt_up.set(true);
        }
    }

    /// Called when a key is pressed in the owner window.
    pub fn on_key_down(&self, e: &KeyEventArgs) {
        let Some(owner) = self.owner() else { return };
        let source = Self::source_of(e);

        // If the event did not originate from within the owner, ignore all
        // keyboard events.
        if !Self::is_focus_within_owner(&owner, source.as_ref()) {
            return;
        }

        if (!e.key_modifiers.contains(KeyModifiers::ALT) || e.key_modifiers.contains(KeyModifiers::CONTROL))
            && !self.main_menu().is_some_and(|menu| menu.is_open())
        {
            return;
        }

        e.set_handled(self.process_key(e.key_symbol.as_deref(), source.as_ref()));
    }

    /// Handles the Alt/F10 keys being released in the window.
    pub fn on_preview_key_up(&self, e: &KeyEventArgs) {
        if e.key == Key::LeftAlt || e.key == Key::RightAlt {
            self.alt_is_down.set(false);

            if self.ignore_alt_up.get() {
                self.ignore_alt_up.set(false);
            } else if self.showing_access_keys.get() {
                if let Some(main_menu) = self.main_menu() {
                    main_menu.open();
                }
            }
        }
    }

    /// Handles a pointer press in the window: hides the access key markers.
    pub fn on_preview_pointer_pressed(&self, _e: &PointerPressedEventArgs) {
        if self.showing_access_keys.get() {
            if let Some(owner) = self.owner() {
                Self::set_show_access_keys(&owner, false);
            }
        }
    }

    /// Closes the main menu and performs other bookeeping.
    fn close_menu(&self, owner: &Element) {
        if let Some(main_menu) = self.main_menu() {
            main_menu.close();
        }
        self.showing_access_keys.set(false);
        Self::set_show_access_keys(owner, false);
    }

    fn main_menu_closed(&self) {
        if let Some(owner) = self.owner() {
            Self::set_show_access_keys(&owner, false);
        }
    }

    /// Processes an access key: raises the AccessKey event on the element
    /// the key targets. Returns whether a target was found.
    pub fn process_key(&self, key: Option<&str>, element: Option<&Element>) -> bool {
        let Some(key) = key.filter(|key| !key.is_empty()) else { return false };
        let key = Self::normalize_key(key);
        let sender_target = Self::get_target_for_element(element, &key);

        // Find the possible targets matching the access key
        let targets = Self::sort_by_hierarchy(self.get_targets_for_key(&key, element, sender_target));
        Self::process_key_for_targets(&key, &targets) != ProcessKeyResult::NoMatch
    }

    fn normalize_key(key: &str) -> String {
        key.to_uppercase()
    }

    fn process_key_for_targets(key: &str, targets: &[Element]) -> ProcessKeyResult {
        if targets.is_empty() {
            return ProcessKeyResult::NoMatch;
        }

        let mut is_single_target = true;
        let mut last_was_focused = false;
        let mut effective_target: Option<&Element> = None;
        let mut chosen_index = 0;

        for (i, target) in targets.iter().enumerate() {
            if !Self::is_targetable(target) {
                continue;
            }

            if effective_target.is_none() {
                effective_target = Some(target);
                chosen_index = i;
            } else {
                if last_was_focused {
                    effective_target = Some(target);
                    chosen_index = i;
                }

                is_single_target = false;
            }

            last_was_focused = target.is_focused();
        }

        let Some(effective_target) = effective_target else { return ProcessKeyResult::NoMatch };

        let args = AccessKeyEventArgs::new(key, !is_single_target);
        effective_target.raise_event(&args);

        if chosen_index == targets.len() - 1 {
            ProcessKeyResult::LastMatch
        } else {
            ProcessKeyResult::MoreMatches
        }
    }

    /// Get the list of access key targets for the sender of the keyboard
    /// event. If sender is null, pretend key was pressed in the active
    /// window.
    fn get_targets_for_key(&self, key: &str, sender: Option<&Element>, sender_target: Option<Element>) -> Vec<Element> {
        let possible_elements = self.copy_matching_and_purge_dead(key);
        let mut final_targets = Vec::with_capacity(1);

        // Go through all the possible elements, find the interesting
        // candidates
        for element in &possible_elements {
            if Some(element) != sender {
                if !Self::is_targetable(element) {
                    continue;
                }

                if let Some(target) = Self::get_target_for_element(Some(element), key) {
                    final_targets.push(target);
                }
            } else if let Some(target) = &sender_target {
                // This is the same element that sent the event so it must be
                // in the same scope. Just add it to the final targets
                final_targets.push(target.clone());
            }
        }

        final_targets
    }

    fn is_targetable(element: &InputElement) -> bool {
        element.is_effectively_enabled() && element.is_effectively_visible()
    }

    fn copy_matching_and_purge_dead(&self, key: &str) -> Vec<Element> {
        let mut registrations = self.registrations.borrow_mut();
        let mut matches = Vec::with_capacity(registrations.len());

        // collect live elements with matching key and remove dead elements
        registrations.retain(|registration| match registration.get_input_element() {
            Some(input_element) => {
                if registration.key == key {
                    matches.push(input_element);
                }
                true
            }
            None => false,
        });

        matches
    }

    /// Returns targeting information for the given element: raises the
    /// AccessKeyPressed event on it and returns the target it names.
    fn get_target_for_element(element: Option<&Element>, key: &str) -> Option<Element> {
        let element = element?;

        let args = AccessKeyPressedEventArgs::new(key);
        element.raise_event(&args);
        args.target()
    }

    /// Checks if the focused element is a descendent of the owner.
    fn is_focus_within_owner(owner: &Element, source: Option<&Element>) -> bool {
        match source {
            Some(source) => source == owner || owner.is_visual_ancestor_of(source),
            None => false,
        }
    }

    /// Sorts the list of targets according to logical ancestors in the
    /// hierarchy so that child elements, for example tab item content, are
    /// processed before the next parent item, i.e. the next tab item.
    fn sort_by_hierarchy(targets: Vec<Element>) -> Vec<Element> {
        // bail out, if there are no targets to sort
        if targets.len() <= 1 {
            return targets;
        }

        let mut sorted: Vec<Element> = Vec::with_capacity(targets.len());
        let mut queue: VecDeque<Element> = targets.into();

        while let Some(element) = queue.pop_front() {
            // if the element was already added, do nothing
            if sorted.contains(&element) {
                continue;
            }

            // add the element itself
            sorted.push(element.clone());

            // add all descendants of the element
            sorted.extend(queue.iter().filter(|child| element.is_logical_ancestor_of(Some(child))).cloned());
        }

        sorted
    }
}

impl IAccessKeyHandler for AccessKeyHandler {
    fn main_menu(&self) -> Option<Rc<dyn IMainMenu>> {
        self.main_menu.borrow().clone()
    }

    fn set_main_menu(&self, value: Option<Rc<dyn IMainMenu>>) {
        let subscription = self.main_menu_closed.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }

        let old = self.main_menu.replace(value.clone());
        drop(old);

        if let Some(main_menu) = value {
            let this = self.this.clone();
            let subscription = main_menu.closed(Rc::new(move |_| {
                if let Some(this) = this.upgrade() {
                    this.main_menu_closed();
                }
            }));
            *self.main_menu_closed.borrow_mut() = Some(subscription);
        }
    }

    fn set_owner(&self, owner: &Element) {
        self.set_owner_without_handlers(owner);

        let handler = |f: fn(&AccessKeyHandler, &KeyEventArgs)| {
            let this = self.this.clone();
            move |_: &crate::interactivity::Interactive, e: &KeyEventArgs| {
                if let Some(this) = this.upgrade() {
                    f(&this, e);
                }
            }
        };

        owner.add_handler_with(
            InputElement::key_down_event(),
            handler(Self::on_preview_key_down),
            RoutingStrategies::TUNNEL,
            false,
        );
        owner.add_handler_with(
            InputElement::key_down_event(),
            handler(Self::on_key_down),
            RoutingStrategies::BUBBLE,
            false,
        );
        owner.add_handler_with(
            InputElement::key_up_event(),
            handler(Self::on_preview_key_up),
            RoutingStrategies::TUNNEL,
            false,
        );

        let this = self.this.clone();
        owner.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e| {
                if let Some(this) = this.upgrade() {
                    this.on_preview_pointer_pressed(e);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
    }

    fn register(&self, access_key: &str, element: &Element) {
        assert!(!access_key.is_empty(), "The access key cannot be empty.");

        let key = Self::normalize_key(access_key);
        let mut registrations = self.registrations.borrow_mut();

        // remove dead elements with matching key
        registrations.retain(|registration| !(registration.key == key && registration.get_input_element().is_none()));

        registrations.push(AccessKeyRegistration::new(key, element.downgrade()));
    }

    fn unregister(&self, element: &InputElement) {
        // remove element and all dead elements
        self.registrations.borrow_mut().retain(|registration| match registration.get_input_element() {
            Some(input_element) => !std::ptr::eq::<InputElement>(&*input_element, element),
            None => false,
        });
    }
}

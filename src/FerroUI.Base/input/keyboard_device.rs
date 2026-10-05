use super::raw::{IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawTextInputEventArgs};
use super::text_input::TextInputMethodManager;
use super::{
    FocusChangedEventArgs, FocusChangingEventArgs, IFocusManager, IInputDevice, IInputManager, IInputRoot, IKeyboardDevice, InputElement,
    KeyEventArgs, KeyModifiers, NavigationMethod, TextInputEventArgs,
};
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use crate::{FerroLocator, LocatorExtensions, Ref};
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Represents the keyboard: turns raw key and text input into routed
/// events on the focused element, and is the source of truth about which
/// element has keyboard focus.
pub struct KeyboardDevice {
    this: Weak<KeyboardDevice>,
    focused_element: RefCell<Option<Ref<InputElement>>>,
    focused_root: RefCell<Option<Rc<dyn IInputRoot>>>,
    text_input_manager: Rc<TextInputMethodManager>,
    property_changed: HandlerList<dyn Fn(&str)>,
}

impl KeyboardDevice {
    /// Creates a keyboard device.
    pub fn new() -> Rc<KeyboardDevice> {
        Rc::new_cyclic(|this| KeyboardDevice {
            this: this.clone(),
            focused_element: RefCell::new(None),
            focused_root: RefCell::new(None),
            text_input_manager: TextInputMethodManager::new(),
            property_changed: HandlerList::new(),
        })
    }

    /// The keyboard device registered with the locator (as
    /// `dyn IKeyboardDevice`), if it is a [`KeyboardDevice`].
    pub fn instance() -> Option<Rc<KeyboardDevice>> {
        FerroLocator::current()
            .get_service::<dyn IKeyboardDevice>()
            .and_then(|device| device.as_any().downcast_ref::<KeyboardDevice>().and_then(|d| d.this.upgrade()))
    }

    /// The input manager registered with the locator.
    pub fn input_manager(&self) -> Option<Rc<dyn IInputManager>> {
        FerroLocator::current().get_service::<dyn IInputManager>()
    }

    /// The focus manager registered with the locator.
    pub fn focus_manager(&self) -> Option<Rc<dyn IFocusManager>> {
        FerroLocator::current().get_service::<dyn IFocusManager>()
    }

    /// Raised with the name of a property of the device that changed
    /// (`"FocusedElement"`).
    pub fn property_changed(&self, handler: impl Fn(&str) + 'static) -> Rc<dyn IDisposable> {
        let token = self.property_changed.add(Rc::new(handler));
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.property_changed.remove(token);
            }
        })
    }

    /// The element that has keyboard focus.
    pub fn focused_element(&self) -> Option<Ref<InputElement>> {
        self.focused_element.borrow().clone()
    }

    fn visual_parent_element(element: &InputElement) -> Option<Ref<InputElement>> {
        element.visual_parent().and_then(|parent| parent.downcast::<InputElement>().ok())
    }

    fn clear_focus_within_ancestors(element: Option<Ref<InputElement>>) {
        let mut el = element;

        while let Some(current) = el {
            current.set_is_keyboard_focus_within(false);
            // A parent that is not an input element ends the walk.
            el = Self::visual_parent_element(&current);
        }
    }

    fn clear_focus_within(element: &InputElement, clear_root: bool) {
        if let Some(children) = element.visual_children_snapshot() {
            for visual in children.iter() {
                if let Some(el) = visual.downcast_ref::<InputElement>() {
                    if el.is_keyboard_focus_within() {
                        Self::clear_focus_within(el, true);
                        break;
                    }
                }
            }
        }

        if clear_root {
            element.set_is_keyboard_focus_within(false);
        }
    }

    fn set_is_focus_within(old_element: Option<&Ref<InputElement>>, new_element: Option<&Ref<InputElement>>) {
        if new_element.is_none() && old_element.is_some() {
            Self::clear_focus_within_ancestors(old_element.cloned());
            return;
        }

        let mut branch: Option<Ref<InputElement>> = None;
        let mut el = new_element.cloned();

        while let Some(current) = el {
            if current.is_keyboard_focus_within() {
                branch = Some(current);
                break;
            }
            el = Self::visual_parent_element(&current);
        }

        if let (Some(_), Some(branch)) = (old_element, &branch) {
            Self::clear_focus_within(branch, false);
        }

        el = new_element.cloned();

        while let Some(current) = el {
            if Some(&current) == branch.as_ref() {
                break;
            }
            current.set_is_keyboard_focus_within(true);
            el = Self::visual_parent_element(&current);
        }
    }

    fn clear_children_focus_within(element: &InputElement, clear_root: bool) {
        if let Some(children) = element.visual_children_snapshot() {
            for visual in children.iter() {
                if let Some(el) = visual.downcast_ref::<InputElement>() {
                    if el.is_keyboard_focus_within() {
                        Self::clear_children_focus_within(el, true);
                        break;
                    }
                }
            }
        }

        if clear_root {
            element.set_is_keyboard_focus_within(false);
        }
    }

    /// Moves keyboard focus to an element (or clears it); the change can be
    /// cancelled or redirected by `LosingFocus`/`GettingFocus` handlers.
    pub fn set_focused_element(
        &self,
        element: Option<&Ref<InputElement>>,
        method: NavigationMethod,
        key_modifiers: KeyModifiers,
    ) {
        self.set_focused_element_with(element, method, key_modifiers, true);
    }

    /// Moves keyboard focus to an element (or clears it).
    ///
    /// `is_focus_change_cancellable` tells whether `LosingFocus` and
    /// `GettingFocus` handlers may cancel or redirect the change.
    pub fn set_focused_element_with(
        &self,
        element: Option<&Ref<InputElement>>,
        method: NavigationMethod,
        key_modifiers: KeyModifiers,
        is_focus_change_cancellable: bool,
    ) {
        let focused_element = self.focused_element();

        if element == focused_element.as_ref() {
            return;
        }

        let mut element = element.cloned();
        let interactive = focused_element.clone();
        let mut change_focus = true;

        let mut losing_focus = FocusChangingEventArgs::new(InputElement::losing_focus_event());
        losing_focus.old_focused_element = focused_element.clone();
        losing_focus.set_new_focused_element(element.clone());
        losing_focus.navigation_method = method;
        losing_focus.key_modifiers = key_modifiers;
        losing_focus.can_cancel_or_redirect_focus = is_focus_change_cancellable;

        if let Some(interactive) = &interactive {
            interactive.raise_event(&losing_focus);
        }

        if losing_focus.canceled() {
            change_focus = false;
        }

        if change_focus {
            if let Some(new_focus) = losing_focus.new_focused_element() {
                let mut getting_focus = FocusChangingEventArgs::new(InputElement::getting_focus_event());
                getting_focus.old_focused_element = focused_element.clone();
                getting_focus.set_new_focused_element(Some(new_focus.clone()));
                getting_focus.navigation_method = method;
                getting_focus.key_modifiers = key_modifiers;
                getting_focus.can_cancel_or_redirect_focus = is_focus_change_cancellable;

                new_focus.raise_event(&getting_focus);

                if getting_focus.canceled() {
                    change_focus = false;
                }

                element = getting_focus.new_focused_element();
            }
        }

        if !change_focus {
            return;
        }

        let old_element = self.focused_element();
        let focused_root = self.focused_root.borrow().clone();

        // Clear keyboard focus from the currently focused element.
        if let (Some(old), Some(focused_root)) = (&old_element, &focused_root) {
            let new_root = element.as_ref().and_then(|e| e.get_input_root());
            let same_root = new_root.as_ref().is_some_and(|r| std::ptr::addr_eq(Rc::as_ptr(r), Rc::as_ptr(focused_root)));

            if !old.is_attached_to_visual_tree() || !same_root {
                Self::clear_children_focus_within(&focused_root.root_element(), true);
            }
        }

        Self::set_is_focus_within(old_element.as_ref(), element.as_ref());
        drop(self.focused_element.replace(element.clone()));
        drop(self.focused_root.replace(element.as_ref().and_then(|e| e.get_input_root())));

        if let Some(interactive) = &interactive {
            let mut lost_focus = FocusChangedEventArgs::new(InputElement::lost_focus_event());
            lost_focus.old_focused_element = old_element.clone();
            lost_focus.new_focused_element = element.clone();
            lost_focus.navigation_method = method;
            lost_focus.key_modifiers = key_modifiers;
            interactive.raise_event(&lost_focus);
        }

        if let Some(new_element) = &element {
            let mut got_focus = FocusChangedEventArgs::new(InputElement::got_focus_event());
            got_focus.old_focused_element = old_element.clone();
            got_focus.new_focused_element = element.clone();
            got_focus.navigation_method = method;
            got_focus.key_modifiers = key_modifiers;
            new_element.raise_event(&got_focus);
        }

        self.text_input_manager.set_focused_element(element);
        self.raise_property_changed("FocusedElement");
    }

    fn raise_property_changed(&self, property_name: &str) {
        if self.property_changed.is_empty() {
            return;
        }
        for (_, handler) in self.property_changed.snapshot().iter() {
            handler(property_name);
        }
    }
}

impl IInputDevice for KeyboardDevice {
    fn process_raw_event(&self, e: &dyn IRawInputEventArgs) {
        if e.handled() {
            return;
        }

        let element = self.focused_element().unwrap_or_else(|| e.root().focus_root());

        if let Some(key_input) = e.downcast_ref::<RawKeyEventArgs>() {
            let routed_event = match key_input.type_() {
                RawKeyEventType::KeyDown => InputElement::key_down_event(),
                RawKeyEventType::KeyUp => InputElement::key_up_event(),
            };

            let mut ev = KeyEventArgs::new();
            ev.set_routed_event(Some(routed_event));
            ev.key = key_input.key();
            ev.key_modifiers = key_input.modifiers().to_key_modifiers();
            ev.physical_key = key_input.physical_key();
            ev.key_symbol = key_input.key_symbol();
            ev.key_device_type = key_input.key_device_type();
            ev.set_source(&element);

            if key_input.type_() == RawKeyEventType::KeyDown {
                let mut current_handler: Option<Ref<crate::Visual>> = Some(element.clone().upcast());

                while let Some(current) = current_handler {
                    if ev.handled() {
                        break;
                    }

                    if let Some(input_element) = current.downcast_ref::<InputElement>() {
                        // Work on a copy of the key bindings if there's a
                        // binding which matches the event: handling a
                        // binding may change the list (e.g. when a new view
                        // is loaded which adds its own key bindings).
                        let bindings_copy = {
                            let bindings = input_element.key_bindings().snapshot();
                            if bindings.iter().any(|b| b.gesture().is_some_and(|g| g.matches(Some(&ev)))) {
                                Some(bindings)
                            } else {
                                None
                            }
                        };

                        if let Some(bindings_copy) = bindings_copy {
                            for binding in bindings_copy.iter() {
                                if ev.handled() {
                                    break;
                                }
                                binding.try_handle(&ev);
                            }
                        }
                    }

                    current_handler = current.visual_parent();
                }
            }

            element.raise_event(&ev);
            e.set_handled(ev.handled());
        }

        if let Some(text) = e.downcast_ref::<RawTextInputEventArgs>() {
            let mut ev = TextInputEventArgs::new();
            ev.text = Some(text.text().to_string());
            ev.set_source(&element);
            ev.set_routed_event(Some(InputElement::text_input_event()));

            element.raise_event(&ev);
            e.set_handled(ev.handled());
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IKeyboardDevice for KeyboardDevice {}

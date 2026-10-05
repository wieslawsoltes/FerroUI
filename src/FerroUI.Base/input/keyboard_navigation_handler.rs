use super::navigation::{TabNavigation, XYFocus};
use super::{
    FocusManager, IKeyboardNavigationHandler, InputElement, Key, KeyDeviceType, KeyEventArgs, KeyModifiers,
    NavigationDirection, NavigationMethod,
};
use crate::interactivity::Interactive;
use crate::Ref;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

type Element = Ref<InputElement>;

/// Handles keyboard navigation for a window.
pub struct KeyboardNavigationHandler {
    this: Weak<KeyboardNavigationHandler>,
    /// The window to which the handler belongs.
    owner: RefCell<Option<crate::WeakRef<InputElement>>>,
}

impl KeyboardNavigationHandler {
    /// Creates a keyboard navigation handler without an owner.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), owner: RefCell::new(None) })
    }

    fn owner(&self) -> Option<Element> {
        self.owner.borrow().as_ref().and_then(crate::WeakRef::upgrade)
    }

    /// Gets the next control in the specified navigation direction.
    ///
    /// Returns the next element in the specified direction, or `None` if
    /// `element` was the last in the requested direction.
    ///
    /// Panics for directions other than next, previous, up, down, left and
    /// right.
    pub fn get_next(element: &Element, direction: NavigationDirection) -> Option<Element> {
        Self::get_next_private(Some(element), None, direction, None)
    }

    fn find_custom(element: &Element) -> Option<Element> {
        element.find_ancestor_of_type_where::<InputElement>(true, |e| e.as_custom_keyboard_navigation().is_some())
    }

    fn get_next_private(
        element: Option<&Element>,
        owner: Option<&Element>,
        direction: NavigationDirection,
        key_device_type: Option<KeyDeviceType>,
    ) -> Option<Element> {
        let element_or_owner = element.or(owner).expect("Either the element or the owner must be set.");

        // If there's a custom keyboard navigation handler as an ancestor,
        // use that.
        let custom = element.and_then(Self::find_custom);
        if let Some(custom) = &custom {
            if let Some(ce) = Self::handle_pre_custom_navigation(custom, element_or_owner, direction) {
                return Some(ce);
            }
        }

        let result = match direction {
            NavigationDirection::Next => TabNavigation::get_next_tab(element_or_owner, false),
            NavigationDirection::Previous => TabNavigation::get_prev_tab(Some(element_or_owner), None, false),
            NavigationDirection::Up
            | NavigationDirection::Down
            | NavigationDirection::Left
            | NavigationDirection::Right => match element {
                // We don't have a start element, so use tab navigation to
                // find the first one.
                None => TabNavigation::get_next_tab(element_or_owner, true),
                Some(element) => XYFocus::try_directional_focus(direction, element, owner, None, key_device_type),
            },
            _ => panic!("The navigation direction {direction:?} is not supported."),
        };

        // If there wasn't a custom navigation handler as an ancestor of the
        // current element, but there is one as an ancestor of the new
        // element, use the custom handler to find the next element.
        if custom.is_none() {
            if let Some(ce) = Self::handle_post_custom_navigation(element_or_owner, result.as_ref(), direction) {
                return Some(ce);
            }
        }

        result
    }

    fn on_key_down(&self, _sender: &Interactive, e: &KeyEventArgs) {
        let direction = match e.key {
            Key::Tab => {
                if !e.key_modifiers.contains(KeyModifiers::SHIFT) {
                    NavigationDirection::Next
                } else {
                    NavigationDirection::Previous
                }
            }
            Key::Left => NavigationDirection::Left,
            Key::Right => NavigationDirection::Right,
            Key::Up => NavigationDirection::Up,
            Key::Down => NavigationDirection::Down,
            _ => return,
        };

        let source = e.source().and_then(|source| source.downcast::<InputElement>().ok());
        let current = source
            .and_then(|source| FocusManager::get_focus_manager(&source))
            .and_then(|focus_manager| focus_manager.get_focused_element());

        e.set_handled(self.move_(current.as_ref(), direction, e.key_modifiers, Some(e.key_device_type)));
    }

    fn handle_pre_custom_navigation(
        custom_handler: &Element,
        element: &Element,
        direction: NavigationDirection,
    ) -> Option<Element> {
        let (handled, next) = custom_handler.as_custom_keyboard_navigation()?.get_next(element, direction);

        if handled {
            if next.is_some() {
                return next;
            }

            return match direction {
                NavigationDirection::Next => TabNavigation::get_next_tab_outside(custom_handler),
                NavigationDirection::Previous => TabNavigation::get_prev_tab_outside(custom_handler),
                _ => None,
            };
        }

        None
    }

    fn handle_post_custom_navigation(
        element: &Element,
        new_element: Option<&Element>,
        direction: NavigationDirection,
    ) -> Option<Element> {
        let custom_handler = Self::find_custom(new_element?)?;
        let (handled, next) = custom_handler.as_custom_keyboard_navigation()?.get_next(element, direction);

        if handled {
            next
        } else {
            None
        }
    }
}

impl IKeyboardNavigationHandler for KeyboardNavigationHandler {
    fn set_owner(&self, owner: &Element) {
        if self.owner.borrow().is_some() {
            panic!("KeyboardNavigationHandler owner has already been set.");
        }

        *self.owner.borrow_mut() = Some(owner.downgrade());

        let this = self.this.clone();
        owner.add_handler(InputElement::key_down_event(), move |sender, e| {
            if let Some(this) = this.upgrade() {
                this.on_key_down(sender, e);
            }
        });
    }

    fn move_(
        &self,
        element: Option<&Element>,
        direction: NavigationDirection,
        key_modifiers: KeyModifiers,
        device_type: Option<KeyDeviceType>,
    ) -> bool {
        let owner = self.owner();
        let next = Self::get_next_private(element, owner.as_ref(), direction, device_type);

        match next {
            Some(next) => {
                let method = if direction == NavigationDirection::Next || direction == NavigationDirection::Previous {
                    NavigationMethod::Tab
                } else {
                    NavigationMethod::Directional
                };

                next.focus_with(method, key_modifiers)
            }
            None => false,
        }
    }
}

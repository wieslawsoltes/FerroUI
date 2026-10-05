use super::{IKeyModifiersEventArgs, InputElement, KeyModifiers, NavigationMethod};
use crate::interactivity::{RoutedEvent, RoutedEventArgs};
use crate::{ferro_routed_event_args, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Provides data for focus changing: the `GettingFocus` and `LosingFocus`
/// events, which can cancel or redirect a focus change.
#[derive(Clone)]
pub struct FocusChangingEventArgs {
    base: RoutedEventArgs,
    new_focused_element: Rc<RefCell<Option<Ref<InputElement>>>>,
    canceled: Rc<Cell<bool>>,
    pub(crate) can_cancel_or_redirect_focus: bool,

    /// The element that had focus before the change.
    pub old_focused_element: Option<Ref<InputElement>>,

    /// Indicates how the change in focus occurred.
    pub navigation_method: NavigationMethod,

    /// Any key modifiers active at the time of focus.
    pub key_modifiers: KeyModifiers,
}

ferro_routed_event_args!(FocusChangingEventArgs: RoutedEventArgs);

impl FocusChangingEventArgs {
    /// Creates args for a routed event.
    pub fn new<T: ?Sized>(routed_event: &RoutedEvent<T>) -> Self {
        Self {
            base: RoutedEventArgs::with_event(routed_event),
            new_focused_element: Rc::new(RefCell::new(None)),
            canceled: Rc::new(Cell::new(false)),
            can_cancel_or_redirect_focus: false,
            old_focused_element: None,
            navigation_method: NavigationMethod::Unspecified,
            key_modifiers: KeyModifiers::NONE,
        }
    }

    /// The element that focus will move to.
    pub fn new_focused_element(&self) -> Option<Ref<InputElement>> {
        self.new_focused_element.borrow().clone()
    }

    pub(crate) fn set_new_focused_element(&self, value: Option<Ref<InputElement>>) {
        let old = self.new_focused_element.replace(value);
        drop(old);
    }

    /// Whether the focus change is canceled.
    pub fn canceled(&self) -> bool {
        self.canceled.get()
    }

    /// Attempts to cancel the current focus change. Returns true if the
    /// focus change was cancelled; otherwise, false.
    pub fn try_cancel(&self) -> bool {
        self.canceled.set(self.can_cancel_or_redirect_focus);
        self.canceled.get()
    }

    /// Attempts to redirect focus from the targeted element to the
    /// specified element. Returns true if the focus change was redirected;
    /// otherwise, false.
    pub fn try_set_new_focused_element(&self, input_element: Option<&Ref<InputElement>>) -> bool {
        if self.can_cancel_or_redirect_focus {
            self.set_new_focused_element(input_element.cloned());
        }

        input_element == self.new_focused_element.borrow().as_ref()
    }
}

impl IKeyModifiersEventArgs for FocusChangingEventArgs {
    fn key_modifiers(&self) -> KeyModifiers {
        self.key_modifiers
    }
}

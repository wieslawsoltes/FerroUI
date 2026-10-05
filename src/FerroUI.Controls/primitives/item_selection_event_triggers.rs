use crate::Application;
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::{
    FocusChangedEventArgs, FocusChangingEventArgs, IKeyModifiersEventArgs, InputElement, Key, KeyEventArgs,
    KeyModifiers, PointerEventArgs, PointerType, PointerUpdateKind, TappedEventArgs,
};
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::{Rect, Visual};
use std::rc::Rc;

/// Defines standard logic for selecting items via user input. Behaviour
/// differs between input devices.
pub struct ItemSelectionEventTriggers;

impl ItemSelectionEventTriggers {
    /// Analyses an input event received by a selectable element, and
    /// determines whether the action should trigger selection on press, on
    /// release, or not at all.
    ///
    /// `selectable` is the selectable element which is processing the
    /// event.
    ///
    /// This method expects to analyse pointer pressed and pointer released
    /// events.
    pub fn should_trigger_selection(selectable: &Visual, event_args: &PointerEventArgs) -> bool {
        if !Self::is_pointer_event_within_bounds(selectable, event_args) {
            // Don't select if the pointer has moved away from the element
            // since being pressed.
            return false;
        }

        let kind = event_args.properties().pointer_update_kind;
        let routed_event = event_args.routed_event();
        let pressed = || routed_event.is_some_and(|event| event == *InputElement::pointer_pressed_event());
        let released = || routed_event.is_some_and(|event| event == *InputElement::pointer_released_event());

        // Only select for left/right button events.
        if !matches!(
            kind,
            PointerUpdateKind::LeftButtonPressed
                | PointerUpdateKind::RightButtonPressed
                | PointerUpdateKind::LeftButtonReleased
                | PointerUpdateKind::RightButtonReleased
        ) {
            return false;
        }

        match event_args.pointer().type_() {
            // Select on mouse press, unless the mouse can generate gestures.
            PointerType::Mouse => {
                if InputElement::get_is_hold_with_mouse_enabled(selectable_element(selectable)) {
                    released()
                } else {
                    pressed()
                }
            }
            // Pen "right clicks" are used for context menus, and gestures
            // are only processed for primary input.
            PointerType::Pen
                if matches!(kind, PointerUpdateKind::RightButtonPressed | PointerUpdateKind::RightButtonReleased) =>
            {
                pressed()
            }
            // For all other pen input, select on release.
            PointerType::Pen => released(),
            // Select on touch release.
            PointerType::Touch => released(),
        }
    }

    /// Whether the position of a pointer event is within the bounds of the
    /// selectable element.
    pub(crate) fn is_pointer_event_within_bounds(selectable: &Visual, event_args: &PointerEventArgs) -> bool {
        Rect::from_size(selectable.bounds().size()).contains(event_args.get_position(Some(selectable)))
    }

    /// Analyses an input event received by a selectable element, and
    /// determines whether the action should trigger selection.
    ///
    /// This method expects to analyse key down events.
    pub fn should_trigger_selection_key(selectable: &Visual, event_args: &KeyEventArgs) -> bool {
        // Only accept space/enter key presses directly from the selectable,
        // otherwise key input can become unpredictable.
        if event_args.is_source(selectable) && matches!(event_args.key, Key::Space | Key::Enter) {
            event_args.routed_event().is_some_and(|event| event == *InputElement::key_down_event())
        } else {
            false
        }
    }

    /// Whether `event_args` should trigger range selection.
    pub fn has_range_selection_modifier(selectable: &Visual, event_args: &dyn IRoutedEventArgs) -> bool {
        Self::has_modifiers(event_args, Self::hotkeys(selectable).map(|hotkeys| hotkeys.selection_modifiers))
    }

    /// Whether `event_args` should trigger toggle selection.
    pub fn has_toggle_selection_modifier(selectable: &Visual, event_args: &dyn IRoutedEventArgs) -> bool {
        Self::has_modifiers(event_args, Self::hotkeys(selectable).map(|hotkeys| hotkeys.command_modifiers))
    }

    fn hotkeys(element: &Visual) -> Option<Rc<PlatformHotkeyConfiguration>> {
        element
            .get_platform_settings()
            .or_else(|| Application::current().and_then(|application| application.platform_settings()))
            .map(|settings| settings.hotkey_configuration())
    }

    fn has_modifiers(event_args: &dyn IRoutedEventArgs, modifiers: Option<KeyModifiers>) -> bool {
        let Some(modifiers) = modifiers else { return false };
        match Self::key_modifiers_of(event_args) {
            Some(event_modifiers) => event_modifiers.contains(modifiers),
            None => false,
        }
    }

    /// The key modifiers of event args that carry key modifiers.
    fn key_modifiers_of(event_args: &dyn IRoutedEventArgs) -> Option<KeyModifiers> {
        if let Some(e) = event_args.downcast_ref::<PointerEventArgs>() {
            return Some(IKeyModifiersEventArgs::key_modifiers(e));
        }
        if let Some(e) = event_args.downcast_ref::<KeyEventArgs>() {
            return Some(IKeyModifiersEventArgs::key_modifiers(e));
        }
        if let Some(e) = event_args.downcast_ref::<FocusChangedEventArgs>() {
            return Some(IKeyModifiersEventArgs::key_modifiers(e));
        }
        if let Some(e) = event_args.downcast_ref::<FocusChangingEventArgs>() {
            return Some(IKeyModifiersEventArgs::key_modifiers(e));
        }
        if let Some(e) = event_args.downcast_ref::<TappedEventArgs>() {
            return Some(IKeyModifiersEventArgs::key_modifiers(e));
        }
        None
    }
}

fn selectable_element(selectable: &Visual) -> &ferroui_base::StyledElement {
    selectable
}

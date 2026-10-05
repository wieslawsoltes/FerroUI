use super::{Key, KeyModifiers};

/// Describes how focus should be moved by directional or tab keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NavigationDirection {
    /// Move the focus to the next control in the tab order.
    Next,
    /// Move the focus to the previous control in the tab order.
    Previous,
    /// Move the focus to the first control in the tab order.
    First,
    /// Move the focus to the last control in the tab order.
    Last,
    /// Move the focus to the left.
    Left,
    /// Move the focus to the right.
    Right,
    /// Move the focus up.
    Up,
    /// Move the focus down.
    Down,
    /// Move the focus up a page.
    PageUp,
    /// Move the focus down a page.
    PageDown,
}

impl NavigationDirection {
    /// Checks whether a navigation direction is a tab movement.
    pub fn is_tab(self) -> bool {
        self == NavigationDirection::Next || self == NavigationDirection::Previous
    }

    /// Checks whether a navigation direction is a directional movement.
    pub fn is_directional(self) -> bool {
        self > NavigationDirection::Previous && self <= NavigationDirection::PageDown
    }
}

impl Key {
    /// Converts a keypress into a navigation direction, if the key is a
    /// navigation key.
    pub fn to_navigation_direction(self, modifiers: KeyModifiers) -> Option<NavigationDirection> {
        match self {
            Key::Tab => Some(if !modifiers.contains(KeyModifiers::SHIFT) {
                NavigationDirection::Next
            } else {
                NavigationDirection::Previous
            }),
            Key::Up => Some(NavigationDirection::Up),
            Key::Down => Some(NavigationDirection::Down),
            Key::Left => Some(NavigationDirection::Left),
            Key::Right => Some(NavigationDirection::Right),
            Key::Home => Some(NavigationDirection::First),
            Key::End => Some(NavigationDirection::Last),
            Key::PageUp => Some(NavigationDirection::PageUp),
            Key::PageDown => Some(NavigationDirection::PageDown),
            _ => None,
        }
    }
}

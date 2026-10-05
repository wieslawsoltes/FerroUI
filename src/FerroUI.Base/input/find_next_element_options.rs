use super::navigation::XYFocusNavigationStrategy;
use super::InputElement;
use crate::{Rect, Ref};

/// Provides options to customize the behavior when identifying the next
/// element to focus during a navigation operation.
#[derive(Clone, Default)]
pub struct FindNextElementOptions {
    /// The element that will be treated as the starting point of the search
    /// for the next focusable element. This does not need to be the element
    /// that currently has focus. If `None`, the focus manager's current
    /// focus is used.
    pub focused_element: Option<Ref<InputElement>>,

    /// The root within which the search for the next focusable element will
    /// be conducted.
    ///
    /// This property defines the boundary for focus navigation operations.
    /// It determines the root element in the visual tree under which the
    /// focusable item search is performed. If not specified, the search
    /// defaults to the current scope.
    pub search_root: Option<Ref<InputElement>>,

    /// The rectangular region within the visual hierarchy that will be
    /// excluded from consideration during focus navigation.
    pub exclusion_rect: Rect,

    /// A rectangular region that acts as a hint for focus navigation: a
    /// preferred or prioritized target when navigating focus. Can be `None`
    /// if no specific hint region is provided.
    pub focus_hint_rectangle: Option<Rect>,

    /// The navigation strategy to apply when navigating focus between
    /// elements; overrides the default strategy when set.
    pub navigation_strategy_override: Option<XYFocusNavigationStrategy>,

    /// Whether occlusivity (overlapping of elements) should be ignored
    /// during focus navigation. When true, the navigation logic disregards
    /// obstructions that may block a potential focus target.
    pub ignore_occlusivity: bool,
}

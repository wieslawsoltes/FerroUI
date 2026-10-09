use super::text_input::ITextInputMethodImpl;
use super::{FocusManager, InputElement, WindowDecorationsElementRole};
use crate::{Point, Ref};
use std::rc::Rc;

/// Defines the interface for top-level input elements: the part of a
/// presentation source that input is delivered through.
pub trait IInputRoot {
    /// The focus manager of the root.
    fn focus_manager(&self) -> Option<Rc<FocusManager>>;

    /// The input element that the pointer is currently over.
    fn pointer_over_element(&self) -> Option<Ref<InputElement>>;

    /// Sets the input element that the pointer is currently over.
    fn set_pointer_over_element(&self, value: Option<Ref<InputElement>>);

    /// The element whose cursor is shown.
    fn cursor_element(&self) -> Option<Ref<InputElement>>;

    /// Sets the element whose cursor is shown.
    fn set_cursor_element(&self, value: Option<Ref<InputElement>>);

    /// The input method of the platform for this root, if it has one.
    ///
    /// A root that supports text input methods overrides this; the default
    /// is a root without one.
    fn input_method(&self) -> Option<Rc<dyn ITextInputMethodImpl>> {
        None
    }

    /// The root input element.
    fn root_element(&self) -> Ref<InputElement>;

    /// Deviation (DEVIATIONS.md, Input): the optional form of
    /// [`root_element`](Self::root_element).
    ///
    /// The root input element, or `None` once the root has closed: what the
    /// reference reads as a null `RootElement`, for the code that runs while
    /// an event that closed its root is still being processed.
    fn try_root_element(&self) -> Option<Ref<InputElement>> {
        Some(self.root_element())
    }

    /// The element keyboard input is sent to when nothing is focused.
    fn focus_root(&self) -> Ref<InputElement>;

    /// Performs a hit-test for chrome/decoration elements at the given
    /// position, in root-relative coordinates.
    ///
    /// Returns `None` if no chrome element was hit (no chrome involvement
    /// at this point),
    /// [`WindowDecorationsElementRole::DecorationsElement`] or
    /// [`WindowDecorationsElementRole::User`] if an interactive chrome
    /// element was hit — the platform should redirect non-client input to
    /// regular client input. Any other value different from
    /// [`WindowDecorationsElementRole::None`] indicates a specific
    /// non-client role (titlebar, resize grip, etc.).
    fn hit_test_chrome_element(&self, _point: Point) -> Option<WindowDecorationsElementRole> {
        None
    }

    /// Called when what is under the pointer may have changed without the
    /// pointer moving, e.g. when the capture of a pointer changes.
    ///
    /// The root re-evaluates the pointer-over element. This is a dispatcher
    /// seam: a host should update the pointer-over state from here, either
    /// immediately or posted to its dispatcher.
    fn pointer_over_invalidated(&self);
}

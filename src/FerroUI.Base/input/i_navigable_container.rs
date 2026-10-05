use super::{InputElement, NavigationDirection};
use crate::Ref;

/// Defines a container in which the child controls can be navigated by
/// keyboard.
pub trait INavigableContainer {
    /// Gets the next control in the specified direction.
    ///
    /// `from` is the control from which movement begins and `wrap` tells
    /// whether to wrap around when the first or last item is reached.
    fn get_control(
        &self,
        direction: NavigationDirection,
        from: Option<&Ref<InputElement>>,
        wrap: bool,
    ) -> Option<Ref<InputElement>>;
}

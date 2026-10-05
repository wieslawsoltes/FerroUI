use super::StyleBase;
use crate::Ref;

/// Represents a style that has been instanced on a control.
pub trait IStyleInstance {
    /// The source style, if it is still alive.
    fn source(&self) -> Option<Ref<StyleBase>>;

    /// Whether this style has an activator.
    ///
    /// A style instance without an activator will always be active.
    fn has_activator(&self) -> bool;

    /// Whether this style is active.
    fn is_active(&self) -> bool;
}

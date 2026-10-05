//! `ICommandBarElement`: an element of a command bar, as the bar sees it.
//!
//! A class cannot implement the trait itself (its methods would shadow the
//! inherent ones of the class): it provides a small adapter handle that
//! does, reached through its `as_command_bar_element` method (and, for
//! untyped code, through the interface conversion its class states).

use ferroui_base::FerroObject;

/// Interface implemented by all command bar elements.
pub trait ICommandBarElement {
    /// Gets whether the element is in compact mode (icon only, no label).
    fn is_compact(&self) -> bool;

    /// Sets whether the element is in compact mode (icon only, no label).
    fn set_is_compact(&self, value: bool);

    /// Gets whether the element is currently displayed inside the overflow
    /// popup.
    fn is_in_overflow(&self) -> bool;

    /// Sets whether the element is currently displayed inside the overflow
    /// popup. Set automatically by the command bar when moving items
    /// between primary and overflow.
    fn set_is_in_overflow(&self, value: bool);

    /// The object behind the handle, when the interface is implemented by a
    /// class: what the type tests of the command bar look at, and the
    /// identity of the element.
    fn as_object(&self) -> Option<&FerroObject> {
        None
    }
}

/// Reference equality of the managed interface references: two handles are
/// equal when they are views of the same object.
impl PartialEq for dyn ICommandBarElement {
    fn eq(&self, other: &Self) -> bool {
        match (self.as_object(), other.as_object()) {
            (Some(a), Some(b)) => std::ptr::eq(a, b),
            (None, None) => std::ptr::addr_eq(self, other),
            _ => false,
        }
    }
}

impl std::fmt::Debug for dyn ICommandBarElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.as_object() {
            Some(object) => write!(f, "ICommandBarElement({})", object.get_type()),
            None => f.write_str("ICommandBarElement"),
        }
    }
}

//! Port of `Metadata/IAddChild.cs`.

/// Defines a method for adding a child of type `T` to an object: what markup
/// calls for each child element of an element whose type implements it.
pub trait IAddChild<T> {
    /// Adds a child.
    fn add_child(&self, child: T);

    /// Identifies the object children are added to. Handles are equal when
    /// they add to the same object; an adapter that implements the contract
    /// for an object behind a class handle returns the address of that
    /// object, so that two handles made from the same object are equal.
    fn reference_id(&self) -> usize {
        self as *const Self as *const () as usize
    }
}

/// Handles compare by the identity of the object they add to, so that they
/// can be held in untyped values.
impl<'a, T> PartialEq for dyn IAddChild<T> + 'a {
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}

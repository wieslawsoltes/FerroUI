use crate::FerroProperty;

/// The classes of the framework whose `on_property_changed` says which properties it reacts
/// to, so that a change of any other property does not look for them.
///
/// The handler of such a class compares the property that changed with the properties of the
/// class, one after the other, and each comparison first finds the property it compares with.
/// Most changes are of a property none of these classes reacts to: the bounds of an element
/// after every arrange, the text of a text block. A class of this list names its properties
/// once, in its static constructor, and its handler asks the change before it compares:
///
/// ```ignore
/// impl Control {
///     fn static_constructor() {
///         PropertyChangeHandler::Control.reacts_to(&[Visual::bounds_property().as_property()]);
///     }
/// }
///
/// fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
///     Self::parent_on_property_changed(this, change);
///     if !change.reaches(PropertyChangeHandler::Control) {
///         return;
///     }
///     if change.is_for(PropertyChangeHandler::Control, Visual::bounds_property().as_property()) {
///         // ...
///     }
/// }
/// ```
///
/// What a class names is kept with the property and shared with the properties added to
/// other owners, which are the same property. A handler that is not in this list is called
/// for every change, as before, and so is every class handler and every listener of a
/// property: the list is about the comparisons inside `on_property_changed` alone.
///
/// An addition (FerroUI deviation, desktop-performance.md section 9.59).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PropertyChangeHandler {
    /// `Animatable::on_property_changed_core`: the transitions of the element.
    Animatable,
    /// `StyledElement::on_property_changed`.
    StyledElement,
    /// `Visual::on_property_changed`.
    Visual,
    /// `Layoutable::on_property_changed`.
    Layoutable,
    /// `InputElement::on_property_changed`.
    InputElement,
    /// `Control::on_property_changed`.
    Control,
}

impl PropertyChangeHandler {
    #[inline]
    pub(crate) const fn bit(self) -> u32 {
        1 << self as u32
    }

    /// Says that the handler of the class reacts to a change of these properties, and of no
    /// other. Called from the static constructor of the class, once for all of them or once
    /// for each; a property named twice is named once.
    ///
    /// A handler that compares with a property it did not name fails an assertion in a debug
    /// build ([`FerroPropertyChangedEventArgs::is_for`](crate::FerroPropertyChangedEventArgs::is_for)).
    pub fn reacts_to(self, properties: &[&'static FerroProperty]) {
        for property in properties {
            property.add_change_handlers(self.bit());
        }
    }
}

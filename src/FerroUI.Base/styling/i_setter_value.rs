use super::Setter;

/// Customizes the behavior of a class when added as a value to a
/// [`Setter`].
///
/// An untyped setter value carries no interface information, so
/// `initialize` is only invoked for values handed to
/// [`Setter::set_setter_value`].
pub trait ISetterValue {
    /// Notifies that the object has been added as a setter value.
    fn initialize(&self, setter: &Setter);
}

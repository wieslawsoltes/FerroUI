/// Defines the method by which focus was changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NavigationMethod {
    /// The focus was changed by an unspecified method, e.g. calling
    /// `InputElement::focus`.
    #[default]
    Unspecified,
    /// The focus was changed by the user tabbing between control.
    Tab,
    /// The focus was changed by the user pressing a directional navigation
    /// key.
    Directional,
    /// The focus was changed by a pointer click.
    Pointer,
}

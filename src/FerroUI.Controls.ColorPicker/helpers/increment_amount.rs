// This source file is adapted from the WinUI project.
// (https://github.com/microsoft/microsoft-ui-xaml)

/// Defines a relative amount that a color component should be incremented.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum IncrementAmount {
    /// A smaller change in value.
    Small,

    /// A larger change in value.
    Large,
}

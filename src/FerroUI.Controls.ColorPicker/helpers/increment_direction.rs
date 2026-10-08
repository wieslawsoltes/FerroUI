// This source file is adapted from the WinUI project.
// (https://github.com/microsoft/microsoft-ui-xaml)

/// Defines the direction a color component should be incremented.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum IncrementDirection {
    /// Decreasing in value towards zero.
    Lower,

    /// Increasing in value towards positive infinity.
    Higher,
}

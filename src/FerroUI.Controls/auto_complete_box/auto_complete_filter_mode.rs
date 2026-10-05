// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.

/// Specifies how text in the text box portion of the
/// [`AutoCompleteBox`](super::AutoCompleteBox) control is used to filter
/// items specified by the `ItemsSource` property for display in the
/// drop-down.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AutoCompleteFilterMode {
    /// Specifies that no filter is used. All items are returned.
    None = 0,

    /// Specifies a culture-sensitive, case-insensitive filter where the
    /// returned items start with the specified text. The filter uses the
    /// current-culture, case-insensitive comparer.
    StartsWith = 1,

    /// Specifies a culture-sensitive, case-sensitive filter where the
    /// returned items start with the specified text. The filter uses the
    /// current-culture comparer.
    StartsWithCaseSensitive = 2,

    /// Specifies an ordinal, case-insensitive filter where the returned
    /// items start with the specified text. The filter uses the ordinal,
    /// case-insensitive comparer.
    StartsWithOrdinal = 3,

    /// Specifies an ordinal, case-sensitive filter where the returned items
    /// start with the specified text. The filter uses the ordinal comparer.
    StartsWithOrdinalCaseSensitive = 4,

    /// Specifies a culture-sensitive, case-insensitive filter where the
    /// returned items contain the specified text.
    Contains = 5,

    /// Specifies a culture-sensitive, case-sensitive filter where the
    /// returned items contain the specified text.
    ContainsCaseSensitive = 6,

    /// Specifies an ordinal, case-insensitive filter where the returned
    /// items contain the specified text.
    ContainsOrdinal = 7,

    /// Specifies an ordinal, case-sensitive filter where the returned items
    /// contain the specified text.
    ContainsOrdinalCaseSensitive = 8,

    /// Specifies a culture-sensitive, case-insensitive filter where the
    /// returned items equal the specified text. The filter uses the
    /// current-culture, case-insensitive comparer.
    Equals = 9,

    /// Specifies a culture-sensitive, case-sensitive filter where the
    /// returned items equal the specified text. The filter uses the
    /// current-culture comparer.
    EqualsCaseSensitive = 10,

    /// Specifies an ordinal, case-insensitive filter where the returned
    /// items equal the specified text. The filter uses the ordinal,
    /// case-insensitive comparer.
    EqualsOrdinal = 11,

    /// Specifies an ordinal, case-sensitive filter where the returned items
    /// equal the specified text. The filter uses the ordinal comparer.
    EqualsOrdinalCaseSensitive = 12,

    /// Specifies that a custom filter is used. This mode is used when the
    /// `TextFilter` or `ItemFilter` properties are set.
    Custom = 13,
}

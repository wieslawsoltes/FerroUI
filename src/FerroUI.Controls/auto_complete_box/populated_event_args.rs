// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.

use crate::ItemsSource;

/// Provides data for the `Populated` event of the
/// [`AutoCompleteBox`](super::AutoCompleteBox).
#[derive(Clone, PartialEq)]
pub struct PopulatedEventArgs {
    data: ItemsSource,
}

impl PopulatedEventArgs {
    /// Initializes a new instance of the [`PopulatedEventArgs`].
    ///
    /// `data` is the list of possible matches added to the drop-down
    /// portion of the control.
    pub fn new(data: ItemsSource) -> Self {
        Self { data }
    }

    /// Gets the list of possible matches added to the drop-down portion of
    /// the control.
    pub fn data(&self) -> &ItemsSource {
        &self.data
    }
}

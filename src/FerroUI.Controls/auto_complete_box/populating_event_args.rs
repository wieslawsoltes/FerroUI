// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.

use ferroui_base::utilities::CancelEventArgs;
use std::ops::Deref;

/// Provides data for the `Populating` event of the
/// [`AutoCompleteBox`](super::AutoCompleteBox).
///
/// A copy shares the cancel flag of the args it was made from.
#[derive(Clone, PartialEq)]
pub struct PopulatingEventArgs {
    base: CancelEventArgs,
    parameter: Option<String>,
}

impl PopulatingEventArgs {
    /// Initializes a new instance of the [`PopulatingEventArgs`].
    ///
    /// `parameter` is the value of the `SearchText` property, which is
    /// used to filter items for the control.
    pub fn new(parameter: Option<String>) -> Self {
        Self { base: CancelEventArgs::new(), parameter }
    }

    /// Gets the text that is used to determine which items to display in
    /// the control.
    pub fn parameter(&self) -> Option<&str> {
        self.parameter.as_deref()
    }
}

impl Deref for PopulatingEventArgs {
    type Target = CancelEventArgs;

    fn deref(&self) -> &CancelEventArgs {
        &self.base
    }
}

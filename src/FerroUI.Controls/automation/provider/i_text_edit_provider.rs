// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the crate root for license information.

use super::{ITextProvider, ITextRangeProvider, ProviderError};
use ferroui_base::reactive::IDisposable;
use std::rc::Rc;

/// The causes carried by WinUI's text-edit notification. An ordinary edit has
/// no inferred cause: the editor publishes a cause only when it performs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TextEditChangeType {
    None = 0,
    AutoCorrect = 1,
    Composition = 2,
    CompositionFinalized = 3,
}
ferroui_base::ferro_markup_enum!(TextEditChangeType {
    None,
    AutoCorrect,
    Composition,
    CompositionFinalized,
}, { namespace: "FerroUI.Automation" });

/// Text accepted by the editor for a specific programmatic editing action.
/// Composition data contains only the replacement substring; finalized data
/// contains the completed composition, or an empty string for cancellation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEditTextChangedEventArgs {
    pub change_type: TextEditChangeType,
    pub changed_data: Vec<String>,
}

/// WinUI's TextEdit pattern, which inherits Text independently of Text2.
/// Returned ranges use the same live document and UTF-16 positions as Text.
pub trait ITextEditProvider: ITextProvider {
    /// Whether this provider's owner already sends its causes through
    /// AutomationPeer's platform event route. Backends then avoid installing a
    /// second forwarding observer. Custom providers keep the default route.
    fn raises_text_edit_events_through_peer(&self) -> bool {
        false
    }

    /// The full active IME preview, regardless of the caret within it. A live
    /// owner with no active composition returns `None`.
    fn get_active_composition(&self) -> Result<Option<Rc<dyn ITextRangeProvider>>, ProviderError>;

    /// The IME's distinct conversion target, when the input backend supplies
    /// one. A composition or selection alone does not identify a target.
    fn get_conversion_target(&self) -> Result<Option<Rc<dyn ITextRangeProvider>>, ProviderError>;

    /// Observes the model's known text-edit causes, without interpreting
    /// ordinary text/selection changes as autocorrection or composition.
    fn text_edit_text_changed(
        &self,
        handler: Rc<dyn Fn(&TextEditTextChangedEventArgs)>,
    ) -> Result<Rc<dyn IDisposable>, ProviderError>;
}

impl PartialEq for dyn ITextEditProvider {
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}

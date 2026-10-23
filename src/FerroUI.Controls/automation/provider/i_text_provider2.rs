// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the crate root for license information.

use super::{ITextProvider, ITextRangeProvider, ProviderError};
use crate::automation::peers::AutomationPeer;
use ferroui_base::Ref;
use std::rc::Rc;

/// The insertion point and whether its text control currently has keyboard focus.
/// `range` is absent when the control does not support a text insertion point.
/// An inactive caret still has a range; activity does not follow caret blinking.
pub struct TextCaretRange {
    pub is_active: bool,
    pub range: Option<Rc<dyn ITextRangeProvider>>,
}

/// The additional operations of the WinUI/UI Automation Text2 pattern.
/// Text ranges use the same live document and UTF-16 coordinates as Text.
pub trait ITextProvider2: ITextProvider {
    fn range_from_annotation(
        &self,
        annotation: &Ref<AutomationPeer>,
    ) -> Result<Rc<dyn ITextRangeProvider>, ProviderError>;
    fn get_caret_range(&self) -> Result<TextCaretRange, ProviderError>;
}

impl PartialEq for dyn ITextProvider2 {
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}

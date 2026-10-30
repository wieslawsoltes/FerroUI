// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the crate root for license information.

use super::{ITextRangeProvider, ProviderError};
use crate::automation::peers::AutomationPeer;
use ferroui_base::Ref;
use std::rc::Rc;

/// A child embedded in a text document. Its container owns the Text pattern;
/// its live range follows the child's current position in that document.
pub trait ITextChildProvider {
    fn peer(&self) -> Ref<AutomationPeer>;
    fn text_container(&self) -> Result<Ref<AutomationPeer>, ProviderError>;
    fn text_range(&self) -> Result<Rc<dyn ITextRangeProvider>, ProviderError>;
}

impl PartialEq for dyn ITextChildProvider {
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}

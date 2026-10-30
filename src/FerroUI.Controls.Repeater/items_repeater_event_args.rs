// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.

use ferroui_controls::Control;
use ferroui_base::Ref;

/// Raised after an element has been prepared and attached to its repeater.
#[derive(Clone, PartialEq)]
pub struct ItemsRepeaterElementPreparedEventArgs {
    pub element: Ref<Control>,
    pub index: i32,
}

/// Raised before a realized element is returned to its element factory.
#[derive(Clone, PartialEq)]
pub struct ItemsRepeaterElementClearingEventArgs {
    pub element: Ref<Control>,
}

/// Raised when an existing element acquires a different source index.
#[derive(Clone, PartialEq)]
pub struct ItemsRepeaterElementIndexChangedEventArgs {
    pub element: Ref<Control>,
    pub old_index: i32,
    pub new_index: i32,
}

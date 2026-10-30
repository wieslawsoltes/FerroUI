// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::controls::*;
use crate::core::*;
use crate::BoxedValue;
use std::rc::Rc;

/// The `IToolDockContent` docking contract.
pub trait IToolDockContent: IDock {
    /// Gets or sets tool template.
    fn tool_template(&self) -> Option<Rc<dyn IToolTemplate>>;
    /// Sets `ToolTemplate`.
    fn set_tool_template(&self, value: Option<Rc<dyn IToolTemplate>>);
    /// Create new tool from template.
    fn create_tool_from_template(&self) -> Option<BoxedValue>;
}

impl PartialEq for dyn IToolDockContent {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

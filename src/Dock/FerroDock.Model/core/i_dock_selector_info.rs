// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Customization of a dockable's appearance in the document/panel selector.
pub trait IDockSelectorInfo {
    /// Returns whether the dockable appears in the selector.
    fn show_in_selector(&self) -> bool;
    /// Sets whether the dockable appears in the selector.
    fn set_show_in_selector(&self, value: bool);
    /// Gets the optional selector title override.
    fn selector_title(&self) -> Option<String>;
    /// Sets the optional selector title override.
    fn set_selector_title(&self, value: Option<String>);
}

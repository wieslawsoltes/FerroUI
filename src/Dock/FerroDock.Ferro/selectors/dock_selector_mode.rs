// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
/// Defines the selector scope for dockables.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DockSelectorMode {
    /// Show documents only.
    #[default]
    Documents = 0,
    /// Show tools only.
    Tools = 1,
    /// Show documents and tools.
    All = 2,
}
ferroui_base::ferro_markup_enum!(DockSelectorMode {Documents,Tools,All},{namespace:"FerroDock.Ferro.Selectors"});

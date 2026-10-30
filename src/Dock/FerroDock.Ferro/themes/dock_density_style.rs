// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
/// Original density presets for docking control themes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DockDensityStyle {
    /// Default control sizing.
    #[default]
    Normal = 0,
    /// Compact sizing for tighter layouts.
    Compact = 1,
}
ferroui_base::ferro_markup_enum!(DockDensityStyle{Normal,Compact},{namespace:"FerroDock.Ferro.Themes"});

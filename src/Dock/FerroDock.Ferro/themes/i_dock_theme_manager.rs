// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::collections::ReadOnlyListHandle;
/// Original theme and preset switching contract for docking applications.
pub trait IDockThemeManager {
    /// Selects the application light/dark variant (zero = light, one = dark).
    fn switch(&self, index: i32);
    /// Gets the shared source list of preset display names.
    fn preset_names(&self) -> ReadOnlyListHandle<String>;
    /// Gets the active preset index.
    fn current_preset_index(&self) -> i32;
    /// Selects a preset by index.
    fn switch_preset(&self, index: i32);
}

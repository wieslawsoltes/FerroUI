// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Defaults used by MDI layout helpers and controls.
pub struct MdiLayoutDefaults;

impl MdiLayoutDefaults {
    /// Default cascade offset between windows.
    pub const CASCADE_OFFSET: f64 = 24.0;
    /// Default minimized window height.
    pub const MINIMIZED_HEIGHT: f64 = 28.0;
    /// Default minimized window width.
    pub const MINIMIZED_WIDTH: f64 = 220.0;
    /// Default spacing between minimized windows.
    pub const MINIMIZED_SPACING: f64 = 4.0;
    /// Default width ratio for cascaded windows.
    pub const DEFAULT_WIDTH_RATIO: f64 = 0.6;
    /// Default height ratio for cascaded windows.
    pub const DEFAULT_HEIGHT_RATIO: f64 = 0.6;
    /// Minimum width for cascaded windows.
    pub const MINIMUM_WIDTH: f64 = 200.0;
    /// Minimum height for cascaded windows.
    pub const MINIMUM_HEIGHT: f64 = 120.0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_preserve_mdi_layout_dimensions() {
        assert_eq!(MdiLayoutDefaults::CASCADE_OFFSET, 24.0);
        assert_eq!(MdiLayoutDefaults::MINIMIZED_HEIGHT, 28.0);
        assert_eq!(MdiLayoutDefaults::MINIMIZED_WIDTH, 220.0);
        assert_eq!(MdiLayoutDefaults::MINIMIZED_SPACING, 4.0);
        assert_eq!(MdiLayoutDefaults::DEFAULT_WIDTH_RATIO, 0.6);
        assert_eq!(MdiLayoutDefaults::DEFAULT_HEIGHT_RATIO, 0.6);
        assert_eq!(MdiLayoutDefaults::MINIMUM_WIDTH, 200.0);
        assert_eq!(MdiLayoutDefaults::MINIMUM_HEIGHT, 120.0);
    }
}

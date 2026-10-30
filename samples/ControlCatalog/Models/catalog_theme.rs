//! Port of `Models/CatalogTheme.cs`, with the WinUI theme of the port as a third theme
//! (`WinUI`, not in the original; docs/porting/winui-theme.md).

use ferroui_base::ferro_markup_enum;

/// The theme of the catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CatalogTheme {
    Fluent = 0,
    Simple = 1,
    /// The WinUI theme (`ferroui-themes-winui`), an addition of the port.
    WinUI = 2,
    /// The macOS 26 theme, available with the `macos-theme` feature.
    MacOS = 3,
    /// The Android Material 3 Expressive theme, available with the `material-theme` feature.
    Material = 4,
}

ferro_markup_enum!(CatalogTheme { Fluent, Simple, WinUI, MacOS, Material });

impl CatalogTheme {
    /// The build feature of the catalog that includes the theme (WinUI is always included).
    pub fn build_feature(self) -> &'static str {
        match self {
            CatalogTheme::Fluent | CatalogTheme::Simple => "classic-themes",
            CatalogTheme::WinUI => "",
            CatalogTheme::MacOS => "macos-theme",
            CatalogTheme::Material => "material-theme",
        }
    }
}

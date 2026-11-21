//! Native theme dictionaries and runtime palette/subtree theme service.
mod shared_theme_include;
#[doc(hidden)]
pub use shared_theme_include::RibbonSharedThemeInclude;
mod ribbon_theme_resources;
pub use ribbon_theme_resources::{
    RibbonThemeResourceError, RibbonThemeResources, to_color, to_ribbon_color,
};

mod ribbon_theme;
pub use ribbon_theme::{RibbonTheme, RibbonThemeApplyError, RibbonThemeError};

pub use ferroribbon_core::theming::{
    RibbonChromeStyle, RibbonColor, RibbonThemePalette, RibbonThemeStyle,
};

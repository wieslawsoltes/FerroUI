//! The controls of this crate that only the WinUI theme and the themes derived from it (macOS 26,
//! Material) style (docs/porting/winui-controls.md, section 5.1): the WinUI controls FerroUI ports
//! as controls, whose templates are ported from WinUI into `ferroui-themes-winui` and adapted by
//! the derived themes. The Fluent and the Simple themes have no control theme for them, and their
//! tests check that they have none; the table of section 5.1 lists the same controls
//! (`winui_only_tests`).

use crate::{FontIcon, SymbolIcon};
use ferroui_base::TypeInfo;

/// The controls only the WinUI theme and the themes derived from it style, each with the reason.
pub const WINUI_ONLY_CONTROLS: &[(&TypeInfo, &str)] = &[
    (FontIcon::TYPE, "WinUI's FontIcon: its text part is in the WinUI-derived themes only"),
    (SymbolIcon::TYPE, "WinUI's SymbolIcon: its text part is in the WinUI-derived themes only"),
];

/// Whether only the WinUI theme and the themes derived from it style `control`.
pub fn is_winui_only(control: &TypeInfo) -> bool {
    WINUI_ONLY_CONTROLS.iter().any(|(candidate, _)| std::ptr::eq(*candidate, control))
}

#[cfg(test)]
mod winui_only_tests {
    use super::WINUI_ONLY_CONTROLS;

    /// The table of section 5.1 of docs/porting/winui-controls.md names the same controls.
    #[test]
    fn the_documentation_lists_the_same_controls() {
        let doc = include_str!("../../../docs/porting/winui-controls.md");
        let start = doc.find("<!-- winui-only:begin -->").expect("the begin marker of the table");
        let end = doc.find("<!-- winui-only:end -->").expect("the end marker of the table");
        let documented: Vec<&str> = doc[start..end]
            .lines()
            .filter_map(|line| line.strip_prefix("| `"))
            .filter_map(|line| line.split('`').next())
            .collect();
        let listed: Vec<&str> = WINUI_ONLY_CONTROLS.iter().map(|(control, _)| control.name()).collect();
        assert_eq!(listed, documented);
    }
}

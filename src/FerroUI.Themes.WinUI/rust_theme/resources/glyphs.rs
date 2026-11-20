//! `Resources/Glyphs.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! Not ported from a document: the outlines the port draws where the templates of microsoft-ui-xaml
//! (https://github.com/microsoft/microsoft-ui-xaml, commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0)
//! show a glyph of the symbol font (FontIcon, FontIconSource or a TextBlock in
//! SymbolThemeFontFamily, "Segoe Fluent Icons,Segoe MDL2 Assets"). The font is part of Windows,
//! is not redistributable and does not exist on the other platforms, so a template of the port
//! draws the glyph as a path of the geometry GlyphGeometry<code point> (docs/porting/winui-theme.md,
//! section 4.7), sized to the ink of the glyph at the font size of the source.
//!
//! GlyphGeometryE73E (CheckMark) and GlyphGeometryE894 (Clear) are the outlines of the Fluent theme
//! of FerroUI (src/FerroUI.Themes.Fluent/Controls/CheckBox.xaml, TextBox.xaml), and so are
//! GlyphGeometryE700 (GlobalNavigationButton) and GlyphGeometryE72B (Back)
//! (src/FerroUI.Themes.Fluent/Controls/NavigationPage.xaml). The others are drawn for the port.
//!
//! NavigationPageMenuIcon is not a glyph of WinUI: it is the key FerroUI's NavigationPage looks up
//! in code for the icon of its drawer toggle, here the GlobalNavigationButton glyph that WinUI's
//! pane toggle button shows (PaneToggleButtonStyle).

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::media::StreamGeometry;

/// Adds the resources of the document to `resources`.
pub(crate) fn merge(resources: &Resources) -> Result {
    // E73E CheckMark: the check of CheckBox (CheckBoxCheckedGlyph).
    resources.deferred("GlyphGeometryE73E", |_| Ok(value(literal(StreamGeometry::parse("M5.5 10.586 1.707 6.793A1 1 0 0 0 .293 8.207l4.5 4.5a1 1 0 0 0 1.414 0l11-11A1 1 0 0 0 15.793.293L5.5 10.586Z"))?)));
    // E9AE: the bar of an indeterminate CheckBox (CheckBoxIndeterminateGlyph).
    resources.deferred("GlyphGeometryE9AE", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M0.75 0h10.5a0.75 0.75 0 0 1 0 1.5H0.75a0.75 0.75 0 0 1 0-1.5Z",
        ))?))
    });
    // E894 Clear: the delete button of TextBox.
    resources.deferred("GlyphGeometryE894", |_| Ok(value(literal(StreamGeometry::parse("M 11.416016,10 20,1.4160156 18.583984,0 10,8.5839846 1.4160156,0 0,1.4160156 8.5839844,10 0,18.583985 1.4160156,20 10,11.416015 18.583984,20 20,18.583985 Z"))?)));
    // EDD9 CaretLeftSolid8, EDDA CaretRightSolid8, EDDB CaretUpSolid8, EDDC CaretDownSolid8: the
    //       arrows of the line buttons of ScrollBar, of the navigation buttons of the calendar, of the
    //       looping selectors of the pickers, of the scroll buttons of TabView and of PipsPager. A solid
    //       triangle with rounded corners whose ink at the font size 8 is 8 wide and 6 high (up and
    //       down; 6 wide and 8 high to the sides), as native WinUI 3 draws it (measured at a scaling of
    //       2: 16 by 12 device pixels).
    resources.deferred("GlyphGeometryEDD9", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M 0.6,4.4 L 5.28,7.52 Q 6,8 6,7.2 L 6,0.8 Q 6,0 5.28,0.48 L 0.6,3.6 Q 0,4 0.6,4.4 Z",
        ))?))
    });
    resources.deferred("GlyphGeometryEDDA", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M 5.4,4.4 L 0.72,7.52 Q 0,8 0,7.2 L 0,0.8 Q 0,0 0.72,0.48 L 5.4,3.6 Q 6,4 5.4,4.4 Z",
        ))?))
    });
    resources.deferred("GlyphGeometryEDDB", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M 4.4,0.6 L 7.52,5.28 Q 8,6 7.2,6 L 0.8,6 Q 0,6 0.48,5.28 L 3.6,0.6 Q 4,0 4.4,0.6 Z",
        ))?))
    });
    resources.deferred("GlyphGeometryEDDC", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M 4.4,5.4 L 7.52,0.72 Q 8,0 7.2,0 L 0.8,0 Q 0,0 0.48,0.72 L 3.6,5.4 Q 4,6 4.4,5.4 Z",
        ))?))
    });
    // F136 StatusCircleOuter and F13D StatusCircleErrorX: the error icon of InfoBar
    //       (InfoBarIconBackgroundGlyph under InfoBarErrorIconGlyph), the icon of the keyed theme
    //       TooltipDataValidationErrors. F136 is the whole circle of the icon; F13D is the cross inside
    //       it.
    resources.deferred("GlyphGeometryF136", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M 8,0 A 8,8 0 1 1 8,16 A 8,8 0 1 1 8,0 Z",
        ))?))
    });
    resources.deferred("GlyphGeometryF13D", |_| Ok(value(literal(StreamGeometry::parse("M 0,1.41 L 1.41,0 L 5,3.59 L 8.59,0 L 10,1.41 L 6.41,5 L 10,8.59 L 8.59,10 L 5,6.41 L 1.41,10 L 0,8.59 L 3.59,5 Z"))?)));
    // E711 Cancel: the dismiss button of the date and time picker flyouts (DatePickerFlyoutPresenter,
    //       TimePickerFlyoutPresenter) and InfoBarCloseButtonSymbol, the close button of InfoBar (NotificationCard).
    resources.deferred("GlyphGeometryE711", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M 0.7,0 6,5.3 11.3,0 12,0.7 6.7,6 12,11.3 11.3,12 6,6.7 0.7,12 0,11.3 5.3,6 0,0.7 Z",
        ))?))
    });
    // E787 Calendar: the glyph of CalendarDatePicker, drawn after the ink of the glyph at 12 px
    //       measured against native WinUI 3 at 200 % (a 16 unit box is the 10.5 px of the ink): a frame
    //       with rounded corners (2.67 units outside, a stroke of 1.1), the line under its header 3.25
    //       units below its top, and the dates, three over two, squares of 1.9 units 3.37 apart.
    //       Filled even-odd.
    resources.deferred("GlyphGeometryE787", |_| Ok(value(literal(StreamGeometry::parse("F0 M 2.67,0 H 13.33 A 2.67,2.67 0 0 1 16,2.67 V 13.33 A 2.67,2.67 0 0 1 13.33,16 H 2.67 A 2.67,2.67 0 0 1 0,13.33 V 2.67 A 2.67,2.67 0 0 1 2.67,0 Z M 2.67,1.1 A 1.57,1.57 0 0 0 1.1,2.67 V 13.33 A 1.57,1.57 0 0 0 2.67,14.9 H 13.33 A 1.57,1.57 0 0 0 14.9,13.33 V 2.67 A 1.57,1.57 0 0 0 13.33,1.1 Z M 1.1,3.25 H 14.9 V 4.55 H 1.1 Z M 3.68,7.05 H 5.58 V 8.95 H 3.68 Z M 7.05,7.05 H 8.95 V 8.95 H 7.05 Z M 10.42,7.05 H 12.32 V 8.95 H 10.42 Z M 3.68,10.42 H 5.58 V 12.32 H 3.68 Z M 7.05,10.42 H 8.95 V 12.32 H 7.05 Z"))?)));
    // E8FB Accept: the accept button of the date and time picker flyouts.
    resources.deferred("GlyphGeometryE8FB", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M 0,7 0.7,6.3 5,10.6 15.3,0.3 16,1 5,12 Z",
        ))?))
    });
    // E921 ChromeMinimize, E922 ChromeMaximize, E923 ChromeRestore, E8BB ChromeClose, E740
    //       FullScreen, E73F BackToWindow: the caption buttons of WindowDrawnDecorations. The outlines of
    //       the caption buttons of the Fluent theme of FerroUI
    //       (src/FerroUI.Themes.Fluent/Controls/WindowDrawnDecorations.xaml), in the 2048 units of the em
    //       of the symbol font, with the y axis of the font turned down.
    resources.deferred("GlyphGeometryE921", |_| {
        Ok(value(literal(StreamGeometry::parse("M2048 819v205h-2048v-205h2048z"))?))
    });
    resources.deferred("GlyphGeometryE922", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M2048 2048v-2048h-2048v2048h2048zM1843 1843h-1638v-1638h1638v1638z",
        ))?))
    });
    resources.deferred("GlyphGeometryE923", |_| Ok(value(literal(StreamGeometry::parse("M2048 1638h-410v410h-1638v-1638h410v-410h1638v1638zM1434 614h-1229v1229h1229v-1229zM1843 205h-1229v205h1024v1024h205v-1229z"))?)));
    resources.deferred("GlyphGeometryE8BB", |_| Ok(value(literal(StreamGeometry::parse("M1169 1024l879 -879l-145 -145l-879 879l-879 -879l-145 145l879 879l-879 879l145 145l879 -879l879 879l145 -145z"))?)));
    resources.deferred("GlyphGeometryE740", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M2048 0v819h-205v-469l-1493 1493h469v205h-819v-819h205v469l1493 -1493h-469v-205h819z",
        ))?))
    });
    resources.deferred("GlyphGeometryE73F", |_| Ok(value(literal(StreamGeometry::parse("M205 1024h819v819h-205v-469l-674 674l-145 -145l674 -674h-469v-205zM1374 819h469v205h-819v-819h205v469l674 -674l145 145z"))?)));
    // E74A Up (the up button of ManagedFileChooser): an arrow of 1 px strokes, 14 x 14. The back
    //       button of TitleBarBackButtonStyle uses E72B and its pane toggle button E700, below.
    resources.deferred("GlyphGeometryE74A", |_| Ok(value(literal(StreamGeometry::parse("F1 M7,0 7,14 6,14 6,0Z M7.207,0.5 0.5,7.207 -0.207,6.5 6.5,-0.207Z M6.5,-0.207 13.207,6.5 12.5,7.207 5.793,0.5Z"))?)));
    // E72C Refresh (the refresh button of ManagedFileChooser): the outline of the Fluent theme of
    //       FerroUI (src/FerroUI.Themes.Fluent/Controls/ManagedFileChooser.xaml).
    resources.deferred("GlyphGeometryE72C", |_| Ok(value(literal(StreamGeometry::parse("M18.62 3.32c.39 0 .7.29.76.66v3c0 .39-.28.7-.66.76h-3a.77.77 0 0 1-.1-1.52h1.08a7.42 7.42 0 1 0 2.65 4.37.77.77 0 1 1 1.5-.3 8.94 8.94 0 1 1-3-5.12V4.09c0-.43.35-.77.77-.77Z"))?)));
    // F136 StatusCircleOuter and the symbols InfoBar draws over it: F13F StatusCircleInfo, F13D
    //       StatusCircleErrorX, F13C StatusCircleExclamation, F13E StatusCircleCheckmark
    //       (InfoBarIconBackgroundGlyph, InfoBarInformationalIconGlyph, InfoBarErrorIconGlyph,
    //       InfoBarWarningIconGlyph, InfoBarSuccessIconGlyph). The circle is 16 x 16; the symbols are
    //       drawn at the size of their ink and centred on it. The check mark is the check mark polygon
    //       of the item chrome of WinUI (CListViewBaseItemChrome::s_checkMarkPoints, 15 x 13).
    resources.deferred("GlyphGeometryF13F", |_| Ok(value(literal(StreamGeometry::parse("M0.75,0 A0.75,0.75 0 1 1 0.75,1.5 A0.75,0.75 0 1 1 0.75,0 Z M0.75,2.75 A0.625,0.625 0 0 1 1.375,3.375 V7.375 A0.625,0.625 0 0 1 0.125,7.375 V3.375 A0.625,0.625 0 0 1 0.75,2.75 Z"))?)));
    resources.deferred("GlyphGeometryF13C", |_| Ok(value(literal(StreamGeometry::parse("M0.75,0 A0.625,0.625 0 0 1 1.375,0.625 V4.625 A0.625,0.625 0 0 1 0.125,4.625 V0.625 A0.625,0.625 0 0 1 0.75,0 Z M0.75,6.5 A0.75,0.75 0 1 1 0.75,8 A0.75,0.75 0 1 1 0.75,6.5 Z"))?)));
    resources.deferred("GlyphGeometryF13E", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M0,7 L2.2,4.3 L6.1,7.9 L12.4,0 L15,2.4 L6.6,13 Z",
        ))?))
    });
    // E8B7 Folder, E7C3 Page, EDA2 HardDrive: the icons of the entries of ManagedFileChooser,
    //       outlines of 1 px (folder 16 x 12, page 13 x 16, drive 16 x 8).
    resources.deferred("GlyphGeometryE8B7", |_| Ok(value(literal(StreamGeometry::parse("M1.5,2 H5.293 A1,1 0 0 1 6,2.293 L7.707,4 H14.5 A1.5,1.5 0 0 1 16,5.5 V12.5 A1.5,1.5 0 0 1 14.5,14 H1.5 A1.5,1.5 0 0 1 0,12.5 V3.5 A1.5,1.5 0 0 1 1.5,2 Z M1,3.5 A0.5,0.5 0 0 1 1.5,3 H5.086 L6.586,4.5 L6.086,5 H1 Z M1,6 H6.5 L7.5,5 H14.5 A0.5,0.5 0 0 1 15,5.5 V12.5 A0.5,0.5 0 0 1 14.5,13 H1.5 A0.5,0.5 0 0 1 1,12.5 Z"))?)));
    resources.deferred("GlyphGeometryE7C3", |_| Ok(value(literal(StreamGeometry::parse("M2.5,0 H9.586 A1.5,1.5 0 0 1 10.646,0.44 L13.56,3.354 A1.5,1.5 0 0 1 14,4.414 V14.5 A1.5,1.5 0 0 1 12.5,16 H2.5 A1.5,1.5 0 0 1 1,14.5 V1.5 A1.5,1.5 0 0 1 2.5,0 Z M2.5,1 A0.5,0.5 0 0 0 2,1.5 V14.5 A0.5,0.5 0 0 0 2.5,15 H12.5 A0.5,0.5 0 0 0 13,14.5 V5 H10.5 A1.5,1.5 0 0 1 9,3.5 V1 Z M10,1.207 V3.5 A0.5,0.5 0 0 0 10.5,4 H12.793 Z"))?)));
    resources.deferred("GlyphGeometryEDA2", |_| Ok(value(literal(StreamGeometry::parse("M2,4 H14 A2,2 0 0 1 16,6 V10 A2,2 0 0 1 14,12 H2 A2,2 0 0 1 0,10 V6 A2,2 0 0 1 2,4 Z M2,5 A1,1 0 0 0 1,6 V10 A1,1 0 0 0 2,11 H14 A1,1 0 0 0 15,10 V6 A1,1 0 0 0 14,5 Z M12,7.25 A0.75,0.75 0 1 1 12,8.75 A0.75,0.75 0 1 1 12,7.25 Z"))?)));
    // E700 GlobalNavigationButton: the pane toggle button of NavigationView (PaneToggleButtonStyle) and of the title bar (TitleBarPaneToggleButtonStyle),
    //       the drawer toggle of DrawerPage and NavigationPage.
    resources.deferred("GlyphGeometryE700", |_| Ok(value(literal(StreamGeometry::parse("M3 17h18a1 1 0 0 1 .117 1.993L21 19H3a1 1 0 0 1-.117-1.993L3 17h18H3Zm0-6 18-.002a1 1 0 0 1 .117 1.993l-.117.007L3 13a1 1 0 0 1-.117-1.993L3 11l18-.002L3 11Zm0-6h18a1 1 0 0 1 .117 1.993L21 7H3a1 1 0 0 1-.117-1.993L3 5h18H3Z"))?)));
    // E72B Back: the back button of NavigationView (NavigationBackButtonNormalStyle), of the title bar (TitleBarBackButtonStyle) and of
    //       NavigationPage.
    resources.deferred("GlyphGeometryE72B", |_| Ok(value(literal(StreamGeometry::parse("M12.7347,4.20949 C13.0332,3.92233 13.508,3.93153 13.7952,4.23005 C14.0823,4.52857 14.0731,5.00335 13.7746,5.29051 L5.50039,13.25 L24.2532,13.25 C24.6674,13.25 25.0032,13.5858 25.0032,13.9999982 C25.0032,14.4142 24.6674,14.75 24.2532,14.75 L5.50137,14.75 L13.7746,22.7085 C14.0731,22.9957 14.0823,23.4705 13.7952,23.769 C13.508,24.0675 13.0332,24.0767 12.7347,23.7896 L3.30673,14.7202 C2.89776,14.3268 2.89776,13.6723 3.30673,13.2788 L12.7347,4.20949 Z"))?)));
    // E712 More: the ellipsis of the More button of CommandBar (EllipsisIcon), three dots.
    resources.deferred("GlyphGeometryE712", |_| Ok(value(literal(StreamGeometry::parse("M1.5,0 A1.5,1.5 0 1 1 1.5,3 A1.5,1.5 0 1 1 1.5,0 Z M7.5,0 A1.5,1.5 0 1 1 7.5,3 A1.5,1.5 0 1 1 7.5,0 Z M13.5,0 A1.5,1.5 0 1 1 13.5,3 A1.5,1.5 0 1 1 13.5,0 Z"))?)));
    // The icon of the drawer toggle FerroUI's NavigationPage looks up in code and casts to a
    //       StreamGeometry (not a key of WinUI): the outline of E700.
    resources.deferred("NavigationPageMenuIcon", |_| Ok(value(literal(StreamGeometry::parse("M3 17h18a1 1 0 0 1 .117 1.993L21 19H3a1 1 0 0 1-.117-1.993L3 17h18H3Zm0-6 18-.002a1 1 0 0 1 .117 1.993l-.117.007L3 13a1 1 0 0 1-.117-1.993L3 11l18-.002L3 11Zm0-6h18a1 1 0 0 1 .117 1.993L21 7H3a1 1 0 0 1-.117-1.993L3 5h18H3Z"))?)));
    // Batch B2 (buttons, ranges and expanders). E70D is the outline of the chevron of the Fluent
    //       theme of FerroUI (src/FerroUI.Themes.Fluent/Controls/DropDownButton.xaml), E70E the same
    //       outline mirrored; the others are drawn for the port.
    // E70D ChevronDown: the spin down button of NumberBox, the chevron of the Expander header.
    resources.deferred("GlyphGeometryE70D", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M 1939,486 L 2029,576 L 1024,1581 L 19,576 L 109,486 L 1024,1401 Z",
        ))?))
    });
    // E70E ChevronUp: the spin up button of NumberBox, the chevron of an Expander that expands up.
    resources.deferred("GlyphGeometryE70E", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M 1939,1581 L 2029,1491 L 1024,486 L 19,1491 L 109,1581 L 1024,666 Z",
        ))?))
    });
    // E72C Refresh, the default content of RefreshVisualizer (SymbolIcon Symbol.Refresh): defined above with the refresh button of ManagedFileChooser.
    // E96E ChevronDownSmall: the chevron of DropDownButton, of SplitButton and the drop-down glyph of ComboBox.
    resources.deferred("GlyphGeometryE96E", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M 1.4,4.6 L 8,11.2 L 14.6,4.6 L 16,6 L 8,14 L 0,6 Z",
        ))?))
    });
    // E972 ChevronDownMed: the chevron of SplitButtonCommandBarStyle.
    resources.deferred("GlyphGeometryE972", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M 1.06,4.94 L 8,11.88 L 14.94,4.94 L 16,6 L 8,14 L 0,6 Z",
        ))?))
    });
    // EA3B: the dot of a pip of PipsPager (PipsPagerNormalGlyph, PipsPagerSelectedGlyph).
    resources.deferred("GlyphGeometryEA3B", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M 0,8 A 8,8 0 1 1 16,8 A 8,8 0 1 1 0,8 Z",
        ))?))
    });
    // EC8F: the popup indicator of NumberBoxTextBoxStyle (an up and a down chevron).
    resources.deferred("GlyphGeometryEC8F", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M 4,6.5 L 8,2.5 L 12,6.5 L 11,7.5 L 8,4.5 L 5,7.5 Z M 4,9.5 L 5,8.5 L 8,11.5 L 11,8.5 L 12,9.5 L 8,13.5 Z",
        ))?))
    });
    // Batch B3 (flyouts, menus, lists and selection). The chevrons are drawn in the units of the
    //       em square of the font (2048): E70D is the outline of the drop-down glyph of the Fluent theme
    //       of FerroUI (src/FerroUI.Themes.Fluent/Controls/ComboBox.xaml), E76C the same outline
    //       turned to the right, E974 a chevron of a heavier stroke.
    // E70D ChevronDown: the expanded glyph of TreeViewItem.
    // E76C ChevronRight: the collapsed glyph of TreeViewItem, the chevron of
    //       RadioMenuFlyoutSubItemStyle.
    resources.deferred("GlyphGeometryE76C", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M486 1939L576 2029L1581 1024L576 19L486 109L1401 1024L486 1939Z",
        ))?))
    });
    // E974 ChevronRightMed: the chevron of a menu item with a submenu (MenuFlyoutSubItem).
    resources.deferred("GlyphGeometryE974", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M768 256L1536 1024L768 1792L598 1622L1196 1024L598 426Z",
        ))?))
    });
    // E915: the bullet of a checked radio menu item (RadioMenuFlyoutItem).
    resources.deferred("GlyphGeometryE915", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M0,5 A5,5 0 1 1 10,5 A5,5 0 1 1 0,5 Z",
        ))?))
    });
    // The icons of the standard commands of the context flyout of the text controls
    //       (StandardUICommand Cut, Copy, Paste, Undo, Redo), drawn on a 16 px grid with strokes of 1 px:
    //       outer contours clockwise, holes counterclockwise.
    // E8C6 Cut: two rings and two crossing blades.
    resources.deferred("GlyphGeometryE8C6", |_| Ok(value(literal(StreamGeometry::parse("M2,12 A2.5,2.5 0 1 1 7,12 A2.5,2.5 0 1 1 2,12 Z M3,12 A1.5,1.5 0 1 0 6,12 A1.5,1.5 0 1 0 3,12 Z M9,12 A2.5,2.5 0 1 1 14,12 A2.5,2.5 0 1 1 9,12 Z M10,12 A1.5,1.5 0 1 0 13,12 A1.5,1.5 0 1 0 10,12 Z M5.18,9.53 L10.58,0.93 L11.42,1.47 L6.02,10.07 Z M9.98,10.07 L4.58,1.47 L5.42,0.93 L10.82,9.53 Z"))?)));
    // E8C8 Copy: a sheet in front of another.
    resources.deferred("GlyphGeometryE8C8", |_| Ok(value(literal(StreamGeometry::parse("M5,4 L5,2.5 A1.5,1.5 0 0 1 6.5,1 L12.5,1 A1.5,1.5 0 0 1 14,2.5 L14,10.5 A1.5,1.5 0 0 1 12.5,12 L11,12 L11,11 L12.5,11 A0.5,0.5 0 0 0 13,10.5 L13,2.5 A0.5,0.5 0 0 0 12.5,2 L6.5,2 A0.5,0.5 0 0 0 6,2.5 L6,4 Z M3.5,4 L9.5,4 A1.5,1.5 0 0 1 11,5.5 L11,13.5 A1.5,1.5 0 0 1 9.5,15 L3.5,15 A1.5,1.5 0 0 1 2,13.5 L2,5.5 A1.5,1.5 0 0 1 3.5,4 Z M3.5,5 A0.5,0.5 0 0 0 3,5.5 L3,13.5 A0.5,0.5 0 0 0 3.5,14 L9.5,14 A0.5,0.5 0 0 0 10,13.5 L10,5.5 A0.5,0.5 0 0 0 9.5,5 Z"))?)));
    // E77F Paste: a clipboard with its clip.
    resources.deferred("GlyphGeometryE77F", |_| Ok(value(literal(StreamGeometry::parse("M3.5,2 L12.5,2 A1.5,1.5 0 0 1 14,3.5 L14,13.5 A1.5,1.5 0 0 1 12.5,15 L3.5,15 A1.5,1.5 0 0 1 2,13.5 L2,3.5 A1.5,1.5 0 0 1 3.5,2 Z M3.5,3 A0.5,0.5 0 0 0 3,3.5 L3,13.5 A0.5,0.5 0 0 0 3.5,14 L12.5,14 A0.5,0.5 0 0 0 13,13.5 L13,3.5 A0.5,0.5 0 0 0 12.5,3 Z M6.5,1 L9.5,1 A1,1 0 0 1 10.5,2 L10.5,3 A1,1 0 0 1 9.5,4 L6.5,4 A1,1 0 0 1 5.5,3 L5.5,2 A1,1 0 0 1 6.5,1 Z"))?)));
    // E7A7 Undo: an arrow to the left that turns back on the right.
    resources.deferred("GlyphGeometryE7A7", |_| Ok(value(literal(StreamGeometry::parse("M2.5,4.5 L9,4.5 A4.5,4.5 0 0 1 9,13.5 L6,13.5 L6,12.5 L9,12.5 A3.5,3.5 0 0 0 9,5.5 L2.5,5.5 Z M5.554,2.154 L2.708,5 L5.554,7.846 L4.846,8.554 L1.292,5 L4.846,1.446 Z"))?)));
    // E7A6 Redo: E7A7 mirrored.
    resources.deferred("GlyphGeometryE7A6", |_| Ok(value(literal(StreamGeometry::parse("M13.5,4.5 L7,4.5 A4.5,4.5 0 0 0 7,13.5 L10,13.5 L10,12.5 L7,12.5 A3.5,3.5 0 0 1 7,5.5 L13.5,5.5 Z M10.446,2.154 L13.292,5 L10.446,7.846 L11.154,8.554 L14.708,5 L11.154,1.446 Z"))?)));
    // F78D RevealPasswordMedium: the eye of the reveal button of PasswordBox; the outline of the
    //       Fluent theme of FerroUI (PasswordBoxRevealButtonData of
    //       src/FerroUI.Themes.Fluent/Controls/TextBox.xaml).
    resources.deferred("GlyphGeometryF78D", |_| Ok(value(literal(StreamGeometry::parse("m10.051 7.0032c2.215 0 4.0105 1.7901 4.0105 3.9984s-1.7956 3.9984-4.0105 3.9984c-2.215 0-4.0105-1.7901-4.0105-3.9984s1.7956-3.9984 4.0105-3.9984zm0 1.4994c-1.3844 0-2.5066 1.1188-2.5066 2.499s1.1222 2.499 2.5066 2.499 2.5066-1.1188 2.5066-2.499-1.1222-2.499-2.5066-2.499zm0-5.0026c4.6257 0 8.6188 3.1487 9.7267 7.5613 0.10085 0.40165-0.14399 0.80877-0.54686 0.90931-0.40288 0.10054-0.81122-0.14355-0.91208-0.54521-0.94136-3.7492-4.3361-6.4261-8.2678-6.4261-3.9334 0-7.3292 2.6792-8.2689 6.4306-0.10063 0.40171-0.50884 0.64603-0.91177 0.54571s-0.648-0.5073-0.54737-0.90901c1.106-4.4152 5.1003-7.5667 9.728-7.5667z"))?)));
    // The glyphs of MediaTransportControls (MediaTransportControls_themeresources.xaml): FontIcon
    //       glyphs of the template and the SymbolIcon symbols it names (Volume, Mute, Stop, RepeatAll,
    //       RepeatOne, FullScreen, BackToWindow), drawn for the port in a box of 20 by 20.
    // F5B0 PlaySolid: PlayPauseSymbol and PlayPauseSymbolLeft in PlayState.
    resources.deferred("GlyphGeometryF5B0", |_| {
        Ok(value(literal(StreamGeometry::parse("M6 3 L17 10 L6 17 Z"))?))
    });
    // F8AE PauseSolid: PlayPauseSymbol and PlayPauseSymbolLeft in PauseState.
    resources.deferred("GlyphGeometryF8AE", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M5 3 H8 V17 H5 Z M12 3 H15 V17 H12 Z",
        ))?))
    });
    // E15B Stop: StopButton (Icon="Stop").
    resources.deferred("GlyphGeometryE15B", |_| {
        Ok(value(literal(StreamGeometry::parse("M5 5 H15 V15 H5 Z"))?))
    });
    // E15D Volume: VolumeMuteSymbol and AudioMuteSymbol in VolumeState.
    resources.deferred("GlyphGeometryE15D", |_| Ok(value(literal(StreamGeometry::parse("M2 7 H6 L11 3 V17 L6 13 H2 Z M13 6 V8 C15 9 15 11 13 12 V14 C18 12 18 8 13 6 Z M15 2 V4 C21 7 21 13 15 16 V18 C24 14 24 6 15 2 Z"))?)));
    // E198 Mute: VolumeMuteSymbol and AudioMuteSymbol in MuteState.
    resources.deferred("GlyphGeometryE198", |_| Ok(value(literal(StreamGeometry::parse("M2 7 H6 L11 3 V17 L6 13 H2 Z M14 6 L16 8 L18 6 L19.5 7.5 L17.5 9.5 L19.5 11.5 L18 13 L16 11 L14 13 L12.5 11.5 L14.5 9.5 L12.5 7.5 Z"))?)));
    // ED3C SkipBack10 and ED3D SkipForward30: SkipBackwardButton and SkipForwardButton.
    resources.deferred("GlyphGeometryED3C", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M10 2 V0 L6 3 L10 6 V4 A6 6 0 1 1 4 10 H2 A8 8 0 1 0 10 2 Z",
        ))?))
    });
    resources.deferred("GlyphGeometryED3D", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M10 2 V0 L14 3 L10 6 V4 A6 6 0 1 0 16 10 H18 A8 8 0 1 1 10 2 Z",
        ))?))
    });
    // E627 Rewind and E628 FastForward: RewindButton and FastForwardButton.
    resources.deferred("GlyphGeometryE627", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M10 4 L2 10 L10 16 Z M18 4 L10 10 L18 16 Z",
        ))?))
    });
    resources.deferred("GlyphGeometryE628", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M2 4 L10 10 L2 16 Z M10 4 L18 10 L10 16 Z",
        ))?))
    });
    // F8AC PreviousSolid and F8AD NextSolid: PreviousTrackButton and NextTrackButton.
    resources.deferred("GlyphGeometryF8AC", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M3 4 H5 V16 H3 Z M17 4 L6 10 L17 16 Z",
        ))?))
    });
    resources.deferred("GlyphGeometryF8AD", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M15 4 H17 V16 H15 Z M3 4 L14 10 L3 16 Z",
        ))?))
    });
    // EC57 Speed: PlaybackRateButton.
    resources.deferred("GlyphGeometryEC57", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M2 14 A8 8 0 0 1 18 14 H16 A6 6 0 0 0 4 14 Z M9 14 L13 7 L14.5 8 L11 15 Z",
        ))?))
    });
    // E1CD RepeatAll and E1CC RepeatOne: RepeatSymbol in RepeatNoneState, RepeatAllState and
    //       RepeatOneState.
    resources.deferred("GlyphGeometryE1CD", |_| Ok(value(literal(StreamGeometry::parse("M4 9 V8 A3 3 0 0 1 7 5 H14 V3 L17 6 L14 9 V7 H7 A1 1 0 0 0 6 8 V9 Z M16 11 V12 A3 3 0 0 1 13 15 H6 V17 L3 14 L6 11 V13 H13 A1 1 0 0 0 14 12 V11 Z"))?)));
    resources.deferred("GlyphGeometryE1CC", |_| Ok(value(literal(StreamGeometry::parse("M4 9 V8 A3 3 0 0 1 7 5 H14 V3 L17 6 L14 9 V7 H7 A1 1 0 0 0 6 8 V9 Z M16 11 V12 A3 3 0 0 1 13 15 H6 V17 L3 14 L6 11 V13 H13 A1 1 0 0 0 14 12 V11 Z M9.25 8.5 H10.75 V12 H9.25 Z"))?)));
    // E799 AspectRatio: ZoomButton.
    resources.deferred("GlyphGeometryE799", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M2 4 H18 V16 H2 Z M4 6 V14 H16 V6 Z M6 8 H9 V9.5 H7.5 V11 H6 Z M14 12 H11 V10.5 H12.5 V9 H14 Z",
        ))?))
    });
    // EC15 Cast: CastButton.
    resources.deferred("GlyphGeometryEC15", |_| Ok(value(literal(StreamGeometry::parse("M2 4 H18 V16 H12 V14 H16 V6 H4 V8 H2 Z M2 10 A6 6 0 0 1 8 16 H6 A4 4 0 0 0 2 12 Z M2 14 A2 2 0 0 1 4 16 H2 Z"))?)));
    // ED1E Subtitles: CCSelectionButton.
    resources.deferred("GlyphGeometryED1E", |_| Ok(value(literal(StreamGeometry::parse("M2 4 H18 V16 H2 Z M4 6 V14 H16 V6 Z M5 8 H9 V9.5 H6.5 V10.5 H9 V12 H5 Z M11 8 H15 V9.5 H12.5 V10.5 H15 V12 H11 Z"))?)));
    // ED1F: AudioTracksSelectionButton.
    resources.deferred("GlyphGeometryED1F", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M8 3 H17 V13.5 A2.5 2.5 0 1 1 15 11.05 V6 H10 V15.5 A2.5 2.5 0 1 1 8 13.05 Z",
        ))?))
    });
    // E1D9 FullScreen and E1D8 BackToWindow: FullWindowSymbol in NonFullWindowState and
    //       FullWindowState.
    resources.deferred("GlyphGeometryE1D9", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M2 8 V2 H8 V4 H4 V8 Z M12 2 H18 V8 H16 V4 H12 Z M18 12 V18 H12 V16 H16 V12 Z M8 18 H2 V12 H4 V16 H8 Z",
        ))?))
    });
    resources.deferred("GlyphGeometryE1D8", |_| {
        Ok(value(literal(StreamGeometry::parse(
            "M6 2 H8 V8 H2 V6 H6 Z M12 2 H14 V6 H18 V8 H12 Z M12 12 H18 V14 H14 V18 H12 Z M2 12 H8 V18 H6 V14 H2 Z",
        ))?))
    });
    Ok(())
}

/// The resource dictionary of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<ResourceDictionary>> {
    let dictionary = ResourceDictionary::new();
    merge(&Resources::new(&dictionary, scope))?;
    Ok(dictionary)
}

//! `WinUITheme.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! The WinUI theme: the theme of a WinUI 3 application of microsoft-ui-xaml
//! (https://github.com/microsoft/microsoft-ui-xaml, commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0),
//! generic.xaml with the theme resources of XamlControlsResources merged over it, ported to FerroUI
//! (docs/porting/winui-theme.md). Copyright (c) Microsoft Corporation for the ported documents;
//! licensed under the MIT License (see NOTICE.md of the crate).
//!
//! The resources: the accent colour of the system, the system colours (Accents/SystemColors, the
//! colours of the platform in high contrast), and one document per source document of the theme,
//! in the order of the merge of WinUI. The list between the markers is written by
//! scripts/winui-theme/convert.py.

#![allow(unused_imports, clippy::all)]

use crate::accents::SystemAccentColors;
use crate::accents::SystemColors;
use crate::rust_theme::prelude::*;
use ferroui_base::styling::Styles;

/// Populates `styles` (and its resources) from the document, declared in `scope`.
pub(crate) fn populate(styles: &Ref<Styles>, scope: &Scope) -> Result {
    let scope = &scope.with_styles(styles);
    let resources = Resources::new(&styles.resources(), scope);
    resources.merge(SystemAccentColors::new());
    resources.merge(SystemColors::new());
    // resources:begin (written by scripts/winui-theme/convert.py; do not edit)
    resources.include(1)?; // Resources/Generic.xaml
    resources.include(2)?; // Resources/Common_themeresources.xaml
    resources.include(3)?; // Resources/Common_themeresources_any.xaml
    resources.include(4)?; // Resources/Deprecated_themeresources_any.xaml
    resources.include(5)?; // Resources/RichEditBox_themeresources.xaml
    resources.include(6)?; // Resources/PasswordBox_themeresources.xaml
    resources.include(7)?; // Resources/TextBox_themeresources.xaml
    resources.include(8)?; // Resources/MenuFlyout_themeresources.xaml
    resources.include(9)?; // Resources/AcrylicBrush_themeresources.xaml
    resources.include(10)?; // Resources/RevealBrush_themeresources.xaml
    resources.include(11)?; // Resources/Deprecated_themeresources.xaml
    resources.include(12)?; // Resources/Hyperlink_themeresources.xaml
    resources.include(13)?; // Resources/HyperlinkButton_themeresources.xaml
    resources.include(14)?; // Resources/RadioButton_themeresources.xaml
    resources.include(15)?; // Resources/AppBarButton_themeresources.xaml
    resources.include(16)?; // Resources/AppBarSeparator_themeresources.xaml
    resources.include(17)?; // Resources/AppBarToggleButton_themeresources.xaml
    resources.include(18)?; // Resources/ListBox_themeresources.xaml
    resources.include(19)?; // Resources/ToggleSwitch_themeresources.xaml
    resources.include(20)?; // Resources/CommandBar_themeresources.xaml
    resources.include(21)?; // Resources/GridViewItem_themeresources.xaml
    resources.include(22)?; // Resources/ListViewItem_themeresources.xaml
    resources.include(23)?; // Resources/TextBlock_themeresources.xaml
    resources.include(24)?; // Resources/TextBlock_themeresources_v2.5.xaml
    resources.include(25)?; // Resources/FlyoutPresenter_themeresources.xaml
    resources.include(26)?; // Resources/Button_themeresources.xaml
    resources.include(27)?; // Resources/ToggleButton_themeresources.xaml
    resources.include(28)?; // Resources/CornerRadius_themeresources.xaml
    resources.include(29)?; // Resources/MediaTransportControls_themeresources.xaml
    resources.include(30)?; // Resources/RepeatButton_themeresources.xaml
    resources.include(31)?; // Resources/CalendarDatePicker_themeresources.xaml
    resources.include(32)?; // Resources/CalendarView_themeresources.xaml
    resources.include(33)?; // Resources/CheckBox_themeresources.xaml
    resources.include(34)?; // Resources/ContentDialog_themeresources.xaml
    resources.include(35)?; // Resources/DatePicker_themeresources.xaml
    resources.include(36)?; // Resources/DateTimePickerFlyout_themeresources.xaml
    resources.include(37)?; // Resources/FlipView_themeresources.xaml
    resources.include(38)?; // Resources/FlipViewItem_themeresources.xaml
    resources.include(39)?; // Resources/Pivot_themeresources.xaml
    resources.include(40)?; // Resources/ScrollBar_themeresources.xaml
    resources.include(41)?; // Resources/ScrollViewer_themeresources.xaml
    resources.include(42)?; // Resources/Slider_themeresources.xaml
    resources.include(43)?; // Resources/TimePicker_themeresources.xaml
    resources.include(44)?; // Resources/ToolTip_themeresources.xaml
    resources.include(45)?; // Resources/RatingControl_themeresources.xaml
    resources.include(46)?; // Resources/InfoBadge_themeresources.xaml
    resources.include(47)?; // Resources/NavigationView_themeresources.xaml
    resources.include(48)?; // Resources/NavigationBackButton_themeresources.xaml
    resources.include(49)?; // Resources/PersonPicture_themeresources.xaml
    resources.include(50)?; // Resources/RefreshContainer_themeresources.xaml
    resources.include(51)?; // Resources/RefreshVisualizer_themeresources.xaml
    resources.include(52)?; // Resources/MenuBar_themeresources.xaml
    resources.include(53)?; // Resources/TreeView_themeresources.xaml
    resources.include(54)?; // Resources/SwipeControl_themeresources.xaml
    resources.include(55)?; // Resources/SplitButton_themeresources.xaml
    resources.include(56)?; // Resources/DropDownButton_themeresources.xaml
    resources.include(57)?; // Resources/RadioButtons_themeresources.xaml
    resources.include(58)?; // Resources/TabView_themeresources.xaml
    resources.include(59)?; // Resources/AutoSuggestBox_themeresources.xaml
    resources.include(60)?; // Resources/ComboBox_themeresources.xaml
    resources.include(61)?; // Resources/SplitView_themeresources.xaml
    resources.include(62)?; // Resources/ProgressBar_themeresources.xaml
    resources.include(63)?; // Resources/ProgressRing_themeresources.xaml
    resources.include(64)?; // Resources/NumberBox_themeresources.xaml
    resources.include(65)?; // Resources/Expander_themeresources.xaml
    resources.include(66)?; // Resources/PagerControl_themeresources.xaml
    resources.include(67)?; // Resources/InfoBar_themeresources.xaml
    resources.include(68)?; // Resources/PipsPager_themeresources.xaml
    resources.include(69)?; // Resources/SelectorBar_themeresources.xaml
    resources.include(70)?; // Resources/TitleBar_themeresources.xaml
    resources.include(71)?; // Resources/TabularSurfaces_themeresources.xaml
    resources.include(72)?; // Resources/TableView_themeresources.xaml
    resources.include(73)?; // Resources/ResizeGripper_themeresources.xaml
    resources.include(74)?; // Resources/ThemeShadows.xaml
    resources.include(75)?; // Resources/Glyphs.xaml
                            // resources:end
                            // The labels of the commands of the context flyout of the text controls (not part of the
                            // WinUI theme: WinUI takes them from the resources of the framework).
    resources.include(76)?; // Strings/InvariantResources.xaml
                            // Not part of the merged dictionaries: the theme looks it up first when its density style is compact.
    resources.deferred("CompactStyles", |scope| {
        Ok(value(crate::rust_theme::density_styles::compact::build(scope)?))
    });
    styles.add(crate::rust_theme::controls::win_ui_controls::build(scope)?);
    Ok(())
}

/// The styles of the document, declared in `scope`: the document included (`StyleInclude`).
#[allow(dead_code)]
pub(crate) fn build(scope: &Scope) -> Result<Ref<Styles>> {
    let styles = Styles::new();
    populate(&styles, scope)?;
    Ok(styles)
}

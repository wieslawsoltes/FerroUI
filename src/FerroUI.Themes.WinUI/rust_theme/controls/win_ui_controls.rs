//! `Controls/WinUIControls.xaml`, converted from XAML by `scripts/themes/xaml_to_rust.py` (#988).
//!
//! Generated: change the document or the converter and run the converter again
//! (docs/xaml-themes.md, section 10). The header of the document:
//!
//! The control themes of the WinUI theme (docs/porting/winui-theme.md): one document per FerroUI
//! control, ported from the styles of microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml,
//! commit 7e4b0c2096be402f51cc8f7d203ebf67cff183d0) that each document names in its header.
//! A document that is embedded but not ready to be part of the theme is listed in
//! Controls/excluded.txt and not included here.

#![allow(unused_imports, clippy::all)]

use crate::rust_theme::prelude::*;
use ferroui_base::layout::Layoutable;
use ferroui_base::styling::Styles;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::MenuItem;
use ferroui_controls::PathIcon;
use ferroui_markup_xaml::rust_theme::style;

/// Populates `styles` (and its resources) from the document, declared in `scope`.
pub(crate) fn populate(styles: &Ref<Styles>, scope: &Scope) -> Result {
    let scope = &scope.with_styles(styles);
    let resources = Resources::new(&styles.resources(), scope);
    // Foundation: hosts of windows and popups, focus visuals.
    crate::rust_theme::controls::adorner_layer::merge(&resources)?;
    crate::rust_theme::controls::embeddable_control_root::merge(&resources)?;
    crate::rust_theme::controls::overlay_popup_host::merge(&resources)?;
    crate::rust_theme::controls::popup_root::merge(&resources)?;
    crate::rust_theme::controls::theme_variant_scope::merge(&resources)?;
    crate::rust_theme::controls::window::merge(&resources)?;
    // Foundation: text, buttons, check box, radio button, text box, scrolling.
    resources.include(77)?; // Controls/TextBlock.xaml
    resources.include(78)?; // Controls/Button.xaml
    resources.include(79)?; // Controls/RepeatButton.xaml
    resources.include(80)?; // Controls/ToggleButton.xaml
    resources.include(81)?; // Controls/RevealStyles.xaml
    resources.include(82)?; // Controls/HyperlinkButton.xaml
    resources.include(83)?; // Controls/CheckBox.xaml
    resources.include(84)?; // Controls/RadioButton.xaml
    resources.include(85)?; // Controls/Thumb.xaml
    resources.include(86)?; // Controls/ScrollBar.xaml
    resources.include(87)?; // Controls/ScrollViewer.xaml
    resources.include(88)?; // Controls/TextBox.xaml
                            // B1: primitives and simple content.
    resources.include(89)?; // Controls/ToolTip.xaml
    resources.include(90)?; // Controls/ProgressBar.xaml
    resources.include(91)?; // Controls/Separator.xaml
    resources.include(92)?; // Controls/PathIcon.xaml
                            // The icons of WinUI that FerroUI ports as controls (docs/porting/winui-controls.md).
    resources.include(93)?; // Controls/FontIcon.xaml
    resources.include(94)?; // Controls/SymbolIcon.xaml
    resources.include(95)?; // Controls/Label.xaml
    resources.include(96)?; // Controls/ItemsControl.xaml
    resources.include(97)?; // Controls/HeaderedContentControl.xaml
    resources.include(98)?; // Controls/GroupBox.xaml
    resources.include(99)?; // Controls/SelectableTextBlock.xaml
    resources.include(100)?; // Controls/TransitioningContentControl.xaml
    resources.include(101)?; // Controls/TextSelectionHandle.xaml
    resources.include(102)?; // Controls/DataValidationErrors.xaml
    resources.include(103)?; // Controls/GridSplitter.xaml
                             // B4: calendar, date and time.
    resources.include(104)?; // Controls/DateTimePickerShared.xaml
    resources.include(105)?; // Controls/CalendarDayButton.xaml
    resources.include(106)?; // Controls/CalendarButton.xaml
    resources.include(107)?; // Controls/CalendarItem.xaml
    resources.include(108)?; // Controls/Calendar.xaml
    resources.include(109)?; // Controls/CalendarDatePicker.xaml
    resources.include(110)?; // Controls/DatePicker.xaml
    resources.include(111)?; // Controls/TimePicker.xaml
                             // B6: window decorations, notifications, tables and dialogs.
    crate::rust_theme::controls::window_drawn_decorations::merge(&resources)?;
    resources.include(112)?; // Controls/NotificationCard.xaml
                             // The status controls of WinUI that FerroUI ports as controls (docs/porting/winui-controls.md);
                             //             InfoBar uses InfoBarCloseButtonStyle of NotificationCard.xaml.
    resources.include(113)?; // Controls/InfoBar.xaml
    resources.include(114)?; // Controls/InfoBadge.xaml
    resources.include(115)?; // Controls/Pivot.xaml
    resources.include(116)?; // Controls/SwipeControl.xaml
    resources.include(117)?; // Controls/TitleBar.xaml
    resources.include(118)?; // Controls/ProgressRing.xaml
    resources.include(119)?; // Controls/WindowNotificationManager.xaml
    resources.include(120)?; // Controls/TableView.xaml
    resources.include(121)?; // Controls/TableViewCell.xaml
    resources.include(122)?; // Controls/TableViewColumnHeader.xaml
    resources.include(123)?; // Controls/TableViewRow.xaml
    resources.include(124)?; // Controls/ManagedFileChooser.xaml
                             // B5: navigation, pages, tabs and command bar.
    resources.include(125)?; // Controls/TabStrip.xaml
    resources.include(126)?; // Controls/TabStripItem.xaml
    resources.include(127)?; // Controls/TabControl.xaml
    resources.include(128)?; // Controls/TabItem.xaml
    resources.include(129)?; // Controls/SplitView.xaml
    resources.include(130)?; // Controls/ContentPage.xaml
    resources.include(131)?; // Controls/TabbedPage.xaml
    resources.include(132)?; // Controls/NavigationPage.xaml
    resources.include(133)?; // Controls/DrawerPage.xaml
                             // The navigation of WinUI that FerroUI ports as controls (docs/porting/winui-controls.md):
                             //             Frame, NavigationView and its items; NavigationView uses the styles of DrawerPage.xaml
                             //             and NavigationPage.xaml.
    resources.include(134)?; // Controls/Frame.xaml
    resources.include(135)?; // Controls/NavigationView.xaml
    resources.include(136)?; // Controls/Carousel.xaml
    resources.include(137)?; // Controls/CarouselPage.xaml
    resources.include(138)?; // Controls/CommandBar.xaml
                             // B2: buttons, ranges and expanders.
    resources.include(139)?; // Controls/DropDownButton.xaml
    resources.include(140)?; // Controls/SplitButton.xaml
    resources.include(141)?; // Controls/ToggleSwitch.xaml
    resources.include(142)?; // Controls/Slider.xaml
    resources.include(143)?; // Controls/MediaTransportControls.xaml
    resources.include(144)?; // Controls/CameraCaptureUI.xaml
    resources.include(145)?; // Controls/NumericUpDown.xaml
                             // An input control of WinUI that FerroUI ports as a control (docs/porting/winui-controls.md).
    resources.include(146)?; // Controls/NumberBox.xaml
    resources.include(147)?; // Controls/ButtonSpinner.xaml
    resources.include(148)?; // Controls/Expander.xaml
    resources.include(149)?; // Controls/PipsPager.xaml
                             // An input control of WinUI that FerroUI ports as a control (docs/porting/winui-controls.md).
    resources.include(150)?; // Controls/RatingControl.xaml
    resources.include(151)?; // Controls/PersonPicture.xaml
    resources.include(152)?; // Controls/PagerControl.xaml
    resources.include(153)?; // Controls/RefreshContainer.xaml
    resources.include(154)?; // Controls/RefreshVisualizer.xaml
                             // B3: flyouts, menus, lists and selection.
    resources.include(155)?; // Controls/FlyoutPresenter.xaml
    resources.include(156)?; // Controls/MenuScrollViewer.xaml
    resources.include(157)?; // Controls/MenuFlyoutPresenter.xaml
    resources.include(158)?; // Controls/ContextMenu.xaml
    resources.include(159)?; // Controls/MenuItem.xaml
    resources.include(160)?; // Controls/Menu.xaml
    resources.include(161)?; // Controls/ListBoxItem.xaml
    resources.include(162)?; // Controls/ListBox.xaml
    resources.include(163)?; // Controls/ListView.xaml
    resources.include(164)?; // Controls/GridView.xaml
    resources.include(165)?; // Controls/ComboBoxItem.xaml
    resources.include(166)?; // Controls/ComboBox.xaml
    resources.include(167)?; // Controls/TreeViewItem.xaml
    resources.include(168)?; // Controls/TreeView.xaml
    resources.include(169)?; // Controls/AutoCompleteBox.xaml
                             // An input control of WinUI that FerroUI ports as a control (docs/porting/winui-controls.md).
    resources.include(170)?; // Controls/AutoSuggestBox.xaml
                             // An input control of WinUI that FerroUI ports as a control (docs/porting/winui-controls.md).
    resources.include(171)?; // Controls/RadioButtons.xaml
                             // WinUI's RichEditBox (docs/porting/winui-rich-text.md).
    resources.include(172)?; // Controls/RichEditBox.xaml
                             // The icon of a menu item (MenuItem.Icon, a logical child of the item): unscaled, in the
                             //       IconRoot Viewbox of 16 of the template of MenuFlyoutItem (DEVIATIONS.md, PI-3). A style of
                             //       the theme rather than of the control theme of PathIcon, whose nested styles cannot name a
                             //       parent (the selector MenuItem > ^ there matched every path icon).
    styles.add(style(scope, of::<MenuItem>().child().of::<PathIcon>(), |style| {
        style.set(Layoutable::width_property(), f64::NAN);
        style.set(Layoutable::height_property(), f64::NAN);
        style.static_resource(TemplatedControl::template_property(), "PathIconUnscaledTemplate")?;
        Ok(())
    })?);
    Ok(())
}

/// The styles of the document, declared in `scope`.
pub(crate) fn build(scope: &Scope) -> Result<Ref<Styles>> {
    let styles = Styles::new();
    populate(&styles, scope)?;
    Ok(styles)
}

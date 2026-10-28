//! Every markup document of the theme loads through the run-time loader:
//! one test per document, so that a failure names the document (as the
//! tests of the documents of the Fluent theme).
//!
//! A document is loaded on its own (parsed, transformed and built), and
//! then every resource it defines is looked up in the whole theme, for the
//! light, the dark and the high contrast variant, and realised there:
//! resources are deferred, and the resources of a document name resources of
//! other documents (a `StaticResource` alias, the base of a control theme),
//! which only exist in the theme.
//!
//! The tests of the documents `Controls/excluded.txt` leaves out of the
//! theme load the document on its own.

use super::support::*;
use crate::{assets, MacOSTheme};
use ferroui_base::controls::{ResourceDictionary, ResourceKey};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::Styles;
use ferroui_base::Ref;

/// The documents that only load as part of the group of the theme: they
/// include other documents with merged resource includes, which the
/// compiler (here: the group load of the theme) resolves.
const GROUP_DOCUMENTS: &[&str] = &["/MacOSTheme.xaml", "/Controls/MacOSControls.xaml"];

/// The documents the theme keeps outside its resources: the compact resources are looked up only
/// when the density style is compact.
const DENSITY_DOCUMENTS: &[&str] = &["/DensityStyles/Compact.xaml"];

fn check_document(path: &str) {
    let _app = start_application();

    if assets::is_excluded(path) {
        if let Err(error) = try_load_document(path) {
            panic!("{path} failed to load: {}", describe(&error));
        }
        return;
    }

    let theme = MacOSTheme::new();
    if GROUP_DOCUMENTS.contains(&path) {
        // Loaded with the theme: the theme document is the theme itself, the list of the control
        // themes is its only style.
        assert_eq!(1, theme.count());
        let controls = theme.get(0);
        let controls = controls.as_object().and_then(|object| object.downcast_ref::<Styles>()).expect("styles");
        assert!(!theme.resources().keys().is_empty());
        assert!(controls.resources().keys().iter().any(|key| matches!(key, ResourceKey::Type(_))));
        return;
    }

    let loaded = match try_load_document(path) {
        Ok(loaded) => loaded,
        Err(error) => panic!("{path} failed to load: {}", describe(&error)),
    };
    let dictionary = from_markup_value::<Ref<ResourceDictionary>>(&Some(loaded))
        .unwrap_or_else(|| panic!("{path} is not a resource dictionary"));

    let mut keys = dictionary.keys();
    for (_, provider) in dictionary.theme_dictionaries_snapshot() {
        let variant = provider.as_object().and_then(|object| object.downcast_ref::<ResourceDictionary>());
        keys.extend(variant.expect("a resource dictionary").keys());
    }
    assert!(!keys.is_empty(), "{path} defines no resources");

    if DENSITY_DOCUMENTS.contains(&path) {
        theme.set_density_style(crate::DensityStyle::Compact);
    }
    let style = theme.as_style();
    for key in keys {
        for variant in variants() {
            let value = style
                .try_get_resource(&key, Some(&variant))
                .unwrap_or_else(|| panic!("{path}: the theme has no resource {key:?} for {variant:?}"));
            if let Some(value) = value {
                realise(&value);
            }
        }
    }
}

macro_rules! document_tests {
    ($($name:ident => $path:literal;)*) => {
        $(
            #[test]
            fn $name() {
                check_document($path);
            }
        )*

        /// The documents with a test.
        const TESTED: &[&str] = &[$($path),*];
    };
}

document_tests! {
    resources_macos_materials => "/Resources/MacOSMaterials.xaml";
    controls_adorner_layer => "/Controls/AdornerLayer.xaml";
    controls_auto_complete_box => "/Controls/AutoCompleteBox.xaml";
    controls_button => "/Controls/Button.xaml";
    controls_calendar => "/Controls/Calendar.xaml";
    controls_calendar_button => "/Controls/CalendarButton.xaml";
    controls_calendar_date_picker => "/Controls/CalendarDatePicker.xaml";
    controls_calendar_day_button => "/Controls/CalendarDayButton.xaml";
    controls_calendar_item => "/Controls/CalendarItem.xaml";
    controls_check_box => "/Controls/CheckBox.xaml";
    controls_data_validation_errors => "/Controls/DataValidationErrors.xaml";
    controls_date_picker => "/Controls/DatePicker.xaml";
    controls_date_time_picker_shared => "/Controls/DateTimePickerShared.xaml";
    controls_combo_box => "/Controls/ComboBox.xaml";
    controls_combo_box_item => "/Controls/ComboBoxItem.xaml";
    controls_context_menu => "/Controls/ContextMenu.xaml";
    controls_embeddable_control_root => "/Controls/EmbeddableControlRoot.xaml";
    controls_grid_splitter => "/Controls/GridSplitter.xaml";
    controls_group_box => "/Controls/GroupBox.xaml";
    controls_headered_content_control => "/Controls/HeaderedContentControl.xaml";
    controls_flyout_presenter => "/Controls/FlyoutPresenter.xaml";
    controls_hyperlink_button => "/Controls/HyperlinkButton.xaml";
    controls_items_control => "/Controls/ItemsControl.xaml";
    controls_label => "/Controls/Label.xaml";
    controls_list_box => "/Controls/ListBox.xaml";
    controls_list_box_item => "/Controls/ListBoxItem.xaml";
    controls_menu => "/Controls/Menu.xaml";
    controls_menu_flyout_presenter => "/Controls/MenuFlyoutPresenter.xaml";
    controls_menu_item => "/Controls/MenuItem.xaml";
    controls_menu_scroll_viewer => "/Controls/MenuScrollViewer.xaml";
    controls_overlay_popup_host => "/Controls/OverlayPopupHost.xaml";
    controls_path_icon => "/Controls/PathIcon.xaml";
    controls_popup_root => "/Controls/PopupRoot.xaml";
    controls_progress_bar => "/Controls/ProgressBar.xaml";
    controls_radio_button => "/Controls/RadioButton.xaml";
    controls_repeat_button => "/Controls/RepeatButton.xaml";
    controls_scroll_bar => "/Controls/ScrollBar.xaml";
    controls_scroll_viewer => "/Controls/ScrollViewer.xaml";
    controls_selectable_text_block => "/Controls/SelectableTextBlock.xaml";
    controls_separator => "/Controls/Separator.xaml";
    controls_text_block => "/Controls/TextBlock.xaml";
    controls_text_box => "/Controls/TextBox.xaml";
    controls_text_selection_handle => "/Controls/TextSelectionHandle.xaml";
    controls_theme_variant_scope => "/Controls/ThemeVariantScope.xaml";
    controls_thumb => "/Controls/Thumb.xaml";
    controls_time_picker => "/Controls/TimePicker.xaml";
    controls_toggle_button => "/Controls/ToggleButton.xaml";
    controls_tool_tip => "/Controls/ToolTip.xaml";
    controls_transitioning_content_control => "/Controls/TransitioningContentControl.xaml";
    controls_tree_view => "/Controls/TreeView.xaml";
    controls_tree_view_item => "/Controls/TreeViewItem.xaml";
    controls_winui_controls => "/Controls/MacOSControls.xaml";
    // B5: navigation, pages, tabs and command bar
    controls_carousel => "/Controls/Carousel.xaml";
    controls_carousel_page => "/Controls/CarouselPage.xaml";
    controls_command_bar => "/Controls/CommandBar.xaml";
    controls_content_page => "/Controls/ContentPage.xaml";
    controls_drawer_page => "/Controls/DrawerPage.xaml";
    controls_navigation_page => "/Controls/NavigationPage.xaml";
    controls_split_view => "/Controls/SplitView.xaml";
    controls_tab_control => "/Controls/TabControl.xaml";
    controls_tab_item => "/Controls/TabItem.xaml";
    controls_tab_strip => "/Controls/TabStrip.xaml";
    controls_tab_strip_item => "/Controls/TabStripItem.xaml";
    controls_tabbed_page => "/Controls/TabbedPage.xaml";
    controls_window => "/Controls/Window.xaml";
    density_styles_compact => "/DensityStyles/Compact.xaml";
    resources_acrylic_brush_themeresources => "/Resources/AcrylicBrush_themeresources.xaml";
    resources_app_bar_button_themeresources => "/Resources/AppBarButton_themeresources.xaml";
    resources_app_bar_separator_themeresources => "/Resources/AppBarSeparator_themeresources.xaml";
    resources_app_bar_toggle_button_themeresources => "/Resources/AppBarToggleButton_themeresources.xaml";
    resources_auto_suggest_box_themeresources => "/Resources/AutoSuggestBox_themeresources.xaml";
    resources_button_themeresources => "/Resources/Button_themeresources.xaml";
    resources_calendar_date_picker_themeresources => "/Resources/CalendarDatePicker_themeresources.xaml";
    resources_calendar_view_themeresources => "/Resources/CalendarView_themeresources.xaml";
    resources_check_box_themeresources => "/Resources/CheckBox_themeresources.xaml";
    resources_combo_box_themeresources => "/Resources/ComboBox_themeresources.xaml";
    resources_command_bar_themeresources => "/Resources/CommandBar_themeresources.xaml";
    resources_common_themeresources => "/Resources/Common_themeresources.xaml";
    resources_common_themeresources_any => "/Resources/Common_themeresources_any.xaml";
    resources_content_dialog_themeresources => "/Resources/ContentDialog_themeresources.xaml";
    resources_corner_radius_themeresources => "/Resources/CornerRadius_themeresources.xaml";
    resources_date_picker_themeresources => "/Resources/DatePicker_themeresources.xaml";
    resources_date_time_picker_flyout_themeresources => "/Resources/DateTimePickerFlyout_themeresources.xaml";
    resources_deprecated_themeresources => "/Resources/Deprecated_themeresources.xaml";
    resources_deprecated_themeresources_any => "/Resources/Deprecated_themeresources_any.xaml";
    resources_drop_down_button_themeresources => "/Resources/DropDownButton_themeresources.xaml";
    resources_expander_themeresources => "/Resources/Expander_themeresources.xaml";
    resources_flip_view_item_themeresources => "/Resources/FlipViewItem_themeresources.xaml";
    resources_flip_view_themeresources => "/Resources/FlipView_themeresources.xaml";
    resources_flyout_presenter_themeresources => "/Resources/FlyoutPresenter_themeresources.xaml";
    resources_generic => "/Resources/Generic.xaml";
    resources_glyphs => "/Resources/Glyphs.xaml";
    resources_hyperlink_button_themeresources => "/Resources/HyperlinkButton_themeresources.xaml";
    resources_hyperlink_themeresources => "/Resources/Hyperlink_themeresources.xaml";
    resources_info_bar_themeresources => "/Resources/InfoBar_themeresources.xaml";
    resources_list_box_themeresources => "/Resources/ListBox_themeresources.xaml";
    resources_list_view_item_themeresources => "/Resources/ListViewItem_themeresources.xaml";
    resources_menu_bar_themeresources => "/Resources/MenuBar_themeresources.xaml";
    resources_menu_flyout_themeresources => "/Resources/MenuFlyout_themeresources.xaml";
    resources_navigation_back_button_themeresources => "/Resources/NavigationBackButton_themeresources.xaml";
    resources_navigation_view_themeresources => "/Resources/NavigationView_themeresources.xaml";
    resources_number_box_themeresources => "/Resources/NumberBox_themeresources.xaml";
    resources_password_box_themeresources => "/Resources/PasswordBox_themeresources.xaml";
    resources_pips_pager_themeresources => "/Resources/PipsPager_themeresources.xaml";
    resources_progress_bar_themeresources => "/Resources/ProgressBar_themeresources.xaml";
    resources_radio_button_themeresources => "/Resources/RadioButton_themeresources.xaml";
    resources_refresh_container_themeresources => "/Resources/RefreshContainer_themeresources.xaml";
    resources_refresh_visualizer_themeresources => "/Resources/RefreshVisualizer_themeresources.xaml";
    resources_repeat_button_themeresources => "/Resources/RepeatButton_themeresources.xaml";
    resources_reveal_brush_themeresources => "/Resources/RevealBrush_themeresources.xaml";
    resources_scroll_bar_themeresources => "/Resources/ScrollBar_themeresources.xaml";
    resources_scroll_viewer_themeresources => "/Resources/ScrollViewer_themeresources.xaml";
    resources_selector_bar_themeresources => "/Resources/SelectorBar_themeresources.xaml";
    resources_slider_themeresources => "/Resources/Slider_themeresources.xaml";
    resources_split_button_themeresources => "/Resources/SplitButton_themeresources.xaml";
    resources_split_view_themeresources => "/Resources/SplitView_themeresources.xaml";
    resources_system_resources => "/Resources/SystemResources.xaml";
    resources_tab_view_themeresources => "/Resources/TabView_themeresources.xaml";
    resources_text_block_themeresources => "/Resources/TextBlock_themeresources.xaml";
    resources_text_block_themeresources_v2_5 => "/Resources/TextBlock_themeresources_v2.5.xaml";
    resources_text_box_themeresources => "/Resources/TextBox_themeresources.xaml";
    resources_theme_shadows => "/Resources/ThemeShadows.xaml";
    resources_time_picker_themeresources => "/Resources/TimePicker_themeresources.xaml";
    resources_title_bar_themeresources => "/Resources/TitleBar_themeresources.xaml";
    resources_toggle_button_themeresources => "/Resources/ToggleButton_themeresources.xaml";
    resources_toggle_switch_themeresources => "/Resources/ToggleSwitch_themeresources.xaml";
    resources_tool_tip_themeresources => "/Resources/ToolTip_themeresources.xaml";
    resources_tree_view_themeresources => "/Resources/TreeView_themeresources.xaml";
    controls_managed_file_chooser => "/Controls/ManagedFileChooser.xaml";
    controls_notification_card => "/Controls/NotificationCard.xaml";
    controls_table_view => "/Controls/TableView.xaml";
    controls_table_view_cell => "/Controls/TableViewCell.xaml";
    controls_table_view_column_header => "/Controls/TableViewColumnHeader.xaml";
    controls_table_view_row => "/Controls/TableViewRow.xaml";
    controls_window_drawn_decorations => "/Controls/WindowDrawnDecorations.xaml";
    controls_window_notification_manager => "/Controls/WindowNotificationManager.xaml";
    resources_resize_gripper_themeresources => "/Resources/ResizeGripper_themeresources.xaml";
    resources_table_view_themeresources => "/Resources/TableView_themeresources.xaml";
    resources_tabular_surfaces_themeresources => "/Resources/TabularSurfaces_themeresources.xaml";
    strings_invariant_resources => "/Strings/InvariantResources.xaml";
    macos_theme => "/MacOSTheme.xaml";
    // B2: buttons, ranges and expanders
    controls_button_spinner => "/Controls/ButtonSpinner.xaml";
    controls_drop_down_button => "/Controls/DropDownButton.xaml";
    controls_expander => "/Controls/Expander.xaml";
    controls_numeric_up_down => "/Controls/NumericUpDown.xaml";
    controls_pips_pager => "/Controls/PipsPager.xaml";
    controls_refresh_container => "/Controls/RefreshContainer.xaml";
    controls_refresh_visualizer => "/Controls/RefreshVisualizer.xaml";
    controls_slider => "/Controls/Slider.xaml";
    controls_split_button => "/Controls/SplitButton.xaml";
    controls_toggle_switch => "/Controls/ToggleSwitch.xaml";
}

#[test]
fn every_document_has_a_test() {
    let mut embedded: Vec<&str> = assets::documents().iter().map(|(path, _)| *path).collect();
    embedded.sort_unstable();
    let mut tested: Vec<&str> = TESTED.to_vec();
    tested.sort_unstable();
    assert_eq!(embedded, tested);
}

/// Every document that is part of the theme loads on its own (parsed, transformed and built):
/// the part of the tests above that does not need the theme as a whole.
#[test]
fn documents_load_on_their_own() {
    let _app = start_application();
    let mut failures = Vec::new();
    let mut loaded = 0;
    for (path, _) in assets::documents() {
        if assets::is_excluded(path) || GROUP_DOCUMENTS.contains(path) {
            continue;
        }
        match try_load_document(path) {
            Ok(_) => loaded += 1,
            Err(error) => failures.push(format!("{path}: {}", describe(&error))),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(loaded > 70);
}

#[test]
fn excluded_documents_are_not_merged() {
    let controls =
        std::str::from_utf8(assets::document("/Controls/MacOSControls.xaml").expect("embedded")).expect("UTF-8");
    for (path, _) in assets::documents() {
        let Some(file) = path.strip_prefix("/Controls/") else { continue };
        if file == "MacOSControls.xaml" {
            continue;
        }
        let included = controls.contains(&format!("\"ferres://FerroUI.Themes.MacOS/Controls/{file}\""));
        assert_eq!(!assets::is_excluded(path), included, "{path}");
    }
}

/// The resource documents are the ones `scripts/winui-theme/convert.py` writes and the theme
/// document includes: every document under `Resources/` is merged into the theme.
#[test]
fn resource_documents_are_merged() {
    let theme = std::str::from_utf8(assets::document("/MacOSTheme.xaml").expect("embedded")).expect("UTF-8");
    for (path, _) in assets::documents() {
        if path.starts_with("/Resources/") {
            assert!(theme.contains(&format!("<MergeResourceInclude Source=\"{path}\" />")), "{path} is not merged");
        }
    }
}

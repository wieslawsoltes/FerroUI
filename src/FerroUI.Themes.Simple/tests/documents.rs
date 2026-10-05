//! Every markup document of the theme loads through the run-time loader:
//! one test per document, so that a failure names the document.
//!
//! A document is loaded on its own (parsed, transformed and built), and
//! then every resource it defines is looked up in the whole theme, for the
//! light and the dark variant, and realised there: resources are deferred,
//! and the control themes of a document may be based on resources of other
//! documents, which only exist in the theme.
//!
//! The tests of the documents `Controls/excluded.txt` leaves out of the
//! theme are ignored with what they wait for; they load the document on its
//! own.

use super::support::*;
use crate::{assets, SimpleTheme};
use ferroui_base::controls::{ResourceDictionary, ResourceKey};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::{Styles, ThemeVariant};
use ferroui_base::Ref;

/// The documents that only load as part of the group of the theme: they
/// include other documents with merged resource includes, which the
/// compiler (here: the group load of the theme) resolves.
const GROUP_DOCUMENTS: &[&str] = &["/SimpleTheme.xaml", "/Controls/SimpleControls.xaml"];

fn check_document(path: &str) {
    let _app = start_application();

    if assets::is_excluded(path) {
        if let Err(error) = try_load_document(path) {
            panic!("{path} failed to load: {}", describe(&error));
        }
        return;
    }

    let theme = SimpleTheme::new();
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

    let style = theme.as_style();
    for key in keys {
        for variant in [ThemeVariant::light(), ThemeVariant::dark()] {
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
    ($($name:ident => $path:literal $(, ignore = $reason:literal)?;)*) => {
        $(
            #[test]
            $(#[ignore = $reason])?
            fn $name() {
                check_document($path);
            }
        )*

        /// The documents with a test, and whether the test is ignored.
        const TESTED: &[(&str, bool)] = &[$(($path, false $(|| !$reason.is_empty())?)),*];
    };
}

document_tests! {
    accents_base => "/Accents/Base.xaml";
    controls_adorner_layer => "/Controls/AdornerLayer.xaml";
    controls_auto_complete_box => "/Controls/AutoCompleteBox.xaml";
    controls_button => "/Controls/Button.xaml";
    controls_button_spinner => "/Controls/ButtonSpinner.xaml";
    controls_calendar => "/Controls/Calendar.xaml";
    controls_calendar_button => "/Controls/CalendarButton.xaml";
    controls_calendar_date_picker => "/Controls/CalendarDatePicker.xaml";
    controls_calendar_day_button => "/Controls/CalendarDayButton.xaml";
    controls_calendar_item => "/Controls/CalendarItem.xaml";
    controls_carousel => "/Controls/Carousel.xaml";
    controls_carousel_page => "/Controls/CarouselPage.xaml";
    controls_check_box => "/Controls/CheckBox.xaml";
    controls_combo_box => "/Controls/ComboBox.xaml";
    controls_combo_box_item => "/Controls/ComboBoxItem.xaml";
    controls_command_bar => "/Controls/CommandBar.xaml";
    controls_content_page => "/Controls/ContentPage.xaml";
    controls_context_menu => "/Controls/ContextMenu.xaml";
    controls_data_validation_errors => "/Controls/DataValidationErrors.xaml";
    controls_date_picker => "/Controls/DatePicker.xaml";
    controls_date_time_picker_shared => "/Controls/DateTimePickerShared.xaml";
    controls_drawer_page => "/Controls/DrawerPage.xaml";
    controls_drop_down_button => "/Controls/DropDownButton.xaml";
    controls_embeddable_control_root => "/Controls/EmbeddableControlRoot.xaml";
    controls_expander => "/Controls/Expander.xaml";
    controls_flyout_presenter => "/Controls/FlyoutPresenter.xaml";
    controls_grid_splitter => "/Controls/GridSplitter.xaml";
    controls_group_box => "/Controls/GroupBox.xaml";
    controls_headered_content_control => "/Controls/HeaderedContentControl.xaml";
    controls_hyperlink_button => "/Controls/HyperlinkButton.xaml";
    controls_items_control => "/Controls/ItemsControl.xaml";
    controls_label => "/Controls/Label.xaml";
    controls_list_box => "/Controls/ListBox.xaml";
    controls_list_box_item => "/Controls/ListBoxItem.xaml";
    controls_managed_file_chooser => "/Controls/ManagedFileChooser.xaml";
    controls_menu => "/Controls/Menu.xaml";
    controls_menu_flyout_presenter => "/Controls/MenuFlyoutPresenter.xaml";
    controls_menu_item => "/Controls/MenuItem.xaml";
    controls_navigation_page => "/Controls/NavigationPage.xaml";
    controls_notification_card => "/Controls/NotificationCard.xaml";
    controls_numeric_up_down => "/Controls/NumericUpDown.xaml";
    controls_overlay_popup_host => "/Controls/OverlayPopupHost.xaml";
    controls_path_icon => "/Controls/PathIcon.xaml";
    controls_pips_pager => "/Controls/PipsPager.xaml";
    controls_popup_root => "/Controls/PopupRoot.xaml";
    controls_progress_bar => "/Controls/ProgressBar.xaml";
    controls_radio_button => "/Controls/RadioButton.xaml";
    controls_refresh_container => "/Controls/RefreshContainer.xaml";
    controls_refresh_visualizer => "/Controls/RefreshVisualizer.xaml";
    controls_repeat_button => "/Controls/RepeatButton.xaml";
    controls_scroll_bar => "/Controls/ScrollBar.xaml";
    controls_scroll_viewer => "/Controls/ScrollViewer.xaml";
    controls_selectable_text_block => "/Controls/SelectableTextBlock.xaml";
    controls_separator => "/Controls/Separator.xaml";
    controls_simple_controls => "/Controls/SimpleControls.xaml";
    controls_slider => "/Controls/Slider.xaml";
    controls_split_button => "/Controls/SplitButton.xaml";
    controls_split_view => "/Controls/SplitView.xaml";
    controls_tab_control => "/Controls/TabControl.xaml";
    controls_tab_item => "/Controls/TabItem.xaml";
    controls_tab_strip => "/Controls/TabStrip.xaml";
    controls_tab_strip_item => "/Controls/TabStripItem.xaml";
    controls_tabbed_page => "/Controls/TabbedPage.xaml";
    controls_table_view => "/Controls/TableView.xaml";
    controls_table_view_cell => "/Controls/TableViewCell.xaml";
    controls_table_view_column_header => "/Controls/TableViewColumnHeader.xaml";
    controls_table_view_row => "/Controls/TableViewRow.xaml";
    controls_text_box => "/Controls/TextBox.xaml";
    controls_text_selection_handle => "/Controls/TextSelectionHandle.xaml";
    controls_theme_variant_scope => "/Controls/ThemeVariantScope.xaml";
    controls_time_picker => "/Controls/TimePicker.xaml";
    controls_toggle_button => "/Controls/ToggleButton.xaml";
    controls_toggle_switch => "/Controls/ToggleSwitch.xaml";
    controls_tool_tip => "/Controls/ToolTip.xaml";
    controls_transitioning_content_control => "/Controls/TransitioningContentControl.xaml";
    controls_tree_view => "/Controls/TreeView.xaml";
    controls_tree_view_item => "/Controls/TreeViewItem.xaml";
    controls_window => "/Controls/Window.xaml";
    controls_window_drawn_decorations => "/Controls/WindowDrawnDecorations.xaml";
    controls_window_notification_manager => "/Controls/WindowNotificationManager.xaml";
    simple_theme => "/SimpleTheme.xaml";
    strings_invariant_resources => "/Strings/InvariantResources.xaml";
}

#[test]
fn every_document_has_a_test() {
    let embedded: Vec<&str> = assets::documents().iter().map(|(path, _)| *path).collect();
    let mut tested: Vec<&str> = TESTED.iter().map(|(path, _)| *path).collect();
    tested.sort_unstable();
    assert_eq!(embedded, tested);

    // The ignored tests are the ones of the documents that are left out of the theme.
    for (path, ignored) in TESTED {
        assert_eq!(assets::is_excluded(path), *ignored, "{path}");
    }
    assert_eq!(TESTED.iter().filter(|(_, ignored)| *ignored).count(), crate::excluded_documents().len());
}

#[test]
fn excluded_documents_are_not_merged() {
    let controls = std::str::from_utf8(assets::document("/Controls/SimpleControls.xaml").expect("embedded")).expect("UTF-8");
    for (path, _) in assets::documents() {
        let Some(file) = path.strip_prefix("/Controls/") else { continue };
        if file == "SimpleControls.xaml" {
            continue;
        }
        let included = controls.contains(&format!("\"ferres://FerroUI.Themes.Simple/Controls/{file}\""));
        assert_eq!(!assets::is_excluded(path), included, "{path}");
    }
}

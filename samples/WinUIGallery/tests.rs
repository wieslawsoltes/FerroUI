use crate::{catalog, examples, navigation::NavigationHistory, GalleryView, CATALOG};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, CheckBox, ComboBox, Slider, TextBlock, TextBox};
use std::collections::HashSet;

pub(crate) fn start() -> ferroui_controls::testing::UnitTestApplicationScope {
    ferroui_markup_xaml::register_types();
    let scope = UnitTestApplication::start(TestServices::styled_window());
    // These are explicit mock resource contracts. Rendering tests install the
    // actual compiled WinUI theme and exercise its light/dark resources.
    let application = ferroui_controls::Application::current().unwrap();
    for name in [
        "TextOnAccentFillColorPrimaryBrush",
        "AccentFillColorDefaultBrush",
        "SystemControlHighlightAltChromeWhiteBrush",
        "SystemControlHighlightAccentBrush",
        "SystemFillColorAttentionBrush",
        "SystemFillColorSolidNeutralBrush",
        "SystemFillColorSuccessBrush",
        "SystemFillColorCautionBrush",
        "SystemFillColorCriticalBrush",
    ] {
        application.resources().add(
            ferroui_base::controls::ResourceKey::from(name),
            Some(std::rc::Rc::new(ferroui_base::media::Brushes::blue()) as ferroui_base::BoxedValue),
        );
    }
    scope
}

#[test]
fn complete_pinned_catalog_has_unique_ids_and_all_source_samples() {
    assert_eq!(CATALOG.len(), 126);
    assert_eq!(
        CATALOG.iter().map(|page| page.samples.len()).sum::<usize>(),
        350
    );
    let mut ids = HashSet::new();
    for entry in CATALOG {
        assert!(ids.insert(entry.id));
        assert!(!entry.source.is_empty());
        assert!(!entry.description.is_empty());
        assert!(entry.source_url().contains(crate::SOURCE_COMMIT));
        for sample in entry.samples {
            assert_eq!(sample.source_sha256.len(), 64);
            assert!(!sample.source.is_empty());
        }
    }
}

#[test]
fn search_uses_words_categories_and_stable_ids() {
    assert_eq!(
        catalog::search("CALENDARDATEPICKER", None)[0].id,
        "CalendarDatePicker"
    );
    let category = catalog::search("", Some("Date & time"));
    assert_eq!(category.len(), 4);
    assert!(category.iter().all(|entry| entry.category == "Date & time"));
    assert!(catalog::search("a term absent from this complete catalog", None).is_empty());
    assert!(catalog::search("button basic", None)
        .iter()
        .all(|entry| entry.matches("button basic")));
}

#[test]
fn navigation_rejects_unknown_pages_and_keeps_valid_forward_history() {
    let mut history = NavigationHistory::default();
    assert!(!history.navigate("NoSuchPage"));
    assert!(history.navigate("Button"));
    assert!(history.navigate("Slider"));
    assert!(history.go_back());
    assert_eq!(history.current(), Some("Button"));
    assert!(!history.navigate("BUTTON"));
    assert!(history.can_go_forward());
    assert!(!history.navigate("NoSuchPage"));
    assert!(history.can_go_forward());
    assert!(history.go_forward());
    assert_eq!(history.current(), Some("Slider"));
    assert!(history.go_back());
    assert!(history.navigate("TextBox"));
    assert!(!history.can_go_forward());
}

#[test]
fn all_reviewed_native_adaptations_load_real_controls() {
    let _application = start();
    let mut errors = Vec::new();
    for adaptation in examples::ADAPTATIONS {
        assert!(catalog::find(adaptation.page_id).is_some());
        if let Err(error) = examples::create(adaptation) {
            errors.push(format!("{}: {error}", adaptation.page_id));
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn button_click_and_disabled_option_update_native_controls() {
    let _application = start();
    let root = examples::create(examples::find("Button").unwrap()).unwrap();
    let primary = root.get_control::<Button>("Primary");
    primary.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    primary.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert_eq!(
        root.get_control::<TextBlock>("Output").text().as_deref(),
        Some("Clicked 2 times")
    );
    root.get_control::<CheckBox>("Disable")
        .set_is_checked(Some(true));
    assert!(!primary.is_enabled());
    root.get_control::<CheckBox>("Disable")
        .set_is_checked(Some(false));
    assert!(primary.is_enabled());
}

#[test]
fn three_state_checkbox_preserves_null_and_reports_each_state() {
    let _application = start();
    let root = examples::create(examples::find("CheckBox").unwrap()).unwrap();
    let primary = root.get_control::<ToggleButton>("ThreeState");
    let output = root.get_control::<TextBlock>("ThreeStateOutput");
    primary.set_is_checked(Some(true));
    assert_eq!(output.text().as_deref(), Some("CheckBox is checked."));
    primary.set_is_checked(None);
    assert_eq!(
        output.text().as_deref(),
        Some("CheckBox state is indeterminate.")
    );
    primary.set_is_checked(Some(false));
    assert_eq!(output.text().as_deref(), Some("CheckBox is unchecked."));
}

#[test]
fn textbox_counts_unicode_scalar_values() {
    let _application = start();
    let root = examples::create(examples::find("TextBox").unwrap()).unwrap();
    root.get_control::<TextBox>("Primary")
        .set_text(Some("A🦀é"));
    assert_eq!(
        root.get_control::<TextBlock>("Output").text().as_deref(),
        Some("3 characters")
    );
}

#[test]
fn password_examples_keep_source_validation_and_reveal_behavior() {
    let _application = start();
    let root = examples::create(examples::find("PasswordBox").unwrap()).unwrap();
    let password = root.get_control::<TextBox>("SimplePassword");
    let validation = root.get_control::<TextBlock>("PasswordValidation");
    password.set_text(Some("Password"));
    assert_eq!(password.text().as_deref(), Some(""));
    assert_eq!(
        validation.text().as_deref(),
        Some("'Password' is not allowed.")
    );
    assert!(validation.is_visible());
    password.set_text(Some("a valid sample value"));
    assert_eq!(validation.text().as_deref(), Some(""));
    assert!(!validation.is_visible());
    password.set_text(Some(""));
    assert!(validation.is_visible());
    let reveal = root.get_control::<CheckBox>("revealModeCheckBox");
    let revealed = root.get_control::<TextBox>("passworBoxWithRevealmode");
    assert!(!revealed.reveal_password());
    reveal.set_is_checked(Some(true));
    assert!(revealed.reveal_password());
    reveal.set_is_checked(Some(false));
    assert!(!revealed.reveal_password());
}

#[test]
fn checkbox_select_all_keeps_source_aggregate_and_user_cycle_behavior() {
    let _application = start();
    let root = examples::create(examples::find("CheckBox").unwrap()).unwrap();
    let all = root.get_control::<CheckBox>("OptionsAllCheckBox");
    let options = ["Option1CheckBox", "Option2CheckBox", "Option3CheckBox"]
        .map(|name| root.get_control::<CheckBox>(name));
    assert_eq!(all.is_checked(), None);
    options[1].set_is_checked(Some(false));
    assert_eq!(all.is_checked(), Some(false));
    all.set_is_checked(Some(true));
    assert!(options
        .iter()
        .all(|option| option.is_checked() == Some(true)));
    // A user cycle from checked to indeterminate clears all options in the
    // source; a programmatic mixed aggregate is retained instead.
    all.set_is_checked(None);
    assert_eq!(all.is_checked(), Some(false));
    assert!(options
        .iter()
        .all(|option| option.is_checked() == Some(false)));
    options[0].set_is_checked(Some(true));
    assert_eq!(all.is_checked(), None);
    options[1].set_is_checked(Some(true));
    options[2].set_is_checked(Some(true));
    assert_eq!(all.is_checked(), Some(true));
    let two_state = root.get_control::<CheckBox>("TwoState");
    two_state.set_is_checked(Some(true));
    assert_eq!(
        root.get_control::<TextBlock>("TwoStateOutput")
            .text()
            .as_deref(),
        Some("You checked the box.")
    );
    two_state.set_is_checked(Some(false));
    assert_eq!(
        root.get_control::<TextBlock>("TwoStateOutput")
            .text()
            .as_deref(),
        Some("You unchecked the box.")
    );
}

#[test]
fn slider_and_combobox_outputs_observe_shared_properties() {
    let _application = start();
    let root = examples::create(examples::find("Slider").unwrap()).unwrap();
    root.get_control::<Slider>("Primary").set_range_value(900.0);
    assert_eq!(
        root.get_control::<TextBlock>("Output").text().as_deref(),
        Some("Value: 900")
    );
    let root = examples::create(examples::find("ComboBox").unwrap()).unwrap();
    root.get_control::<ComboBox>("Primary")
        .set_selected_index(2);
    assert_eq!(
        root.get_control::<TextBlock>("Output").text().as_deref(),
        Some("Selected index: 2")
    );
}

#[test]
fn native_gallery_filters_navigates_and_retains_history() {
    let _application = start();
    let gallery = GalleryView::new().unwrap();
    assert_eq!(gallery.visible_page_count(), 126);
    assert_eq!(gallery.current_page(), Some("Button"));
    gallery.set_search("CalendarDatePicker");
    assert_eq!(gallery.visible_page_count(), 1);
    assert!(gallery.navigate("CalendarDatePicker"));
    assert_eq!(gallery.current_page(), Some("CalendarDatePicker"));
    assert!(gallery.go_back());
    assert_eq!(gallery.current_page(), Some("Button"));
    assert!(gallery.go_forward());
    assert_eq!(gallery.current_page(), Some("CalendarDatePicker"));
    assert!(!gallery.navigate("NoSuchPage"));
    gallery.set_search("");
    assert_eq!(gallery.visible_page_count(), 126);
}

#[test]
fn every_catalog_page_opens_in_the_native_shell() {
    let _application = start();
    let gallery = GalleryView::new().unwrap();
    for entry in CATALOG {
        gallery.navigate(entry.id);
        assert_eq!(gallery.current_page(), Some(entry.id));
        assert!(
            gallery.native_example_error().is_none(),
            "{}: {:?}",
            entry.id,
            gallery.native_example_error()
        );
    }
}

#[test]
fn source_preview_preserves_unported_bindings_and_substitution_tokens() {
    let button = catalog::find("Button").unwrap();
    let sample = &button.samples[0];
    assert!(sample.xaml.contains("$(IsEnabled)"));
    let wrapped = crate::gallery::wrap_winui_sample(sample);
    assert!(wrapped.contains("$(IsEnabled)"));
    assert!(wrapped.contains("Click=\"Button_Click\""));
    assert!(wrapped.contains("http://schemas.microsoft.com/winfx/2006/xaml/presentation"));
}

#[test]
fn winui_preview_uses_opt_in_public_loader_and_rejects_unported_bindings() {
    let _application = start();
    let root = crate::gallery::load_winui_preview("<TextBlock xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\" Text=\"Preview\"/>").unwrap();
    assert_eq!(
        root.cast::<TextBlock>().unwrap().text().as_deref(),
        Some("Preview")
    );
    assert!(crate::gallery::load_winui_preview("<TextBlock xmlns=\"http://schemas.microsoft.com/winfx/2006/xaml/presentation\" xmlns:x=\"http://schemas.microsoft.com/winfx/2006/xaml\" Text=\"{x:Bind Text}\"/>").is_err());
}

#[test]
fn winui_preview_reports_missing_resource_without_unwinding() {
    let _application = start();
    assert!(crate::gallery::load_winui_preview(r#"<StackPanel xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><StackPanel.Resources><ControlTheme x:Key="Example" TargetType="Button" BasedOn="{StaticResource {x:Type Button}}"/></StackPanel.Resources><Button Theme="{StaticResource Example}"/></StackPanel>"#).is_err());
}

#[test]
fn contrast_calculation_matches_original_endpoints_symmetry_and_thresholds() {
    use crate::contrast::{contrast_ratio, parse_rgb, relative_luminance, ContrastResult};
    let black = parse_rgb("#000000").unwrap();
    let white = parse_rgb("FFFFFF").unwrap();
    assert_eq!(relative_luminance(black), 0.0);
    assert_eq!(relative_luminance(white), 1.0);
    assert_eq!(contrast_ratio(black, white), 21.0);
    assert_eq!(contrast_ratio(white, black), 21.0);
    assert_eq!(contrast_ratio(white, white), 1.0);
    let marginal = ContrastResult::calculate(parse_rgb("#777777").unwrap(), white);
    assert!((marginal.ratio - 4.47809).abs() < 0.0001);
    assert!(!marginal.normal_text);
    assert!(marginal.large_text);
    assert!(marginal.components);
    for invalid in ["#fff", "#FFFFFFFF", "FFGG00", "", "🦀0000"] {
        assert!(parse_rgb(invalid).is_none());
    }
}

#[test]
fn contrast_inputs_update_ratio_and_report_invalid_values() {
    let _application = start();
    let root = examples::create(examples::find("AccessibilityColorContrast").unwrap()).unwrap();
    let ratio = root.get_control::<TextBlock>("ContrastRatio");
    assert_eq!(ratio.text().as_deref(), Some("21.00:1"));
    let input = root.get_control::<TextBox>("TextColor");
    input.set_text(Some("#777777"));
    assert_eq!(ratio.text().as_deref(), Some("4.48:1"));
    assert!(root
        .get_control::<TextBlock>("ContrastChecks")
        .text()
        .unwrap()
        .starts_with("Regular text (4.5:1): Fail"));
    input.set_text(Some("invalid"));
    assert_eq!(
        ratio.text().as_deref(),
        Some("Enter two six-digit RGB colors.")
    );
    input.set_text(Some("#FFFFFF"));
    assert_eq!(ratio.text().as_deref(), Some("1.00:1"));
}

#[test]
fn winui_numberbox_example_keeps_expression_nan_and_valuechanged_behavior() {
    use ferroui_controls::NumberBox;
    let _application = start();
    let root = examples::create(examples::find("NumberBox").unwrap()).unwrap();
    let expression = root.get_control::<NumberBox>("Expression");
    assert!(expression.value().is_nan());
    expression.set_text("1 + 2^2");
    assert!(expression.commit_input());
    assert_eq!(expression.value(), 5.0);
    expression.set_text("");
    assert!(expression.commit_input());
    assert!(expression.value().is_nan());
    root.get_control::<NumberBox>("Number")
        .set_numeric_value(31.0);
    assert_eq!(
        root.get_control::<TextBlock>("Output").text().as_deref(),
        Some("Value: 31")
    );
}

#[test]
fn page_transition_sample_uses_winui_frame_journal_and_source_page_types() {
    use crate::frame_sample::{FirstSamplePage, SecondSamplePage};
    use ferroui_controls::Frame;
    let _application = start();
    let root = examples::create(examples::find("PageTransition").unwrap()).unwrap();
    let frame = root.get_control::<Frame>("ContentFrame");
    assert_eq!(
        frame.current_source_page_type(),
        Some(FirstSamplePage::TYPE)
    );
    assert!(!frame.can_go_back());
    root.get_control::<Button>("NavigationNext")
        .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert_eq!(
        frame.current_source_page_type(),
        Some(SecondSamplePage::TYPE)
    );
    assert_eq!(frame.back_stack_depth(), 1);
    root.get_control::<Button>("NavigationBack")
        .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert_eq!(
        frame.current_source_page_type(),
        Some(FirstSamplePage::TYPE)
    );
    assert!(frame.can_go_forward());
    root.get_control::<Button>("NavigationForward")
        .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert_eq!(
        frame.current_source_page_type(),
        Some(SecondSamplePage::TYPE)
    );
    assert!(root
        .get_control::<TextBlock>("NavigationStatus")
        .text()
        .unwrap()
        .starts_with("Second page · Forward"));
}

#[test]
fn command_bar_examples_keep_click_and_three_state_handlers() {
    let _application = start();
    for page in ["AppBarButton", "CommandBar"] {
        let root = examples::create(examples::find(page).unwrap()).unwrap();
        root.get_control::<Button>("Primary")
            .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
        assert_eq!(
            root.get_control::<TextBlock>("Output").text().as_deref(),
            Some("Clicked 1 times")
        );
    }
    let root = examples::create(examples::find("AppBarToggleButton").unwrap()).unwrap();
    root.get_control::<ToggleButton>("Primary")
        .set_is_checked(None);
    assert_eq!(
        root.get_control::<TextBlock>("Output").text().as_deref(),
        Some("Indeterminate")
    );
}

#[test]
fn radio_groups_port_original_selection_and_color_handlers() {
    use ferroui_base::media::Color;
    use ferroui_controls::RadioButtons;
    use ferroui_controls::Border;
    let _application = start();
    let root = examples::create(examples::find("RadioButton").unwrap()).unwrap();
    root.get_control::<ToggleButton>("Option2")
        .set_is_checked(Some(true));
    assert_eq!(
        root.get_control::<TextBlock>("Control1Output")
            .text()
            .as_deref(),
        Some("You selected Option 2")
    );
    let background = root.get_control::<RadioButtons>("BackgroundRadioButtons");
    let border = root.get_control::<RadioButtons>("BorderRadioButtons");
    assert_eq!(background.max_columns(), 3);
    background.set_selected_index(0);
    border.set_selected_index(0);
    let output = root.get_control::<Border>("ControlOutput");
    assert_eq!(
        output
            .background()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        Color::from_rgb(0, 128, 0)
    );
    assert_eq!(
        output
            .border_brush()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        Color::from_rgb(0, 100, 0)
    );
    background.set_selected_index(1);
    border.set_selected_index(1);
    assert_eq!(
        output
            .background()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        Color::from_rgb(255, 255, 0)
    );
    assert_eq!(
        output
            .border_brush()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        Color::from_rgb(255, 215, 0)
    );
}

#[test]
fn icon_examples_preserve_source_symbol_and_font_glyph() {
    use ferroui_controls::{FontIcon, Symbol, SymbolIcon};
    let _application = start();
    let root = examples::create(examples::find("IconElement").unwrap()).unwrap();
    assert_eq!(
        root.get_control::<SymbolIcon>("AcceptIcon").symbol(),
        Symbol::Accept
    );
    assert_eq!(
        root.get_control::<FontIcon>("FontGlyph").glyph(),
        "\u{e790}"
    );
}

#[test]
fn dropping_native_example_releases_handler_targets() {
    let _application = start();
    let root = examples::create(examples::find("Button").unwrap()).unwrap();
    let primary = root.get_control::<Button>("Primary").downgrade();
    let output = root.get_control::<TextBlock>("Output").downgrade();
    drop(root);
    assert!(primary.upgrade().is_none());
    assert!(output.upgrade().is_none());
}

#[test]
fn popup_sample_opens_closes_and_observes_offset_options() {
    use ferroui_controls::primitives::Popup;
    use ferroui_controls::{Control, Window};
    let _application = start();
    let root = examples::create(examples::find("Popup").unwrap()).unwrap();
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    let popup = root.get_control::<Popup>("StandardPopup");
    root.get_control::<Slider>("HorizontalOffset")
        .set_range_value(42.0);
    root.get_control::<Slider>("VerticalOffset")
        .set_range_value(-12.0);
    assert_eq!(popup.horizontal_offset(), 42.0);
    assert_eq!(popup.vertical_offset(), -12.0);
    root.get_control::<Button>("ShowPopup")
        .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert!(popup.is_open());
    assert!(popup.host().is_some());
    assert_eq!(
        root.get_control::<TextBlock>("PopupOutput")
            .text()
            .as_deref(),
        Some("Open")
    );
    root.get_control::<Button>("ClosePopup")
        .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert!(!popup.is_open());
    assert_eq!(
        root.get_control::<TextBlock>("PopupOutput")
            .text()
            .as_deref(),
        Some("Closed")
    );
    window.close();
}

#[test]
fn clipboard_sample_uses_host_service_for_copy_paste_formats_and_clear() {
    use ferroui_base::input::platform::IClipboard;
    use ferroui_controls::testing::{MockWindowingPlatform, TestClipboardImpl};
    use ferroui_controls::{Control, Window};
    let _application = start();
    let root = examples::create(examples::find("Clipboard").unwrap()).unwrap();
    let clipboard = TestClipboardImpl::new();
    let platform = MockWindowingPlatform::create_window_mock();
    platform.setup_feature::<dyn IClipboard>(clipboard.clipboard());
    let window = Window::with_impl(platform);
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    let click = |name| {
        root.get_control::<Button>(name)
            .raise_event(&RoutedEventArgs::with_event(&Button::click_event()))
    };
    click("CopyText");
    assert_eq!(clipboard.text().as_deref(), Some("Copy this text"));
    assert_eq!(clipboard.set_data_count(), 1);
    clipboard.set_text(Some("External text 🦀"));
    click("PasteText");
    assert_eq!(
        root.get_control::<TextBox>("PastedText").text().as_deref(),
        Some("External text 🦀")
    );
    click("ShowClipboardFormats");
    assert!(root
        .get_control::<TextBlock>("ClipboardOutput")
        .text()
        .unwrap()
        .starts_with("Available formats: "));
    click("ClearClipboard");
    assert_eq!(clipboard.clear_count(), 1);
    assert!(clipboard.text().is_none());
    click("PasteText");
    assert_eq!(
        root.get_control::<TextBlock>("ClipboardOutput")
            .text()
            .as_deref(),
        Some("The clipboard has no text.")
    );
    window.close();
}

#[test]
fn clipboard_sample_reports_missing_service_and_platform_failure() {
    use ferroui_base::input::platform::{ClipboardErrorKind, IClipboard};
    use ferroui_controls::testing::{MockWindowingPlatform, TestClipboardImpl};
    use ferroui_controls::{Control, Window};
    let _application = start();
    let root = examples::create(examples::find("Clipboard").unwrap()).unwrap();
    let copy = root.get_control::<Button>("CopyText");
    copy.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert_eq!(
        root.get_control::<TextBlock>("ClipboardOutput")
            .text()
            .as_deref(),
        Some("This window has no clipboard service.")
    );
    let clipboard = TestClipboardImpl::throwing(ClipboardErrorKind::AccessDenied);
    let platform = MockWindowingPlatform::create_window_mock();
    platform.setup_feature::<dyn IClipboard>(clipboard.clipboard());
    let window = Window::with_impl(platform);
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    copy.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert_eq!(
        root.get_control::<TextBlock>("ClipboardOutput")
            .text()
            .as_deref(),
        Some("Clipboard operation failed: Access to the clipboard was denied.")
    );
    window.close();
}

#[test]
fn refresh_sample_inserts_source_item_and_completes_its_deferral() {
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::pull_to_refresh::{RefreshContainer, RefreshVisualizerState};
    use ferroui_controls::{Control, ListBox, Window};
    let _application = start();
    let root = examples::create(examples::find("PullToRefresh").unwrap()).unwrap();
    let window = Window::new();
    ferroui_controls::testing::add_pull_to_refresh_themes(&window.styles());
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    let refresh = root.get_control::<RefreshContainer>("Refresh");
    let items = root.get_control::<ListBox>("RefreshItems");
    let visualizer = refresh
        .visualizer()
        .expect("the native theme creates a refresh visualizer");
    assert_eq!(items.items().count(), 5);
    root.get_control::<Button>("RequestRefresh")
        .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert_eq!(
        visualizer.refresh_visualizer_state(),
        RefreshVisualizerState::Refreshing
    );
    assert_eq!(
        root.get_control::<TextBlock>("RefreshOutput")
            .text()
            .as_deref(),
        Some("Refreshing…")
    );
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(items.items().count(), 6);
    assert_eq!(
        items
            .items()
            .get_at(0)
            .unwrap()
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("NewControl")
    );
    assert_eq!(
        visualizer.refresh_visualizer_state(),
        RefreshVisualizerState::Idle
    );
    assert_eq!(
        root.get_control::<TextBlock>("RefreshOutput")
            .text()
            .as_deref(),
        Some("NewControl inserted; refresh completed.")
    );
    window.close();
}

#[test]
fn autosuggest_filter_preserves_all_source_names_and_word_matching() {
    use crate::control_samples::cats_for_query;
    assert_eq!(crate::cat_data::CATS.len(), 97);
    assert_eq!(cats_for_query("").len(), 97);
    assert_eq!(cats_for_query("AMERICAN BOBTAIL"), ["American Bobtail"]);
    assert_eq!(cats_for_query("  British   Short "), ["British Shorthair"]);
    assert_eq!(
        cats_for_query("a breed absent from the source"),
        ["No results found"]
    );
}

#[test]
fn autosuggest_example_filters_only_user_changes_and_reports_selected_cat() {
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::AutoSuggestBox;
    use ferroui_controls::{Control, ListBox, Window};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;
    let _application = start();
    let root = examples::create(examples::find("AutoSuggestBox").unwrap()).unwrap();
    let suggestions = root.get_control::<AutoSuggestBox>("CatSuggestions");
    let parts = Rc::new(RefCell::new(None));
    let captured = parts.clone();
    suggestions.template_applied(move |_, args| {
        if let (Some(editor), Some(list)) = (
            args.name_scope().find_as::<TextBox>("PART_TextBox"),
            args.name_scope().find_as::<ListBox>("PART_SuggestionsList"),
        ) {
            *captured.borrow_mut() = Some((editor, list));
        }
    });
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    let (editor, list) = parts
        .borrow()
        .clone()
        .expect("native editor and suggestion list");
    let tick = || {
        for timer in Dispatcher::timers_for_unit_tests() {
            if timer.interval() == Duration::from_millis(150) {
                Dispatcher::force_fire_timer_for_unit_tests(&timer);
            }
        }
        Dispatcher::ui_thread().run_jobs(None);
    };
    suggestions.set_text("American");
    tick();
    assert_eq!(
        suggestions.items().count(),
        0,
        "programmatic changes must not filter"
    );
    editor.set_text(Some("British Shorthair"));
    tick();
    assert_eq!(suggestions.items().count(), 1);
    list.set_selected_index(0);
    assert_eq!(
        root.get_control::<TextBlock>("SuggestionOutput")
            .text()
            .as_deref(),
        Some("British Shorthair")
    );
    editor.set_text(Some("no such breed"));
    tick();
    assert_eq!(
        suggestions
            .items()
            .get_at(0)
            .unwrap()
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("No results found")
    );
    window.close();
}

#[test]
fn infobar_examples_keep_source_severity_message_action_and_options() {
    use ferroui_controls::{InfoBar, InfoBarSeverity};
    use ferroui_controls::HyperlinkButton;
    let _application = start();
    let root = examples::create(examples::find("InfoBar").unwrap()).unwrap();
    let first = root.get_control::<InfoBar>("TestInfoBar1");
    root.get_control::<ComboBox>("SeverityComboBox")
        .set_selected_index(3);
    assert_eq!(first.severity(), InfoBarSeverity::Error);
    first.set_is_open(false);
    assert_eq!(
        root.get_control::<CheckBox>("IsOpenCheckBox1").is_checked(),
        Some(false)
    );
    root.get_control::<CheckBox>("IsOpenCheckBox1")
        .set_is_checked(Some(true));
    assert!(first.is_open());
    let second = root.get_control::<InfoBar>("TestInfoBar2");
    assert!(second.message().starts_with("A long essential app message"));
    root.get_control::<ComboBox>("MessageComboBox")
        .set_selected_index(0);
    assert_eq!(second.message(), "A short essential app message.");
    let action = root.get_control::<ComboBox>("ActionButtonComboBox");
    action.set_selected_index(1);
    assert_eq!(
        second
            .action_button()
            .unwrap()
            .content()
            .unwrap()
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("Action")
    );
    action.set_selected_index(2);
    assert!(second.action_button().unwrap().is::<HyperlinkButton>());
    action.set_selected_index(0);
    assert!(second.action_button().is_none());
    let third = root.get_control::<InfoBar>("TestInfoBar3");
    root.get_control::<CheckBox>("IsClosableCheckBox")
        .set_is_checked(Some(false));
    root.get_control::<CheckBox>("IsIconVisibleCheckBox")
        .set_is_checked(Some(false));
    assert!(!third.is_closable());
    assert!(!third.is_icon_visible());
}

#[test]
fn control_search_preserves_title_tokens_prefix_priority_and_unique_metadata() {
    use crate::control_samples::controls_for_query;
    assert_eq!(controls_for_query("").len(), 126);
    assert_eq!(controls_for_query("button")[0].id, "Button");
    assert_eq!(
        controls_for_query("split button")
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        ["SplitButton", "ToggleSplitButton"]
    );
    assert!(
        controls_for_query("raises a Click event").is_empty(),
        "the source searches titles, not subtitles"
    );
    let titles: HashSet<_> = CATALOG.iter().map(|entry| entry.title).collect();
    assert_eq!(
        titles.len(),
        CATALOG.len(),
        "title item values must resolve to one source entry"
    );
    assert_eq!(
        catalog::find("Button").unwrap().subtitle,
        "A control that responds to user input and raises a Click event."
    );
}

#[test]
fn control_search_uses_chosen_suggestion_before_free_query_and_preserves_no_result() {
    use ferroui_base::input::{InputElement, Key, KeyEventArgs};
    use ferroui_controls::AutoSuggestBox;
    use ferroui_controls::{ItemsSource, ListBox};
    use std::cell::RefCell;
    use std::rc::Rc;
    let _application = start();
    let root = examples::create(examples::find("AutoSuggestBox").unwrap()).unwrap();
    let search = root.get_control::<AutoSuggestBox>("ControlSuggestions");
    let list = Rc::new(RefCell::new(None));
    let captured = list.clone();
    search.template_applied(move |_, args| {
        *captured.borrow_mut() = args.name_scope().find_as::<ListBox>("PART_SuggestionsList")
    });
    search.apply_template();
    let list = list.borrow().clone().expect("native suggestions list");
    search.set_items_source(Some(ItemsSource::from_strs(["Slider", "Button"])));
    search.set_is_suggestion_list_open(true);
    list.set_selected_index(0);
    search.set_text("Button");
    let mut enter = KeyEventArgs::new();
    enter.key = Key::Enter;
    enter.set_routed_event(Some(InputElement::key_down_event()));
    search.raise_event(&enter);
    assert_eq!(
        root.get_control::<TextBlock>("ControlTitle")
            .text()
            .as_deref(),
        Some("Slider")
    );
    search.set_text("button");
    search.submit_query();
    assert_eq!(
        root.get_control::<TextBlock>("ControlTitle")
            .text()
            .as_deref(),
        Some("Button")
    );
    assert_eq!(
        root.get_control::<TextBlock>("ControlSubtitle")
            .text()
            .as_deref(),
        Some(catalog::find("Button").unwrap().subtitle)
    );
    search.set_items_source(Some(ItemsSource::from_strs(["No results found"])));
    search.set_text("No matching title exists");
    search.set_is_suggestion_list_open(true);
    list.set_selected_index(0);
    assert_eq!(
        search.text(),
        "No matching title exists",
        "the sentinel must not autocomplete the query"
    );
    let mut enter = KeyEventArgs::new();
    enter.key = Key::Enter;
    enter.set_routed_event(Some(InputElement::key_down_event()));
    search.raise_event(&enter);
    assert_eq!(
        root.get_control::<TextBlock>("ControlTitle")
            .text()
            .as_deref(),
        Some("Button")
    );
}

#[test]
fn embedded_control_images_resolve_original_uri_paths_and_exact_bytes() {
    use ferroui_base::platform::AssetLoader;
    use ferroui_base::utilities::{Uri, UriKind};
    use std::io::Read;
    let _application = start();
    crate::assets::register();
    assert_eq!(
        crate::assets_data::ASSETS.len(),
        132,
        "108 control images, 22 sample media files and two explicit source-path case aliases"
    );
    for entry in CATALOG {
        let path = entry
            .image_uri
            .strip_prefix("ferres://WinUIGallery/")
            .unwrap();
        let expected = crate::assets_data::ASSETS
            .iter()
            .find(|(registered, _)| *registered == path)
            .unwrap()
            .1;
        let uri = Uri::new(entry.image_uri, UriKind::Absolute).unwrap();
        let mut actual = Vec::new();
        AssetLoader::open(&uri, None)
            .unwrap()
            .read_to_end(&mut actual)
            .unwrap();
        assert_eq!(
            actual, expected,
            "{} resolves its original image bytes",
            entry.id
        );
        assert!(actual.starts_with(b"\x89PNG\r\n\x1a\n"));
    }
}

#[test]
fn embedded_sample_media_resolve_original_paths_and_exact_bytes() {
    use ferroui_base::platform::AssetLoader;
    use ferroui_base::utilities::{Uri, UriKind};
    use std::io::Read;
    let _application = start();
    crate::assets::register();
    let media: Vec<_> = crate::assets_data::ASSETS
        .iter()
        .filter(|(path, _)| path.starts_with("Assets/SampleMedia/"))
        .collect();
    assert_eq!(media.len(), 22);
    for (path, expected) in media {
        let uri = Uri::new(&format!("ferres://WinUIGallery/{path}"), UriKind::Absolute).unwrap();
        let mut actual = Vec::new();
        AssetLoader::open(&uri, None).unwrap().read_to_end(&mut actual).unwrap();
        assert_eq!(&actual, expected, "{path} resolves its original source bytes");
        if path.ends_with(".jpg") {
            assert!(actual.starts_with(&[0xff, 0xd8, 0xff]), "{path}");
        } else if path.ends_with(".gif") {
            assert!(actual.starts_with(b"GIF8"), "{path}");
        } else {
            assert!(path.ends_with(".svg"), "Unexpected media type: {path}");
            assert!(std::str::from_utf8(&actual).unwrap().contains("<svg"));
        }
    }
}

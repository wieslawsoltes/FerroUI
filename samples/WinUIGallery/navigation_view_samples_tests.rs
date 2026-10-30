//! Original NavigationView page interactions, separate from native control contracts.

use crate::navigation_view_samples::{
    CategoryEntry, SamplePage1, SamplePage2, SamplePage3, SamplePage4, SamplePage5, SamplePage6,
    SampleSettingsPage,
};
use crate::{examples, tests::start};
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::layout::Orientation;
use ferroui_base::{BoxedValue, Rect, Ref, Size};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Button, CheckBox, Control, Frame, NavigationView, NavigationViewBackButtonVisible,
    NavigationViewItem, NavigationViewItemHeader, NavigationViewItemSeparator,
    NavigationViewPaneDisplayMode, NavigationViewSelectionFollowsFocus, RadioButton,
    StackPanel, Symbol, SymbolIcon, TextBlock, TextBox, ToolTip,
};
use std::rc::Rc;

fn page() -> Ref<Control> {
    examples::create(examples::find("NavigationView").unwrap()).unwrap()
}

fn item(nav: &NavigationView, index: usize) -> Ref<NavigationViewItem> {
    Control::from_boxed(&nav.menu_items().get_at(index).unwrap())
        .unwrap()
        .cast()
        .unwrap()
}

fn text(value: Option<BoxedValue>) -> Option<String> {
    value.and_then(|value| value.downcast_ref::<String>().cloned())
}

fn invoke(item: &NavigationViewItem) {
    let mut key = KeyEventArgs::new();
    key.key = Key::Enter;
    key.set_routed_event(Some(InputElement::key_down_event()));
    item.raise_event(&key);
    assert!(key.handled(), "the native item consumes activation");
}

fn click_check(root: &Control, name: &str, checked: bool) {
    let checkbox = root.get_control::<CheckBox>(name);
    checkbox.set_is_checked(Some(checked));
    checkbox.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

fn assert_source_tile_heights(frame: &Frame, expected: usize) {
    let content = frame
        .current_page()
        .unwrap()
        .content()
        .as_ref()
        .and_then(Control::from_boxed)
        .unwrap();
    let tiles = content
        .get_logical_descendants()
        .filter_map(|child| child.cast::<Control>())
        .filter(|child| child.min_height() == 150.0)
        .count();
    assert_eq!(
        tiles, expected,
        "the source TileHeight resource reaches the real destination controls"
    );
}

#[test]
fn navigation_view_page_initializes_all_eight_source_compositions() {
    let _application = start();
    let root = page();
    for number in ["2", "4", "5", "6", "7", "8", "9"] {
        let nav = root.get_control::<NavigationView>(&format!("nvSample{number}"));
        let frame = root.get_control::<Frame>(&format!("contentFrame{number}"));
        assert!(nav.selected_item().is_some(), "nvSample{number}");
        assert_eq!(
            frame.current_source_page_type(),
            Some(SamplePage1::TYPE),
            "contentFrame{number}"
        );
        assert_eq!(frame.back_stack_depth(), 0);
    }
    let options = root.get_control::<NavigationView>("nvSample");
    assert!(options.selected_item().is_none());
    assert!(root
        .get_control::<Frame>("contentFrame")
        .current_page()
        .is_none());
    let left = root.get_control::<NavigationView>("nvSample5");
    assert_eq!(left.menu_items().count(), 4);
    assert_eq!(
        left.pane_display_mode(),
        NavigationViewPaneDisplayMode::Auto
    );
    for (index, symbol) in [
        Symbol::Play,
        Symbol::Save,
        Symbol::Refresh,
        Symbol::Download,
    ]
    .into_iter()
    .enumerate()
    {
        let item = item(&left, index);
        assert_eq!(
            text(item.content()),
            Some(format!("Menu Item{}", index + 1))
        );
        assert_eq!(text(item.tag()), Some(format!("SamplePage{}", index + 1)));
        assert_eq!(
            item.icon().unwrap().cast::<SymbolIcon>().unwrap().symbol(),
            symbol
        );
    }
    assert!(!root
        .get_control::<TextBlock>("NavigationViewError")
        .is_visible());
}

#[test]
fn navigation_view_source_selection_loads_distinct_pages_and_settings() {
    let _application = start();
    let root = page();
    let nav = root.get_control::<NavigationView>("nvSample5");
    let frame = root.get_control::<Frame>("contentFrame5");
    for (index, expected) in [
        SamplePage1::TYPE,
        SamplePage2::TYPE,
        SamplePage3::TYPE,
        SamplePage4::TYPE,
    ]
    .into_iter()
    .enumerate()
    {
        invoke(&item(&nav, index));
        assert_eq!(frame.current_source_page_type(), Some(expected));
        assert_eq!(
            text(nav.header()),
            Some(format!("Sample Page {}", index + 1))
        );
        assert_source_tile_heights(&frame, [5, 1, 5, 6][index]);
    }
    invoke(&nav.settings_item());
    assert_eq!(
        frame.current_source_page_type(),
        Some(SampleSettingsPage::TYPE)
    );
    // The original settings branch leaves the previous header in place.
    assert_eq!(text(nav.header()).as_deref(), Some("Sample Page 4"));
    assert_eq!(frame.back_stack_depth(), 4);
}

#[test]
fn navigation_view_top_and_tabs_preserve_actual_source_targets_and_history() {
    let _application = start();
    let root = page();
    let nav = root.get_control::<NavigationView>("nvSample6");
    assert_eq!(nav.pane_display_mode(), NavigationViewPaneDisplayMode::Top);
    assert_eq!(text(item(&nav, 3).tag()).as_deref(), Some("SamplePage3"));
    invoke(&item(&nav, 3));
    assert_eq!(
        root.get_control::<Frame>("contentFrame6")
            .current_source_page_type(),
        Some(SamplePage3::TYPE)
    );
    assert_eq!(text(nav.header()).as_deref(), Some("This is Header Text"));
    let tabs = root.get_control::<NavigationView>("nvSample7");
    assert_eq!(
        tabs.selection_follows_focus(),
        NavigationViewSelectionFollowsFocus::Enabled
    );
    assert_eq!(
        tabs.is_back_button_visible(),
        NavigationViewBackButtonVisible::Collapsed
    );
    invoke(&item(&tabs, 1));
    let frame = root.get_control::<Frame>("contentFrame7");
    assert_eq!(frame.current_source_page_type(), Some(SamplePage2::TYPE));
    // The running Gallery calls Navigate, although its instructional snippet
    // recommends NavigateToType with navigation-stack recording disabled.
    assert_eq!(frame.back_stack_depth(), 1);
}

#[test]
fn navigation_view_category_template_preserves_models_headers_and_separator() {
    let _application = start();
    let root = page();
    let nav = root.get_control::<NavigationView>("nvSample4");
    let source = nav.menu_items_source().unwrap();
    assert_eq!(source.count(), 6);
    assert_eq!(
        nav.menu_items().count(),
        0,
        "the source owns the data; MenuItems stays separate"
    );
    let separator = nav.container_from_menu_item(&source.get_at(2)).unwrap();
    assert!(separator.cast::<NavigationViewItemSeparator>().is_some());
    let header = nav.container_from_menu_item(&source.get_at(3)).unwrap();
    assert!(header.cast::<NavigationViewItemHeader>().is_some());
    assert_eq!(text(header.content()).as_deref(), Some("More categories"));
    for (index, name, glyph, tooltip) in [
        (0, "Category 1", Symbol::Home, "This is category 1"),
        (1, "Category 2", Symbol::Keyboard, "This is category 2"),
        (4, "Category 3", Symbol::Library, "This is category 3"),
        (5, "Category 4", Symbol::Mail, "This is category 4"),
    ] {
        let model = source.get_at(index).unwrap();
        assert!(matches!(
            model.downcast_ref::<CategoryEntry>(),
            Some(CategoryEntry::Category { .. })
        ));
        let container = nav
            .container_from_menu_item(&Some(model.clone()))
            .unwrap()
            .cast::<NavigationViewItem>()
            .unwrap();
        assert_eq!(text(container.content()).as_deref(), Some(name));
        assert_eq!(text(ToolTip::get_tip(&container)).as_deref(), Some(tooltip));
        assert_eq!(
            container
                .icon()
                .unwrap()
                .cast::<SymbolIcon>()
                .unwrap()
                .symbol(),
            glyph
        );
        invoke(&container);
        assert!(Rc::ptr_eq(&nav.selected_item().unwrap(), &model));
        assert_eq!(
            root.get_control::<Frame>("contentFrame4")
                .current_source_page_type(),
            Some(SamplePage1::TYPE)
        );
        assert_eq!(
            text(nav.header()),
            Some(format!("Sample Page {}", name.chars().last().unwrap()))
        );
    }
}

#[test]
fn navigation_view_footer_uses_the_same_selection_and_original_destinations() {
    let _application = start();
    let root = page();
    let nav = root.get_control::<NavigationView>("nvSample9");
    assert!(!nav.is_settings_visible());
    assert_eq!(nav.menu_items().count(), 3);
    assert_eq!(nav.footer_menu_items().count(), 3);
    for (index, label, expected) in [
        (0, "Account", SamplePage4::TYPE),
        (1, "Your Cart", SamplePage5::TYPE),
        (2, "Help", SamplePage5::TYPE),
    ] {
        let value = nav.footer_menu_items().get_at(index).unwrap();
        let footer = Control::from_boxed(&value)
            .unwrap()
            .cast::<NavigationViewItem>()
            .unwrap();
        assert_eq!(text(footer.content()).as_deref(), Some(label));
        invoke(&footer);
        assert_eq!(
            root.get_control::<Frame>("contentFrame9")
                .current_source_page_type(),
            Some(expected)
        );
        assert!(Rc::ptr_eq(&nav.selected_item().unwrap(), &value));
        assert!(!item(&nav, 0).is_selected());
    }
    root.get_control::<RadioButton>("nvSample9Top")
        .set_is_checked(Some(true));
    assert_eq!(nav.pane_display_mode(), NavigationViewPaneDisplayMode::Top);
    assert!(!nav.is_pane_open());
    root.get_control::<RadioButton>("nvSample9Left")
        .set_is_checked(Some(true));
    assert_eq!(nav.pane_display_mode(), NavigationViewPaneDisplayMode::Left);
    assert!(nav.is_pane_open());
}

#[test]
fn navigation_view_hierarchy_navigates_children_and_keeps_action_parent_unselected() {
    let _application = start();
    let root = page();
    let nav = root.get_control::<NavigationView>("nvSample8");
    let frame = root.get_control::<Frame>("contentFrame8");
    let account = item(&nav, 1);
    invoke(&account);
    assert!(account.is_expanded());
    assert_eq!(frame.current_source_page_type(), Some(SamplePage2::TYPE));
    let mail = Control::from_boxed(&account.menu_items().get_at(0).unwrap())
        .unwrap()
        .cast::<NavigationViewItem>()
        .unwrap();
    invoke(&mail);
    assert_eq!(frame.current_source_page_type(), Some(SamplePage3::TYPE));
    let documents = item(&nav, 2);
    assert!(!documents.selects_on_invoked());
    invoke(&documents);
    assert!(documents.is_expanded());
    assert!(!documents.is_selected());
    assert_eq!(frame.current_source_page_type(), Some(SamplePage3::TYPE));
    for (index, expected, tile_count) in [(0, SamplePage5::TYPE, 3), (1, SamplePage6::TYPE, 2)] {
        let child = Control::from_boxed(&documents.menu_items().get_at(index).unwrap())
            .unwrap()
            .cast::<NavigationViewItem>()
            .unwrap();
        invoke(&child);
        assert_eq!(frame.current_source_page_type(), Some(expected));
        assert_source_tile_heights(&frame, tile_count);
    }
    root.get_control::<RadioButton>("nvSample8LeftCompact")
        .set_is_checked(Some(true));
    assert_eq!(
        nav.pane_display_mode(),
        NavigationViewPaneDisplayMode::LeftCompact
    );
    assert!(!nav.is_pane_open());
    root.get_control::<RadioButton>("nvSample8Top")
        .set_is_checked(Some(true));
    assert_eq!(nav.pane_display_mode(), NavigationViewPaneDisplayMode::Top);
    root.get_control::<RadioButton>("nvSample8Left")
        .set_is_checked(Some(true));
    assert!(nav.is_pane_open());
}

#[test]
fn navigation_view_api_options_update_native_state_and_header_bindings() {
    let _application = start();
    let root = page();
    let nav = root.get_control::<NavigationView>("nvSample");
    root.get_control::<TextBox>("headerText")
        .set_text(Some("Updated header"));
    assert_eq!(text(nav.header()).as_deref(), Some("Updated header"));
    root.get_control::<TextBox>("paneText")
        .set_text(Some("Updated pane"));
    assert_eq!(nav.pane_title(), "Updated pane");
    click_check(&root, "settingsCheck", false);
    assert!(!nav.is_settings_visible());
    click_check(&root, "visibleCheck", false);
    assert_eq!(
        nav.is_back_button_visible(),
        NavigationViewBackButtonVisible::Collapsed
    );
    click_check(&root, "visibleCheck", true);
    assert_eq!(
        nav.is_back_button_visible(),
        NavigationViewBackButtonVisible::Visible
    );
    click_check(&root, "enableCheck", true);
    assert!(nav.is_back_enabled());
    click_check(&root, "headerCheck", false);
    assert!(!nav.always_show_header());
    click_check(&root, "sffCheck", true);
    assert_eq!(
        nav.selection_follows_focus(),
        NavigationViewSelectionFollowsFocus::Enabled
    );
    invoke(&root.get_control::<NavigationViewItem>("SamplePage1Item"));
    assert_eq!(
        root.get_control::<TextBox>("headerText").text().as_deref(),
        Some("Sample Page 1")
    );
    click_check(&root, "suppressselectionCheck_Checked", true);
    invoke(&root.get_control::<NavigationViewItem>("SamplePage2Item"));
    assert_eq!(
        root.get_control::<Frame>("contentFrame")
            .current_source_page_type(),
        Some(SamplePage1::TYPE)
    );
    click_check(&root, "suppressselectionCheck_Checked", false);
    invoke(&root.get_control::<NavigationViewItem>("SamplePage2Item"));
    assert_eq!(
        root.get_control::<Frame>("contentFrame")
            .current_source_page_type(),
        Some(SamplePage2::TYPE)
    );
}

#[test]
fn navigation_view_api_search_custom_content_and_footer_options_keep_source_behavior() {
    let _application = start();
    let root = page();
    let nav = root.get_control::<NavigationView>("nvSample");
    let first_search = nav.auto_suggest_box().unwrap().downgrade();
    click_check(&root, "autoSuggestCheck", false);
    assert!(nav.auto_suggest_box().is_none());
    // The name scope can retain the initial markup object; callbacks do not
    // retain a replacement search field after it is removed.
    click_check(&root, "autoSuggestCheck", true);
    let replacement = nav.auto_suggest_box().unwrap();
    assert_eq!(
        AutomationProperties::get_name(&replacement).as_deref(),
        Some("search")
    );
    assert_eq!(
        replacement
            .query_icon()
            .unwrap()
            .cast::<SymbolIcon>()
            .unwrap()
            .symbol(),
        Symbol::Find
    );
    let replacement_weak = replacement.downgrade();
    drop(replacement);
    click_check(&root, "autoSuggestCheck", false);
    assert!(replacement_weak.upgrade().is_none());
    click_check(&root, "panemc_Check", true);
    assert!(root.get_control::<Control>("PaneHyperlink").is_visible());
    click_check(&root, "paneFooterCheck", true);
    let footer = root.get_control::<StackPanel>("FooterStackPanel");
    assert!(footer.is_visible());
    root.get_control::<RadioButton>("nvSampleTop")
        .set_is_checked(Some(true));
    assert_eq!(nav.pane_display_mode(), NavigationViewPaneDisplayMode::Top);
    assert_eq!(footer.orientation(), Orientation::Horizontal);
    assert!(!nav.is_pane_open());
    root.get_control::<RadioButton>("nvSampleLeft")
        .set_is_checked(Some(true));
    assert_eq!(footer.orientation(), Orientation::Vertical);
    assert!(nav.is_pane_open());
    click_check(&root, "panemc_Check", false);
    click_check(&root, "paneFooterCheck", false);
    assert!(!root.get_control::<Control>("PaneHyperlink").is_visible());
    assert!(!footer.is_visible());
    drop(footer);
    drop(nav);
    drop(root);
    assert!(first_search.upgrade().is_none());
}

#[test]
fn navigation_view_adaptive_example_changes_mode_at_its_native_threshold() {
    let _application = start();
    let root = page();
    let nav = root.get_control::<NavigationView>("nvSample2");
    root.measure(Size::new(1200.0, f64::INFINITY));
    root.arrange(Rect::new(0.0, 0.0, 1200.0, 6000.0));
    assert_eq!(nav.pane_display_mode(), NavigationViewPaneDisplayMode::Top);
    root.measure(Size::new(600.0, f64::INFINITY));
    root.arrange(Rect::new(0.0, 0.0, 600.0, 6000.0));
    assert_eq!(nav.pane_display_mode(), NavigationViewPaneDisplayMode::Auto);
    nav.set_compact_mode_threshold_width(500.0);
    assert_eq!(nav.pane_display_mode(), NavigationViewPaneDisplayMode::Top);
    assert_eq!(
        root.get_control::<Frame>("contentFrame2")
            .back_stack_depth(),
        0,
        "layout changes do not navigate again"
    );
}

#[test]
fn navigation_view_page_releases_controls_frames_and_category_models() {
    let _application = start();
    let root = page();
    let weak = root.downgrade();
    let navs = ["", "2", "4", "5", "6", "7", "8", "9"].map(|number| {
        root.get_control::<NavigationView>(&format!("nvSample{number}"))
            .downgrade()
    });
    let frames = ["", "2", "4", "5", "6", "7", "8", "9"].map(|number| {
        root.get_control::<Frame>(&format!("contentFrame{number}"))
            .downgrade()
    });
    let category = root
        .get_control::<NavigationView>("nvSample4")
        .selected_item()
        .unwrap();
    let weak_category = Rc::downgrade(&category);
    drop(category);
    drop(root);
    assert!(weak.upgrade().is_none());
    assert!(navs.iter().all(|nav| nav.upgrade().is_none()));
    assert!(frames.iter().all(|frame| frame.upgrade().is_none()));
    assert!(weak_category.upgrade().is_none());
}

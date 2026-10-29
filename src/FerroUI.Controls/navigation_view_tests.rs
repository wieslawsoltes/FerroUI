//! Tests of `NavigationView` and its automation peer (WinUI; docs/porting/winui-controls.md,
//! section 10.4), adapted from #286.
//!
//! NavigationView has no template of its own: the theme of the tests gives it the parts of the
//! template of the WinUI theme (`navigation_test_support`), and a navigation view gets them once it
//! is in a window. What the theme draws for a state is checked through the pseudo-classes the
//! control sets and the values of its `TemplateSettings`; the WinUI theme's tests check the
//! template itself.

use crate::automation::peers::ControlAutomationPeer;
use crate::automation::provider::ISelectionProvider;
use crate::navigation_test_support::{has_class, has_no_template_of_its_own, part, show, start, start_with_focus, Scene};
use crate::presenters::ContentPresenter;
use crate::primitives::{Popup, TemplatedControl};
use crate::templates::{FuncControlTemplate, FuncDataTemplate};
use crate::{
    reference_equals, AutoSuggestBox, Border, Button, ContentControl, Control, NavigationView,
    NavigationViewBackButtonVisible, NavigationViewDisplayMode, NavigationViewItem, NavigationViewItemBase,
    NavigationViewItemHeader, NavigationViewItemSeparator, NavigationViewPaneDisplayMode,
    NavigationViewSelectionFollowsFocus, SplitView, SplitViewDisplayMode, StackPanel, TextBlock,
};
use ferroui_base::collections::FerroList;
use ferroui_base::input::{FocusChangedEventArgs, InputElement, Key, KeyEventArgs, KeyModifiers, NavigationMethod};
use ferroui_base::media::FlowDirection;
use ferroui_base::styling::ControlTheme;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, Rect, Ref, Size};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn item(text: &str) -> Ref<NavigationViewItem> {
    let item = NavigationViewItem::new();
    item.set_content(Some(Rc::new(text.to_owned())));
    item
}

fn boxed(item: &NavigationViewItem) -> Option<BoxedValue> {
    Some(Control::boxed(item.to_ref()))
}

/// Measures and arranges the navigation view at `width`: the adaptive layout follows the width it
/// is measured with.
fn layout(nav: &NavigationView, width: f64) {
    nav.measure(Size::new(width, 500.0));
    nav.arrange(Rect::new(0.0, 0.0, width, 500.0));
}

/// The navigation view in a window of `width`: in the tree it gets its template from the theme.
fn attach(nav: &NavigationView, width: f64) -> Scene {
    show(nav, width, 500.0)
}

fn press(element: &InputElement, key: Key) -> KeyEventArgs {
    let mut args = KeyEventArgs::new();
    args.key = key;
    args.set_routed_event(Some(InputElement::key_down_event()));
    element.raise_event(&args);
    args
}

fn invoke(item: &NavigationViewItem) {
    press(item, Key::Enter);
}

fn raise_item_focus(item: &NavigationViewItem) {
    let mut args = FocusChangedEventArgs::new(InputElement::got_focus_event());
    args.new_focused_element = Some(item.to_ref().upcast());
    args.navigation_method = NavigationMethod::Directional;
    item.raise_event(&args);
}

fn host(nav: &NavigationView, name: &str) -> Ref<StackPanel> {
    part::<StackPanel>(nav, name)
}

fn content_of(nav: &NavigationView, name: &str) -> Option<BoxedValue> {
    part::<ContentControl>(nav, name).content()
}

#[test]
fn navigation_defaults_and_read_only_collections_are_instance_owned() {
    let _app = start();
    let nav = NavigationView::new();
    let other = NavigationView::new();
    assert!(nav.is_pane_open());
    assert_eq!(nav.display_mode(), NavigationViewDisplayMode::Minimal);
    assert_eq!(nav.pane_display_mode(), NavigationViewPaneDisplayMode::Auto);
    assert_eq!(nav.compact_mode_threshold_width(), 641.0);
    assert_eq!(nav.expanded_mode_threshold_width(), 1008.0);
    assert_eq!(nav.compact_pane_length(), 48.0);
    assert_eq!(nav.open_pane_length(), 320.0);
    assert!(nav.is_settings_visible() && nav.is_pane_visible() && nav.is_pane_toggle_button_visible());
    assert!(nav.always_show_header());
    assert!(!nav.is_back_enabled());
    assert_eq!(nav.is_back_button_visible(), NavigationViewBackButtonVisible::Auto);
    assert_eq!(nav.selection_follows_focus(), NavigationViewSelectionFollowsFocus::Disabled);
    assert!(nav.header().is_none() && nav.selected_item().is_none());
    assert_ne!(nav.menu_items(), other.menu_items());
    assert_ne!(nav.footer_menu_items(), other.footer_menu_items());
    assert!(!nav.settings_item().ptr_eq(&other.settings_item()));
    assert!(!nav.template_settings().ptr_eq(&other.template_settings()));
    assert!(NavigationView::menu_items_property().setter().is_none());
    assert!(NavigationView::footer_menu_items_property().setter().is_none());
    assert!(NavigationView::display_mode_property().setter().is_none());
    assert!(NavigationView::settings_item_property().setter().is_none());
    assert!(NavigationView::template_settings_property().setter().is_none());
    // The defaults of the template settings of WinUI's IDL.
    let settings = nav.template_settings();
    assert_eq!(settings.top_padding(), 0.0);
    assert!(!settings.overflow_button_visibility());
    assert!(settings.pane_toggle_button_visibility());
    assert!(!settings.back_button_visibility());
    assert!(!settings.top_pane_visibility());
    assert!(settings.left_pane_visibility());
    assert_eq!(settings.open_pane_length(), 320.0);
    // No template is built in Rust.
    assert!(has_no_template_of_its_own(NavigationView::TYPE));
    let _scene = attach(&nav, 800.0);
    assert!(nav.visual_children_count() > 0);
    assert!(part::<SplitView>(&nav, "RootSplitView").template().is_some());
    assert_eq!(settings.pane_toggle_button_width(), 48.0);
    assert_eq!(settings.smaller_pane_toggle_button_width(), 40.0);
}

#[test]
fn auto_layout_preserves_exact_threshold_edges_and_user_forced_close() {
    let _app = start();
    let nav = NavigationView::new();
    let _scene = attach(&nav, 640.0);
    for (width, mode, open) in [
        (640.0, NavigationViewDisplayMode::Minimal, false),
        (641.0, NavigationViewDisplayMode::Compact, false),
        (1007.0, NavigationViewDisplayMode::Compact, false),
        (1008.0, NavigationViewDisplayMode::Expanded, true),
    ] {
        layout(&nav, width);
        assert_eq!(nav.display_mode(), mode, "width {width}");
        assert_eq!(nav.is_pane_open(), open, "width {width}");
    }
    nav.set_is_pane_open(false);
    layout(&nav, 1100.0);
    assert!(!nav.is_pane_open(), "explicit close survives an expanded resize");
    nav.set_is_pane_open(true);
    layout(&nav, 900.0);
    assert_eq!(nav.display_mode(), NavigationViewDisplayMode::Compact);
    assert!(!nav.is_pane_open(), "expanded to compact closes");
    nav.set_is_pane_open(true);
    nav.invalidate_measure();
    layout(&nav, 900.0);
    assert!(nav.is_pane_open(), "ordinary remeasure must not undo a user opening");
}

/// The display modes drive the split view (`UpdateVisualStateForDisplayModeGroup`), and the top
/// pane takes the containers into its own hosts (`UpdateRepeaterItemsSource`).
#[test]
fn explicit_modes_drive_the_split_view_and_move_the_containers_to_the_top_pane() {
    let _app = start();
    let nav = NavigationView::new();
    let home = item("Home");
    nav.menu_items().add(boxed(&home));
    let _scene = attach(&nav, 800.0);
    let split = part::<SplitView>(&nav, "RootSplitView");
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    assert_eq!(nav.display_mode(), NavigationViewDisplayMode::Expanded);
    assert_eq!(split.display_mode(), SplitViewDisplayMode::CompactInline);
    assert!(has_class(&nav, ":expanded"));
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftMinimal);
    assert_eq!(nav.display_mode(), NavigationViewDisplayMode::Minimal);
    assert_eq!(split.display_mode(), SplitViewDisplayMode::Overlay);
    assert!(!nav.is_pane_open());
    assert!(has_class(&nav, ":minimal") || has_class(&nav, ":minimal-with-back-button"));
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    assert!(nav.is_pane_open());
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftCompact);
    assert_eq!(split.display_mode(), SplitViewDisplayMode::CompactOverlay);
    assert!(has_class(&nav, ":compact"));
    assert!(has_class(&nav, ":closed-compact"));
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Top);
    nav.update_layout();
    assert_eq!(nav.display_mode(), NavigationViewDisplayMode::Minimal);
    assert!(!nav.is_pane_open());
    assert!(!split.is_pane_open());
    assert!(has_class(&nav, ":top-navigation-minimal"));
    assert!(nav.template_settings().top_pane_visibility());
    assert!(!nav.template_settings().left_pane_visibility());
    assert!(!nav.template_settings().pane_toggle_button_visibility());
    assert_eq!(host(&nav, "MenuItemsHost").children().count(), 0);
    assert_eq!(host(&nav, "TopNavMenuItemsHost").children().get(0), home.clone().upcast::<Control>());
    // The settings item is the last footer item, without its text in the top pane.
    assert_eq!(
        host(&nav, "TopFooterMenuItemsHost").children().get(0),
        nav.settings_item().upcast::<Control>()
    );
    assert!(nav.settings_item().content().is_none());
    assert!(home.is_top_navigation());
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    nav.update_layout();
    assert!(nav.template_settings().left_pane_visibility());
    assert_eq!(host(&nav, "TopNavMenuItemsHost").children().count(), 0);
    assert_eq!(host(&nav, "MenuItemsHost").children().get(0), home.clone().upcast::<Control>());
    assert_eq!(host(&nav, "FooterMenuItemsHost").children().get(0), nav.settings_item().upcast::<Control>());
    assert_eq!(
        nav.settings_item().content().unwrap().downcast_ref::<String>().map(String::as_str),
        Some("Settings")
    );
    assert!(!home.is_top_navigation());
}

#[test]
fn pane_lengths_coerce_negative_values_without_changing_existing_split_view_defaults() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_compact_pane_length(-1.0);
    nav.set_open_pane_length(-5.0);
    nav.set_compact_mode_threshold_width(-3.0);
    nav.set_expanded_mode_threshold_width(-7.0);
    assert_eq!(nav.compact_pane_length(), 0.0);
    assert_eq!(nav.open_pane_length(), 0.0);
    assert_eq!(nav.compact_mode_threshold_width(), 0.0);
    assert_eq!(nav.expanded_mode_threshold_width(), 0.0);
    let existing = SplitView::new();
    assert!(!existing.is_pane_open());
    assert_eq!(existing.display_mode(), SplitViewDisplayMode::Overlay);
    assert_eq!(existing.open_pane_length(), 320.0);
    nav.set_open_pane_length(320.0);
    let _scene = attach(&nav, 800.0);
    // UpdateOpenPaneLength: the open pane is at most as wide as the navigation view.
    nav.set_open_pane_length(1000.0);
    assert_eq!(nav.template_settings().open_pane_length(), 800.0);
    nav.set_open_pane_length(f64::NAN);
    assert!(nav.open_pane_length().is_nan());
    assert!(nav.template_settings().open_pane_length().is_finite());
    assert_eq!(
        part::<SplitView>(&nav, "RootSplitView").open_pane_length(),
        nav.template_settings().open_pane_length()
    );
}

#[test]
fn invocation_precedes_selected_flags_and_selection_changed_and_repeat_is_not_a_change() {
    let _app = start();
    let nav = NavigationView::new();
    let a = item("A");
    let b = item("B");
    nav.menu_items().add(boxed(&a));
    nav.menu_items().add(boxed(&b));
    nav.set_selected_item(boxed(&a));
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    let wa = a.downgrade();
    let wb = b.downgrade();
    nav.item_invoked(move |args| {
        let a = wa.upgrade().unwrap();
        let b = wb.upgrade().unwrap();
        seen.borrow_mut().push(format!(
            "invoke:{}:{}:{}",
            args.invoked_item().unwrap().downcast_ref::<String>().unwrap(),
            a.is_selected(),
            b.is_selected()
        ));
    });
    let seen = events.clone();
    let wa = a.downgrade();
    let wb = b.downgrade();
    nav.selection_changed(move |args| {
        let a = wa.upgrade().unwrap();
        let b = wb.upgrade().unwrap();
        assert!(args.selected_item_container().unwrap().ptr_eq(&b));
        seen.borrow_mut().push(format!("select:{}:{}", a.is_selected(), b.is_selected()));
    });
    invoke(&b);
    assert_eq!(&*events.borrow(), &["invoke:B:true:false", "select:false:true"]);
    events.borrow_mut().clear();
    invoke(&b);
    assert_eq!(&*events.borrow(), &["invoke:B:false:true"]);
}

#[test]
fn invocation_redirected_by_application_commits_only_the_final_selection() {
    let _app = start();
    let nav = NavigationView::new();
    let a = item("A");
    let b = item("B");
    let c = item("C");
    for item in [&a, &b, &c] {
        nav.menu_items().add(boxed(item));
    }
    nav.set_selected_item(boxed(&a));
    let weak = nav.downgrade();
    let destination = c.downgrade();
    nav.item_invoked(move |_| {
        weak.upgrade().unwrap().set_selected_item(boxed(&destination.upgrade().unwrap()));
    });
    let changes = Rc::new(Cell::new(0));
    let seen = changes.clone();
    nav.selection_changed(move |_| seen.set(seen.get() + 1));
    invoke(&b);
    assert!(reference_equals(&nav.selected_item(), &boxed(&c)));
    assert!(!a.is_selected() && !b.is_selected() && c.is_selected());
    assert_eq!(changes.get(), 1);
}

#[test]
fn disabled_and_nonselecting_items_preserve_selection_and_programmatic_suppression_invokes() {
    let _app = start();
    let nav = NavigationView::new();
    let selected = item("Current");
    let action = item("Action");
    action.set_selects_on_invoked(false);
    nav.menu_items().add(boxed(&selected));
    nav.menu_items().add(boxed(&action));
    nav.set_selected_item(boxed(&selected));
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    nav.item_invoked(move |_| seen.set(seen.get() + 1));
    action.set_is_enabled(false);
    invoke(&action);
    assert_eq!(count.get(), 0);
    action.set_is_enabled(true);
    invoke(&action);
    assert_eq!(count.get(), 1);
    assert!(reference_equals(&nav.selected_item(), &boxed(&selected)));
    nav.set_selected_item(boxed(&action));
    assert_eq!(count.get(), 2);
    assert!(selected.is_selected() && !action.is_selected());
    assert!(reference_equals(&nav.selected_item(), &boxed(&selected)));
}

#[test]
fn source_collections_remain_live_and_preserve_model_and_container_identities() {
    let _app = start();
    let nav = NavigationView::new();
    let original = item("Original");
    nav.menu_items().add(boxed(&original));
    let first: BoxedValue = Rc::new("First".to_owned());
    let second: BoxedValue = Rc::new("Second".to_owned());
    let source = Rc::new(FerroList::<Option<BoxedValue>>::new());
    source.add(Some(first.clone()));
    nav.set_menu_items_source(Some(source.clone().into()));
    let first_container = nav.container_from_menu_item(&Some(first.clone())).unwrap();
    nav.menu_items().add(boxed(&item("While source is active")));
    assert_eq!(nav.menu_items().count(), 2);
    source.add(Some(second.clone()));
    assert!(nav.container_from_menu_item(&Some(first.clone())).unwrap().ptr_eq(&first_container));
    assert!(reference_equals(&nav.menu_item_from_container(&first_container), &Some(first.clone())));
    nav.set_selected_item(Some(first.clone()));
    source.remove_at(0);
    assert!(nav.selected_item().is_none());
    assert!(nav.container_from_menu_item(&Some(first)).is_none());
    nav.set_menu_items_source(None);
    assert!(nav.container_from_menu_item(&boxed(&original)).is_some());
    source.clear();
    assert!(nav.container_from_menu_item(&boxed(&original)).is_some(), "old source is disconnected");
}

#[test]
fn data_templates_can_generate_headings_separators_and_items_with_original_models() {
    let _app = start();
    let nav = NavigationView::new();
    let template = FuncDataTemplate::for_type::<String>(
        |data, _| {
            if data == "Heading" {
                let header = NavigationViewItemHeader::new();
                header.set_content(Some(Rc::new(data.clone())));
                Some(header.upcast())
            } else if data == "-" {
                Some(NavigationViewItemSeparator::new().upcast())
            } else {
                Some(item(data).upcast())
            }
        },
        false,
    );
    nav.set_menu_item_template(Some(template));
    for data in ["Heading", "-", "A"] {
        nav.menu_items().add(Some(Rc::new(data.to_owned())));
    }
    let heading: Option<BoxedValue> = Some(Rc::new("Heading".to_owned()));
    let separator: Option<BoxedValue> = Some(Rc::new("-".to_owned()));
    let data: Option<BoxedValue> = Some(Rc::new("A".to_owned()));
    assert!(nav.container_from_menu_item(&heading).unwrap().cast::<NavigationViewItemHeader>().is_some());
    assert!(nav.container_from_menu_item(&separator).unwrap().cast::<NavigationViewItemSeparator>().is_some());
    let container = nav.container_from_menu_item(&data).unwrap().cast::<NavigationViewItem>().unwrap();
    invoke(&container);
    assert!(reference_equals(&nav.selected_item(), &data));
}

#[test]
fn nested_selection_updates_ancestor_state_and_clears_after_child_removal() {
    let _app = start();
    let nav = NavigationView::new();
    let parent = item("Parent");
    let child = item("Child");
    parent.menu_items().add(boxed(&child));
    nav.menu_items().add(boxed(&parent));
    nav.set_selected_item(boxed(&child));
    assert!(child.is_selected() && parent.is_child_selected() && !parent.is_selected());
    assert!(nav.container_from_menu_item(&boxed(&child)).unwrap().ptr_eq(&child));
    parent.menu_items().clear();
    assert!(nav.selected_item().is_none());
    assert!(!parent.is_child_selected());
    child.set_is_selected(true);
    assert!(nav.selected_item().is_none(), "removed child no longer controls selection");
}

#[test]
fn expansion_events_support_lazy_children_and_footer_settings_selection() {
    let _app = start();
    let nav = NavigationView::new();
    let parent = item("Parent");
    parent.set_has_unrealized_children(true);
    nav.menu_items().add(boxed(&parent));
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    nav.expanding(move |args| {
        let parent = args.expanding_item_container().cast::<NavigationViewItem>().unwrap();
        assert!(reference_equals(&args.expanding_item(), &boxed(&parent)));
        if parent.menu_items().count() == 0 {
            parent.menu_items().add(boxed(&item("Lazy child")));
        }
        seen.borrow_mut().push("expanding");
    });
    let seen = events.clone();
    nav.collapsed(move |_| seen.borrow_mut().push("collapsed"));
    nav.expand(&parent);
    nav.collapse(&parent);
    assert_eq!(&*events.borrow(), &["expanding", "collapsed"]);
    let footer = item("Footer");
    nav.footer_menu_items().add(boxed(&footer));
    invoke(&footer);
    assert!(reference_equals(&nav.selected_item(), &boxed(&footer)));
    let settings_seen = Rc::new(Cell::new(false));
    let seen = settings_seen.clone();
    nav.selection_changed(move |args| seen.set(args.is_settings_selected()));
    invoke(&nav.settings_item());
    assert!(settings_seen.get());
    assert!(nav.settings_item().is_selected() && !footer.is_selected());
}

#[test]
fn escape_close_is_cancellable_but_explicit_close_keeps_its_source_semantics() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftMinimal);
    let _scene = attach(&nav, 500.0);
    nav.set_is_pane_open(true);
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    let cancellation = nav.pane_closing(move |args| {
        seen.borrow_mut().push("closing");
        args.set_cancel(true);
    });
    let seen = events.clone();
    nav.pane_closed(move || seen.borrow_mut().push("closed"));
    assert!(!press(&nav, Key::Escape).handled());
    assert!(nav.is_pane_open());
    assert_eq!(&*events.borrow(), &["closing"]);
    nav.set_is_pane_open(false);
    assert!(!nav.is_pane_open());
    assert_eq!(&*events.borrow(), &["closing", "closing", "closed"]);
    cancellation.dispose();
    events.borrow_mut().clear();
    nav.set_is_pane_open(true);
    assert!(press(&nav, Key::Escape).handled());
    assert_eq!(&*events.borrow(), &["closed"]);
}

#[test]
fn pane_opening_reentrancy_does_not_raise_a_stale_opened_event() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_is_pane_open(false);
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    let weak = nav.downgrade();
    nav.pane_opening(move || {
        seen.borrow_mut().push("opening");
        weak.upgrade().unwrap().set_is_pane_open(false);
    });
    let seen = events.clone();
    nav.pane_closing(move |_| seen.borrow_mut().push("closing"));
    let seen = events.clone();
    nav.pane_closed(move || seen.borrow_mut().push("closed"));
    let seen = events.clone();
    nav.pane_opened(move || seen.borrow_mut().push("opened"));
    nav.set_is_pane_open(true);
    assert!(!nav.is_pane_open());
    assert_eq!(&*events.borrow(), &["opening", "closing", "closed"]);
}

/// The content, the header, the pane header, custom content and footer, the auto suggest box and
/// the overlay reach the parts of the pane in use (`SwapPaneHeaderContent`,
/// `UpdateContentBindingsForPaneDisplayMode`); the header and the pane follow the states.
#[test]
fn header_content_search_overlay_and_pane_content_reach_the_parts_of_the_pane_in_use() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let _scene = attach(&nav, 800.0);
    let body = TextBlock::new();
    body.set_text(Some("Content"));
    nav.set_content(Some(Control::boxed(body.clone())));
    nav.set_header(Some(Rc::new("Header".to_owned())));
    nav.set_pane_title("Navigation".into());
    let pane_header = Border::new();
    let custom = Border::new();
    let footer = Border::new();
    let overlay = Border::new();
    nav.set_pane_header(Some(pane_header.clone().upcast()));
    nav.set_pane_custom_content(Some(custom.clone().upcast()));
    nav.set_pane_footer(Some(footer.clone().upcast()));
    nav.set_content_overlay(Some(overlay.clone().upcast()));
    assert!(has_class(&nav, ":auto-suggest-box-collapsed"));
    let search = AutoSuggestBox::new();
    nav.set_auto_suggest_box(Some(search.clone()));
    assert!(!has_class(&nav, ":auto-suggest-box-collapsed"));
    assert!(reference_equals(
        &part::<ContentPresenter>(&nav, "PART_ContentPresenter").content(),
        &Some(Control::boxed(body))
    ));
    assert_eq!(
        content_of(&nav, "HeaderContent").unwrap().downcast_ref::<String>().map(String::as_str),
        Some("Header")
    );
    assert!(reference_equals(&content_of(&nav, "PaneHeaderContentBorder"), &Some(Control::boxed(pane_header.clone()))));
    assert!(reference_equals(&content_of(&nav, "PaneCustomContentBorder"), &Some(Control::boxed(custom.clone()))));
    assert!(reference_equals(&content_of(&nav, "FooterContentBorder"), &Some(Control::boxed(footer.clone()))));
    assert!(reference_equals(&content_of(&nav, "PaneAutoSuggestBoxPresenter"), &Some(Control::boxed(search.clone()))));
    assert!(reference_equals(
        &part::<ContentPresenter>(&nav, "TopNavContentOverlayAreaGrid").content(),
        &Some(Control::boxed(overlay))
    ));
    // The pane title is the content of the pane toggle button while the button is shown.
    let title = part::<TextBlock>(&nav, "PaneTitleTextBlock");
    assert!(reference_equals(
        &part::<Button>(&nav, "TogglePaneButton").content(),
        &Some(Control::boxed(title.clone()))
    ));
    nav.set_always_show_header(false);
    assert!(has_class(&nav, ":header-collapsed"));
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftMinimal);
    assert!(!has_class(&nav, ":header-collapsed"), "the header shows in the minimal mode");
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Top);
    assert!(has_class(&nav, ":header-collapsed"));
    // The top pane takes the pane contents and the pane title.
    assert!(content_of(&nav, "PaneHeaderContentBorder").is_none());
    assert!(reference_equals(&content_of(&nav, "PaneHeaderOnTopPane"), &Some(Control::boxed(pane_header))));
    assert!(reference_equals(&content_of(&nav, "PaneCustomContentOnTopPane"), &Some(Control::boxed(custom))));
    assert!(reference_equals(&content_of(&nav, "PaneFooterOnTopPane"), &Some(Control::boxed(footer))));
    assert!(content_of(&nav, "PaneAutoSuggestBoxPresenter").is_none());
    assert!(reference_equals(&content_of(&nav, "TopPaneAutoSuggestBoxPresenter"), &Some(Control::boxed(search))));
    // UpdatePaneTitleFrameworkElementParents: in the top pane the title is shown only without the
    // pane toggle button, in PaneTitleOnTopPane.
    assert!(part::<Button>(&nav, "TogglePaneButton").content().is_none());
    assert!(content_of(&nav, "PaneTitleOnTopPane").is_none());
    nav.set_is_pane_toggle_button_visible(false);
    assert!(reference_equals(&content_of(&nav, "PaneTitleOnTopPane"), &Some(Control::boxed(title.clone()))));
    assert!(part::<ContentControl>(&nav, "PaneTitleOnTopPane").is_visible());
    nav.set_is_pane_toggle_button_visible(true);
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    assert!(content_of(&nav, "PaneTitleOnTopPane").is_none());
    assert!(reference_equals(&part::<Button>(&nav, "TogglePaneButton").content(), &Some(Control::boxed(title))));
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Top);
    nav.set_is_pane_visible(false);
    assert!(!nav.template_settings().top_pane_visibility());
    assert!(!nav.template_settings().left_pane_visibility());
    assert!(has_class(&nav, ":pane-collapsed"));
}

#[test]
fn replacing_template_disconnects_old_buttons_and_preserves_selected_containers() {
    let _app = start();
    let nav = NavigationView::new();
    let a = item("A");
    nav.menu_items().add(boxed(&a));
    nav.set_selected_item(boxed(&a));
    let _scene = attach(&nav, 800.0);
    let old_toggle = part::<Button>(&nav, "TogglePaneButton");
    let old_back = part::<Button>(&nav, "NavigationViewBackButton");
    let old_split = part::<SplitView>(&nav, "RootSplitView");
    nav.set_is_back_enabled(true);
    let backs = Rc::new(Cell::new(0));
    let seen = backs.clone();
    nav.back_requested(move |_| seen.set(seen.get() + 1));
    old_back.on_click();
    assert_eq!(backs.get(), 1);
    let replacement = FuncControlTemplate::for_type::<NavigationView>(|_, _| Border::new().upcast());
    nav.set_template(Some(replacement));
    nav.apply_template();
    let open = nav.is_pane_open();
    old_toggle.on_click();
    old_back.on_click();
    old_split.set_is_pane_open(!open);
    assert_eq!(nav.is_pane_open(), open);
    assert_eq!(backs.get(), 1);
    assert!(nav.container_from_menu_item(&boxed(&a)).unwrap().ptr_eq(&a));
    assert!(a.is_selected());
}

#[test]
fn selection_peer_reports_the_container_and_does_not_keep_the_control_alive() {
    let _app = start();
    let nav = NavigationView::new();
    let a = item("A");
    nav.menu_items().add(boxed(&a));
    nav.set_selected_item(boxed(&a));
    let peer = ControlAutomationPeer::create_peer_for_element(&nav);
    let selection = peer.get_provider::<dyn ISelectionProvider>().unwrap();
    assert!(!selection.can_select_multiple() && !selection.is_selection_required());
    assert_eq!(selection.get_selection().len(), 1);
    assert!(selection.get_selection()[0].ptr_eq(&ControlAutomationPeer::create_peer_for_element(&a)));
    let weak = nav.downgrade();
    drop(nav);
    assert!(weak.upgrade().is_none());
    assert!(selection.get_selection().is_empty());
}

#[test]
fn selected_item_state_is_stable_when_disposed_listeners_and_removed_items_are_retained() {
    let _app = start();
    let nav = NavigationView::new();
    let a = item("A");
    let b = item("B");
    nav.menu_items().add(boxed(&a));
    nav.menu_items().add(boxed(&b));
    let changes = Rc::new(Cell::new(0));
    let seen = changes.clone();
    let subscription = nav.selection_changed(move |_| seen.set(seen.get() + 1));
    nav.set_selected_item(boxed(&a));
    assert_eq!(changes.get(), 1);
    subscription.dispose();
    nav.set_selected_item(boxed(&b));
    assert_eq!(changes.get(), 1);
    nav.menu_items().remove(&boxed(&b));
    assert!(nav.selected_item().is_none());
    invoke(&b);
    b.set_is_selected(true);
    assert!(nav.selected_item().is_none());
}

/// `MenuItemContainerStyle` is applied at the style priority under a theme of the application, and
/// removed with the property. The theme of the pane toggle button (`PaneToggleButtonStyle`) is a
/// template binding of the WinUI theme, which its tests check.
#[test]
fn container_styles_preserve_local_priority_and_clear_their_own_values() {
    let _app = start();
    let nav = NavigationView::new();
    let a = item("A");
    let b = item("B");
    let local = ControlTheme::for_type::<NavigationViewItem>();
    a.set_theme(Some(local.clone()));
    nav.menu_items().add(boxed(&a));
    nav.menu_items().add(boxed(&b));
    let menu_theme = ControlTheme::for_type::<NavigationViewItemBase>();
    nav.set_menu_item_container_style(Some(menu_theme.clone()));
    assert_eq!(a.theme(), Some(local.clone()));
    assert_eq!(b.theme(), Some(menu_theme));
    nav.set_menu_item_container_style(None);
    assert_eq!(a.theme(), Some(local));
    assert!(b.theme().is_none());
}

#[test]
fn footer_source_switching_and_header_template_reach_the_parts() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let source = Rc::new(FerroList::<Option<BoxedValue>>::new());
    let footer = item("Footer source");
    source.add(boxed(&footer));
    nav.set_footer_menu_items_source(Some(source.clone().into()));
    assert!(nav.container_from_menu_item(&boxed(&footer)).unwrap().ptr_eq(&footer));
    nav.footer_menu_items().add(boxed(&item("Manual footer")));
    assert_eq!(nav.footer_menu_items().count(), 1);
    nav.set_selected_item(boxed(&footer));
    source.clear();
    assert!(nav.selected_item().is_none());
    nav.set_footer_menu_items_source(None);
    assert!(nav.container_from_menu_item(&nav.footer_menu_items().get_at(0)).is_some());
    nav.set_header(Some(Rc::new("Name".to_owned())));
    nav.set_header_template(Some(FuncDataTemplate::for_type::<String>(
        |value, _| {
            let text = TextBlock::new();
            text.set_text(Some(&format!("Header: {value}")));
            Some(text.upcast())
        },
        false,
    )));
    let _scene = attach(&nav, 800.0);
    let header = part::<ContentControl>(&nav, "HeaderContent");
    assert_eq!(header.content().unwrap().downcast_ref::<String>().map(String::as_str), Some("Name"));
    assert!(header.content_template().is_some());
    // The manual footer item is in the footer host, before the settings item.
    let footer_host = host(&nav, "FooterMenuItemsHost");
    assert_eq!(footer_host.children().count(), 2);
    assert_eq!(footer_host.children().get(1), nav.settings_item().upcast::<Control>());
}

#[test]
fn focus_following_raises_invocation_once_and_keyboard_navigation_moves_focus() {
    let _app = start_with_focus();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let a = item("A");
    let b = item("B");
    nav.menu_items().add(boxed(&a));
    nav.menu_items().add(boxed(&b));
    let _scene = attach(&nav, 800.0);
    // Activating the window can focus the first item: move away before the selection follows the
    // focus, so that the next change of the focus is deterministic.
    b.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
    nav.set_selection_follows_focus(NavigationViewSelectionFollowsFocus::Enabled);
    assert!(nav.template_settings().single_selection_follows_focus());
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    nav.item_invoked(move |_| seen.set(seen.get() + 1));
    a.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
    assert!(a.is_selected());
    assert_eq!(count.get(), 1);
    assert!(press(&a, Key::Down).handled());
    assert!(b.is_focused() && b.is_selected());
    assert_eq!(count.get(), 2);
    b.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
    assert_eq!(count.get(), 2);
}

#[test]
fn mode_and_pane_events_expose_exact_values_and_source_enum_numbers() {
    let _app = start();
    assert_eq!(
        [
            NavigationViewDisplayMode::Minimal as i32,
            NavigationViewDisplayMode::Compact as i32,
            NavigationViewDisplayMode::Expanded as i32
        ],
        [0, 1, 2]
    );
    assert_eq!(
        [
            NavigationViewPaneDisplayMode::Auto as i32,
            NavigationViewPaneDisplayMode::Left as i32,
            NavigationViewPaneDisplayMode::Top as i32,
            NavigationViewPaneDisplayMode::LeftCompact as i32,
            NavigationViewPaneDisplayMode::LeftMinimal as i32
        ],
        [0, 1, 2, 3, 4]
    );
    assert_eq!(
        [
            NavigationViewBackButtonVisible::Collapsed as i32,
            NavigationViewBackButtonVisible::Visible as i32,
            NavigationViewBackButtonVisible::Auto as i32
        ],
        [0, 1, 2]
    );
    assert_eq!(
        [NavigationViewSelectionFollowsFocus::Disabled as i32, NavigationViewSelectionFollowsFocus::Enabled as i32],
        [0, 1]
    );
    let args = crate::NavigationViewItemInvokedEventArgs::new();
    assert!(args.invoked_item().is_none() && args.invoked_item_container().is_none() && !args.is_settings_invoked());
    let nav = NavigationView::new();
    let _scene = attach(&nav, 800.0);
    let modes = Rc::new(RefCell::new(Vec::new()));
    let seen = modes.clone();
    nav.display_mode_changed(move |args| seen.borrow_mut().push(args.display_mode()));
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftMinimal);
    assert_eq!(&*modes.borrow(), &[NavigationViewDisplayMode::Expanded, NavigationViewDisplayMode::Minimal]);
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    nav.pane_opening(move || seen.borrow_mut().push("opening"));
    let seen = events.clone();
    nav.pane_opened(move || seen.borrow_mut().push("opened"));
    nav.set_is_pane_open(true);
    assert_eq!(&*events.borrow(), &["opening", "opened"]);
    let settings = nav.settings_item();
    let weak_settings = settings.downgrade();
    let invoked = Rc::new(Cell::new(false));
    let seen = invoked.clone();
    nav.item_invoked(move |args| {
        assert!(args.is_settings_invoked());
        assert!(args.invoked_item_container().unwrap().ptr_eq(&weak_settings.upgrade().unwrap()));
        seen.set(true);
    });
    invoke(&settings);
    assert!(invoked.get());
}

#[test]
fn toggle_force_close_mode_reset_and_pane_visibility_follow_source_transitions() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let _scene = attach(&nav, 1100.0);
    layout(&nav, 1100.0);
    part::<Button>(&nav, "TogglePaneButton").on_click();
    assert!(!nav.is_pane_open());
    layout(&nav, 1200.0);
    assert!(!nav.is_pane_open());
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Auto);
    assert!(nav.is_pane_open(), "a mode assignment resets the force-close flag");
    nav.set_is_pane_visible(false);
    assert!(!nav.is_pane_open());
    assert_eq!(part::<SplitView>(&nav, "RootSplitView").display_mode(), SplitViewDisplayMode::CompactOverlay);
    nav.set_is_pane_visible(true);
    assert!(nav.is_pane_open());
    nav.set_is_pane_toggle_button_visible(false);
    assert!(!nav.template_settings().pane_toggle_button_visibility());
    assert!(!part::<Button>(&nav, "TogglePaneButton").is_visible());
    nav.set_is_back_button_visible(NavigationViewBackButtonVisible::Collapsed);
    assert!(!nav.template_settings().back_button_visibility());
    assert!(has_class(&nav, ":back-button-collapsed"));
    assert!(!part::<Button>(&nav, "NavigationViewBackButton").is_visible());
    nav.set_is_settings_visible(false);
    assert!(!nav.settings_item().is_visible());
    nav.set_is_settings_visible(true);
    assert!(nav.settings_item().is_visible());
    nav.set_selected_item(nav.settings_item().content());
    assert!(nav.settings_item().is_selected());
    assert!(nav.container_from_menu_item(&nav.selected_item()).unwrap().ptr_eq(&nav.settings_item()));
}

/// `UpdateBackAndCloseButtonsVisibility`: the back button is shown in the inline modes (with the
/// row WinUI sizes in code), and the close button replaces it in an open minimal pane.
#[test]
fn back_and_close_buttons_follow_the_display_mode() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let _scene = attach(&nav, 1100.0);
    assert!(nav.template_settings().back_button_visibility());
    assert!(has_class(&nav, ":back-button-row-visible"));
    let close = part::<Button>(&nav, "NavigationViewCloseButton");
    assert!(!close.is_visible());
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftMinimal);
    assert!(!has_class(&nav, ":back-button-row-visible"), "the overlay pane has no back button row");
    nav.set_is_pane_open(true);
    assert!(!nav.template_settings().back_button_visibility());
    assert!(close.is_visible());
    assert!(has_class(&nav, ":minimal-with-back-button"));
    assert!(!has_class(&nav, ":pane-not-overlaying"), "an open minimal pane overlays the content");
    close.on_click();
    assert!(!nav.is_pane_open());
    assert!(!close.is_visible());
    assert!(has_class(&nav, ":pane-not-overlaying"));
}

#[test]
fn compact_context_restores_expansion_and_public_expand_ignores_leaf_items() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let parent = item("Parent");
    let leaf = item("Leaf");
    parent.menu_items().add(boxed(&leaf));
    nav.menu_items().add(boxed(&parent));
    let _scene = attach(&nav, 800.0);
    nav.expand(&parent);
    assert!(parent.is_expanded());
    nav.expand(&leaf);
    assert!(!leaf.is_expanded());
    let foreign = item("Unattached parent");
    foreign.set_has_unrealized_children(true);
    nav.expand(&foreign);
    assert!(foreign.is_expanded());
    nav.collapse(&foreign);
    assert!(!foreign.is_expanded());
    let empty_source = Rc::new(FerroList::<Option<BoxedValue>>::new());
    foreign.set_menu_items_source(Some(empty_source.into()));
    nav.expand(&foreign);
    assert!(!foreign.is_expanded(), "an active empty source takes precedence over HasUnrealizedChildren");
    foreign.set_menu_items_source(None);
    nav.expand(&foreign);
    assert!(foreign.is_expanded());
    nav.collapse(&foreign);
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftCompact);
    assert!(!parent.is_expanded());
    nav.set_is_pane_open(true);
    assert!(parent.is_expanded());
    let seen = Rc::new(Cell::new(false));
    let observed = seen.clone();
    let weak = parent.downgrade();
    nav.collapsed(move |args| {
        let parent = weak.upgrade().unwrap();
        assert!(args.collapsed_item_container().ptr_eq(&parent));
        assert!(reference_equals(&args.collapsed_item(), &boxed(&parent)));
        observed.set(true);
    });
    nav.collapse(&parent);
    assert!(seen.get());
}

/// A closed compact pane collapses the top level header text (`HeaderTextCollapsed`), marks the
/// top level items (`ClosedCompactAndTopLevelItem`) and the separators (`HorizontalLineCompact`);
/// the top pane turns the separators (`VerticalLine`). #286 set the sizes of the parts in code.
#[test]
fn closed_left_and_minimal_panes_propagate_the_item_context_and_the_expansion_memory() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let header = NavigationViewItemHeader::new();
    header.set_content(Some(Rc::new("Group".to_owned())));
    let separator = NavigationViewItemSeparator::new();
    let parent = item("Parent");
    parent.menu_items().add(boxed(&item("Child")));
    nav.menu_items().add(Some(Control::boxed(header.clone())));
    nav.menu_items().add(boxed(&parent));
    nav.menu_items().add(Some(Control::boxed(separator.clone())));
    let _scene = attach(&nav, 900.0);
    assert!(!has_class(&header, ":header-text-collapsed"));
    assert!(!has_class(&parent, ":closed-compact-and-top-level-item"));
    assert!(!has_class(&separator, ":horizontal-line-compact"));
    nav.expand(&parent);
    nav.set_is_pane_open(false);
    assert_eq!(nav.display_mode(), NavigationViewDisplayMode::Expanded);
    assert!(has_class(&nav, ":closed-compact"));
    assert!(has_class(&header, ":header-text-collapsed"));
    assert!(has_class(&parent, ":closed-compact-and-top-level-item"));
    assert!(has_class(&separator, ":horizontal-line-compact"));
    assert!(!parent.is_expanded());
    nav.set_is_pane_open(true);
    assert!(parent.is_expanded());
    assert!(!has_class(&header, ":header-text-collapsed"));
    assert!(!has_class(&parent, ":closed-compact-and-top-level-item"));
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftMinimal);
    assert!(!parent.is_expanded());
    nav.set_is_pane_open(true);
    assert!(parent.is_expanded(), "Minimal mode remembers expansion too");
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Top);
    assert!(has_class(&separator, ":vertical-line"));
    assert!(has_class(&header, ":top-mode"));
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    assert!(!has_class(&separator, ":vertical-line"));
    assert!(!has_class(&header, ":top-mode"));
}

#[test]
fn focus_following_reaches_the_flyout_hierarchy_without_invoking_ancestors() {
    let _app = start_with_focus();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Top);
    let anchor = item("Anchor");
    let parent = item("Parent");
    let child = item("Child");
    let action = item("Action");
    action.set_selects_on_invoked(false);
    let disabled = item("Disabled");
    disabled.set_is_enabled(false);
    child.menu_items().add(boxed(&item("Grandchild")));
    parent.menu_items().add(boxed(&child));
    parent.menu_items().add(boxed(&action));
    parent.menu_items().add(boxed(&disabled));
    nav.menu_items().add(boxed(&anchor));
    nav.menu_items().add(boxed(&parent));
    nav.set_selected_item(boxed(&anchor));
    let scene = attach(&nav, 800.0);
    nav.expand(&parent);
    scene.0.update_layout();
    child.apply_template();
    assert!(part::<Popup>(&parent, "ChildrenFlyout").is_open());
    assert!(action.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    let weak_anchor = anchor.downgrade();
    let weak_child = child.downgrade();
    nav.item_invoked(move |args| {
        let anchor = weak_anchor.upgrade().unwrap();
        let child = weak_child.upgrade().unwrap();
        assert!(args.invoked_item_container().unwrap().ptr_eq(&child));
        seen.borrow_mut().push(format!("invoke:{}:{}", anchor.is_selected(), child.is_selected()));
    });
    let seen = events.clone();
    let weak_anchor = anchor.downgrade();
    let weak_child = child.downgrade();
    nav.selection_changed(move |args| {
        let anchor = weak_anchor.upgrade().unwrap();
        let child = weak_child.upgrade().unwrap();
        assert!(args.selected_item_container().unwrap().ptr_eq(&child));
        seen.borrow_mut().push(format!("select:{}:{}", anchor.is_selected(), child.is_selected()));
    });
    nav.set_selection_follows_focus(NavigationViewSelectionFollowsFocus::Enabled);
    assert!(child.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
    assert_eq!(&*events.borrow(), &["invoke:true:false", "select:false:true"]);
    assert!(parent.is_expanded() && child.is_expanded());
    assert!(parent.is_child_selected() && !parent.is_selected());
    child.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
    assert_eq!(events.borrow().len(), 2);
    assert!(action.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
    assert!(!disabled.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
    assert!(child.is_selected() && !action.is_selected());
    assert_eq!(events.borrow().len(), 2);
    // A click invokes first, then focuses (what the release over the item does; the child is in a
    // popup, whose window the bounds hit tester of the tests does not reach). Following that
    // focus must not invoke the selected child a second time.
    child.invoke();
    child.focus_with(NavigationMethod::Pointer, KeyModifiers::NONE);
    assert_eq!(&*events.borrow(), &["invoke:true:false", "select:false:true", "invoke:false:true"]);
    nav.set_selection_follows_focus(NavigationViewSelectionFollowsFocus::Disabled);
    anchor.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
    assert!(child.is_selected());
    assert_eq!(events.borrow().len(), 3);
}

#[test]
fn focus_subscriptions_preserve_redirects_and_detach_after_retemplate_removal_and_drop() {
    let _app = start();
    let nav = NavigationView::new();
    let anchor = item("Anchor");
    let parent = item("Parent");
    let child = item("Child");
    parent.menu_items().add(boxed(&child));
    let source = Rc::new(FerroList::<Option<BoxedValue>>::new());
    source.add(boxed(&anchor));
    source.add(boxed(&parent));
    nav.set_menu_items_source(Some(source.clone().into()));
    nav.set_selected_item(boxed(&anchor));
    nav.set_selection_follows_focus(NavigationViewSelectionFollowsFocus::Enabled);
    let invocations = Rc::new(Cell::new(0));
    let observed = invocations.clone();
    let weak_owner = nav.downgrade();
    let weak_anchor = anchor.downgrade();
    nav.item_invoked(move |_| {
        observed.set(observed.get() + 1);
        weak_owner.upgrade().unwrap().set_selected_item(boxed(&weak_anchor.upgrade().unwrap()));
    });
    nav.apply_template();
    // Each new template replaces the subscriptions. The redirect above leaves Child unselected,
    // which shows duplicate handlers instead of hiding them behind the guard of the selection.
    for _ in 0..3 {
        nav.set_template(Some(FuncControlTemplate::for_type::<NavigationView>(|_, _| Border::new().upcast())));
        nav.apply_template();
    }
    // A retained realized child can get the focus before it has a visual route through the
    // owner; its own subscription still finds NavigationView.
    raise_item_focus(&child);
    assert_eq!(invocations.get(), 1);
    assert!(press(&child, Key::Home).handled());
    assert!(anchor.is_selected() && !child.is_selected());
    assert!(reference_equals(&nav.selected_item(), &boxed(&anchor)));
    source.remove_at(1);
    raise_item_focus(&child);
    assert!(!press(&child, Key::Home).handled());
    assert_eq!(invocations.get(), 1);
    assert!(reference_equals(&nav.selected_item(), &boxed(&anchor)));
    source.add(boxed(&parent));
    raise_item_focus(&child);
    assert!(press(&child, Key::Home).handled());
    assert_eq!(invocations.get(), 2);
    assert!(anchor.is_selected() && !child.is_selected());
    let weak_owner = nav.downgrade();
    drop(nav);
    assert!(weak_owner.upgrade().is_none());
    // The source and every container outlive their former owner.
    raise_item_focus(&child);
    assert!(!press(&child, Key::Home).handled());
    source.add(boxed(&item("Later")));
    assert_eq!(invocations.get(), 2);
    assert!(weak_owner.upgrade().is_none());
}

#[test]
fn flyout_directional_keys_follow_the_vertical_tree_and_top_roots_follow_flow_direction() {
    let _app = start_with_focus();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Top);
    nav.set_is_settings_visible(false);
    let before = item("Before");
    let parent = item("Parent");
    let after = item("After");
    let first = item("First");
    let grandchild = item("Grandchild");
    first.menu_items().add(boxed(&grandchild));
    let header = NavigationViewItemHeader::new();
    header.set_content(Some(Rc::new("Group".to_owned())));
    let disabled = item("Disabled");
    disabled.set_is_enabled(false);
    let hidden = item("Hidden");
    hidden.set_is_visible(false);
    let last = item("Last");
    parent.menu_items().add(boxed(&first));
    parent.menu_items().add(Some(Control::boxed(header)));
    parent.menu_items().add(boxed(&disabled));
    parent.menu_items().add(boxed(&hidden));
    parent.menu_items().add(boxed(&last));
    for item in [&before, &parent, &after] {
        nav.menu_items().add(boxed(item));
    }
    let invocations = Rc::new(Cell::new(0));
    let observed = invocations.clone();
    nav.item_invoked(move |_| observed.set(observed.get() + 1));
    let scene = show(&nav, 900.0, 500.0);
    nav.expand(&parent);
    scene.0.update_layout();
    assert!(part::<Popup>(&parent, "ChildrenFlyout").is_open());
    assert!(first.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
    assert!(press(&first, Key::Down).handled());
    assert!(last.is_focused(), "hidden, disabled and header rows are skipped");
    assert!(press(&last, Key::Up).handled());
    assert!(first.is_focused());
    nav.expand(&first);
    first.update_layout();
    assert!(press(&first, Key::Down).handled());
    assert!(grandchild.is_focused());
    assert!(press(&grandchild, Key::Down).handled());
    assert!(last.is_focused());
    assert!(press(&last, Key::Up).handled());
    assert!(grandchild.is_focused());
    assert!(press(&grandchild, Key::Up).handled());
    assert!(first.is_focused());
    assert!(press(&first, Key::Down).handled());
    assert!(grandchild.is_focused());
    assert!(press(&grandchild, Key::End).handled());
    assert!(last.is_focused());
    assert!(press(&last, Key::Home).handled());
    assert!(first.is_focused());
    assert!(parent.is_expanded());
    assert!(before.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
    assert!(press(&before, Key::Right).handled());
    assert!(parent.is_focused());
    assert!(press(&parent, Key::Right).handled());
    assert!(after.is_focused(), "Top root arrows move between roots");
    assert!(press(&after, Key::Left).handled());
    assert!(parent.is_focused());
    nav.set_flow_direction(FlowDirection::RightToLeft);
    assert!(press(&parent, Key::Left).handled());
    assert!(after.is_focused());
    assert!(press(&after, Key::Right).handled());
    assert!(parent.is_focused());
    assert!(press(&parent, Key::Down).handled());
    assert!(first.is_focused(), "Down enters the flyout of the expanded top root");
    assert!(press(&first, Key::Right).handled());
    assert!(!first.is_expanded(), "the item collapses its own children in a right to left flow");
    assert!(parent.is_expanded());
    assert_eq!(invocations.get(), 0);
    assert!(nav.selected_item().is_none());
}

#[test]
fn clearing_an_attached_template_disconnects_retained_parts_and_restores_the_content() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    nav.set_is_back_enabled(true);
    let selected = item("Selected");
    nav.menu_items().add(boxed(&selected));
    nav.set_selected_item(boxed(&selected));
    let content = Border::new();
    nav.set_content(Some(Control::boxed(content.clone())));
    let scene = attach(&nav, 800.0);
    let template = nav.template();
    let old_toggle = part::<Button>(&nav, "TogglePaneButton");
    let old_back = part::<Button>(&nav, "NavigationViewBackButton");
    let old_split = part::<SplitView>(&nav, "RootSplitView");
    let back_count = Rc::new(Cell::new(0));
    let observed = back_count.clone();
    nav.back_requested(move |_| observed.set(observed.get() + 1));
    nav.set_template(None);
    nav.apply_template();
    assert_eq!(nav.visual_children_count(), 0);
    let open = nav.is_pane_open();
    old_toggle.on_click();
    old_back.on_click();
    old_split.set_is_pane_open(!open);
    assert_eq!(nav.is_pane_open(), open);
    assert_eq!(back_count.get(), 0);
    nav.set_template(template);
    nav.apply_template();
    scene.0.update_layout();
    assert!(selected.is_selected());
    assert!(part::<ContentPresenter>(&nav, "PART_ContentPresenter").child().unwrap().ptr_eq(&content));
    part::<Button>(&nav, "NavigationViewBackButton").on_click();
    assert_eq!(back_count.get(), 1);
    old_back.on_click();
    assert_eq!(back_count.get(), 1);
}

#[test]
fn pending_selection_completes_once_for_the_current_model_with_or_without_a_container() {
    let _app = start();
    let nav = NavigationView::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    nav.selection_changed(move |args| observed.borrow_mut().push(args.clone()));
    let first: Option<BoxedValue> = Some(Rc::new("Not realized first".to_owned()));
    let second: Option<BoxedValue> = Some(Rc::new("Not realized second".to_owned()));
    nav.set_selected_item(first);
    nav.set_selected_item(second.clone());
    assert!(changes.borrow().is_empty());
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(changes.borrow().len(), 1);
    assert!(reference_equals(&changes.borrow()[0].selected_item(), &second));
    assert!(changes.borrow()[0].selected_item_container().is_none());

    // A container realized before the callback of the dispatcher completes the pending
    // notification at once, and only once.
    let realized: Option<BoxedValue> = Some(Rc::new("Realized before idle".to_owned()));
    nav.set_selected_item(realized.clone());
    nav.menu_items().add(realized.clone());
    assert_eq!(changes.borrow().len(), 2);
    assert!(reference_equals(&changes.borrow()[1].selected_item(), &realized));
    assert!(changes.borrow()[1].selected_item_container().unwrap().is_selected());
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(changes.borrow().len(), 2);

    // A newer clear prevents the older callback from raising an event.
    nav.set_selected_item(Some(Rc::new("Superseded by clear".to_owned())));
    nav.set_selected_item(None);
    assert_eq!(changes.borrow().len(), 3);
    assert!(changes.borrow()[2].selected_item().is_none());
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(changes.borrow().len(), 3);

    // A suppressed action restores the unresolved selection with its outstanding notification.
    let action = item("Action");
    action.set_selects_on_invoked(false);
    nav.menu_items().add(boxed(&action));
    nav.set_selected_item(second.clone());
    nav.set_selected_item(boxed(&action));
    assert!(reference_equals(&nav.selected_item(), &second));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(changes.borrow().len(), 4);
    assert!(reference_equals(&changes.borrow()[3].selected_item(), &second));
    assert!(changes.borrow()[3].selected_item_container().is_none());
}

#[test]
fn queued_selection_completion_does_not_keep_navigation_alive() {
    let _app = start();
    let nav = NavigationView::new();
    let changes = Rc::new(Cell::new(0));
    let observed = changes.clone();
    nav.selection_changed(move |_| observed.set(observed.get() + 1));
    nav.set_selected_item(Some(Rc::new("Not realized".to_owned())));
    let weak = nav.downgrade();
    drop(nav);
    assert!(weak.upgrade().is_none());
    Dispatcher::ui_thread().run_jobs(None);
    assert!(weak.upgrade().is_none());
    assert_eq!(changes.get(), 0);
}

#[test]
fn reentrant_menu_template_changes_rebuild_generated_containers_with_the_new_factory() {
    let _app = start();
    let nav = NavigationView::new();
    let replacement = FuncDataTemplate::for_type::<String>(|data, _| Some(item(&format!("New {data}")).upcast()), false);
    let weak = nav.downgrade();
    let changed = Rc::new(Cell::new(false));
    let change_once = changed.clone();
    let old = FuncDataTemplate::for_type::<String>(
        move |data, _| {
            if !change_once.replace(true) {
                weak.upgrade().unwrap().set_menu_item_template(Some(replacement.clone()));
            }
            Some(item(&format!("Old {data}")).upcast())
        },
        false,
    );
    nav.set_menu_item_template(Some(old));
    let model: Option<BoxedValue> = Some(Rc::new("Model".to_owned()));
    nav.menu_items().add(model.clone());
    assert!(changed.get());
    let container = nav.container_from_menu_item(&model).unwrap();
    assert_eq!(container.content().unwrap().downcast_ref::<String>().unwrap(), "New Model");
    assert!(reference_equals(&container.data_context(), &model));
    nav.set_selected_item(model.clone());
    assert!(container.is_selected());
    assert!(reference_equals(&nav.menu_item_from_container(&container), &model));
}

/// A new template takes the pane contents and the containers; the parts of the old one are
/// emptied (`Parts::detach`), so that the content of the application has one parent. #286's test of
/// the data templates of its own presenters is not taken: the content and the header reach the
/// template through template bindings.
#[test]
fn retemplating_moves_the_pane_contents_and_the_containers_to_the_new_parts() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let home = item("Home");
    nav.menu_items().add(boxed(&home));
    let pane_header = Border::new();
    nav.set_pane_header(Some(pane_header.clone().upcast()));
    let scene = attach(&nav, 800.0);
    let old_header = part::<ContentControl>(&nav, "PaneHeaderContentBorder");
    let old_menu = host(&nav, "MenuItemsHost");
    assert!(reference_equals(&old_header.content(), &Some(Control::boxed(pane_header.clone()))));
    assert_eq!(old_menu.children().get(0), home.clone().upcast::<Control>());
    nav.set_template(Some(crate::navigation_test_support::navigation_view_template()));
    nav.apply_template();
    scene.0.update_layout();
    assert!(old_header.content().is_none());
    assert_eq!(old_menu.children().count(), 0);
    let new_header = part::<ContentControl>(&nav, "PaneHeaderContentBorder");
    assert!(!new_header.ptr_eq(&old_header));
    assert!(reference_equals(&new_header.content(), &Some(Control::boxed(pane_header))));
    assert_eq!(host(&nav, "MenuItemsHost").children().get(0), home.upcast::<Control>());
}

#[test]
fn the_templated_parent_of_the_parts_is_the_navigation_view() {
    let _app = start();
    let nav = NavigationView::new();
    let _scene = attach(&nav, 800.0);
    let split = part::<SplitView>(&nav, "RootSplitView");
    let templated: Option<Ref<TemplatedControl>> = split.templated_parent().and_then(|parent| parent.cast());
    assert!(templated.is_some_and(|parent| parent.ptr_eq(&nav)));
}

//! Native NavigationView contracts anchored to the pinned WinUI implementation.

use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::collections::FerroList;
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, Key, KeyEventArgs, KeyModifiers, NavigationMethod,
};
use ferroui_base::layout::{ILayoutManager, Orientation};
use ferroui_base::media::FlowDirection;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::styling::ControlTheme;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, Rect, Ref, Size};
use ferroui_controls::automation::peers::ControlAutomationPeer;
use ferroui_controls::automation::provider::ISelectionProvider;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::{Popup, TemplatedControl};
use ferroui_controls::reference_equals;
use ferroui_controls::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions,
};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{
    AutoSuggestBox, NavigationView, NavigationViewBackButtonVisible, NavigationViewDisplayMode,
    NavigationViewItem, NavigationViewItemBase, NavigationViewItemHeader,
    NavigationViewItemSeparator, NavigationViewPaneDisplayMode,
    NavigationViewSelectionFollowsFocus,
};
use ferroui_controls::{
    Border, Button, Control, SplitView, SplitViewDisplayMode, StackPanel, TextBlock, Window,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct TestGlobalClock {
    subject: LightweightSubject<TimeSpan>,
    state: Cell<PlayState>,
}
impl IObservable<TimeSpan> for TestGlobalClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}
impl IClock for TestGlobalClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, value: PlayState) {
        self.state.set(value);
    }
}
impl IGlobalClock for TestGlobalClock {}
fn styled_services() -> TestServices {
    TestServices::styled_window().with_global_clock(Rc::new(TestGlobalClock {
        subject: LightweightSubject::new(),
        state: Cell::new(PlayState::Run),
    }))
}
fn start() -> UnitTestApplicationScope {
    ferroui_controls::register_types();
    UnitTestApplication::start(styled_services())
}
fn start_with_focus() -> UnitTestApplicationScope {
    ferroui_controls::register_types();
    let focus = TestServices::real_focus();
    let mut services = styled_services();
    services.keyboard_device = focus.keyboard_device;
    services.keyboard_navigation = focus.keyboard_navigation;
    services.input_manager = focus.input_manager;
    UnitTestApplication::start(services)
}
fn item(text: &str) -> Ref<NavigationViewItem> {
    let item = NavigationViewItem::new();
    item.set_content(Some(Rc::new(text.to_owned())));
    item
}
fn boxed(item: &NavigationViewItem) -> Option<BoxedValue> {
    Some(Control::boxed(item.to_ref()))
}
fn part<T: ferroui_base::ObjectType>(owner: &TemplatedControl, name: &str) -> Ref<T> {
    owner
        .get_template_descendants()
        .iter()
        .find(|part| part.name().as_deref() == Some(name))
        .unwrap_or_else(|| panic!("missing NavigationView part {name}"))
        .cast::<T>()
        .unwrap()
}
fn layout(nav: &NavigationView, width: f64) {
    nav.measure(Size::new(width, 500.0));
    nav.arrange(Rect::new(0.0, 0.0, width, 500.0));
}
fn attach(nav: &NavigationView, width: f64) -> Ref<Window> {
    let window = Window::new();
    window.set_width(width);
    window.set_height(500.0);
    window.set_content(Some(Control::boxed(nav.to_ref())));
    window.show();
    window.layout_manager().execute_layout_pass();
    window
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
    assert!(
        nav.is_settings_visible() && nav.is_pane_visible() && nav.is_pane_toggle_button_visible()
    );
    assert!(nav.always_show_header());
    assert!(!nav.is_back_enabled());
    assert_eq!(
        nav.is_back_button_visible(),
        NavigationViewBackButtonVisible::Auto
    );
    assert_eq!(
        nav.selection_follows_focus(),
        NavigationViewSelectionFollowsFocus::Disabled
    );
    assert!(nav.header().is_none() && nav.selected_item().is_none());
    assert_ne!(nav.menu_items(), other.menu_items());
    assert_ne!(nav.footer_menu_items(), other.footer_menu_items());
    assert!(!nav.settings_item().ptr_eq(&other.settings_item()));
    assert!(NavigationView::menu_items_property().setter().is_none());
    assert!(NavigationView::footer_menu_items_property()
        .setter()
        .is_none());
    assert!(NavigationView::display_mode_property().setter().is_none());
    assert!(NavigationView::settings_item_property().setter().is_none());
    assert!(!nav.has_local_value(TemplatedControl::template_property()));
    nav.apply_template();
    assert!(nav.visual_children_count() > 0);
    assert!(part::<SplitView>(&nav, "RootSplitView")
        .template()
        .is_some());
}

#[test]
fn auto_layout_preserves_exact_threshold_edges_and_user_forced_close() {
    let _app = start();
    let nav = NavigationView::new();
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
    assert!(
        !nav.is_pane_open(),
        "explicit close survives an expanded resize"
    );
    nav.set_is_pane_open(true);
    layout(&nav, 900.0);
    assert_eq!(nav.display_mode(), NavigationViewDisplayMode::Compact);
    assert!(!nav.is_pane_open(), "expanded to compact closes");
    nav.set_is_pane_open(true);
    nav.invalidate_measure();
    layout(&nav, 900.0);
    assert!(
        nav.is_pane_open(),
        "ordinary remeasure must not undo a user opening"
    );
}

#[test]
fn explicit_modes_drive_native_split_geometry_and_top_reparenting() {
    let _app = start();
    let nav = NavigationView::new();
    nav.menu_items().add(boxed(&item("Home")));
    let scene = attach(&nav, 800.0);
    let split = part::<SplitView>(&nav, "RootSplitView");
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    assert_eq!(nav.display_mode(), NavigationViewDisplayMode::Expanded);
    assert_eq!(split.display_mode(), SplitViewDisplayMode::CompactInline);
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftMinimal);
    assert_eq!(nav.display_mode(), NavigationViewDisplayMode::Minimal);
    assert!(!nav.is_pane_open());
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    assert!(nav.is_pane_open());
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftCompact);
    assert_eq!(split.display_mode(), SplitViewDisplayMode::CompactOverlay);
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Top);
    nav.update_layout();
    assert_eq!(nav.display_mode(), NavigationViewDisplayMode::Minimal);
    assert!(!nav.is_pane_open());
    assert!(split.pane().is_none());
    assert!(part::<ContentPresenter>(&nav, "TopPanePresenter")
        .content()
        .is_some());
    assert_eq!(
        part::<StackPanel>(&nav, "MenuItemsHost").orientation(),
        Orientation::Horizontal
    );
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    assert!(split.pane().is_some());
    nav.update_layout();
    assert!(part::<ContentPresenter>(&nav, "TopPanePresenter")
        .content()
        .is_none());
    assert_eq!(
        part::<StackPanel>(&nav, "MenuItemsHost").orientation(),
        Orientation::Vertical
    );
    scene.close();
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
    nav.apply_template();
    nav.set_open_pane_length(f64::NAN);
    assert!(nav.open_pane_length().is_nan());
    assert_eq!(
        part::<SplitView>(&nav, "RootSplitView").open_pane_length(),
        0.0
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
            args.invoked_item()
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
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
        seen.borrow_mut()
            .push(format!("select:{}:{}", a.is_selected(), b.is_selected()));
    });
    invoke(&b);
    assert_eq!(
        &*events.borrow(),
        &["invoke:B:true:false", "select:false:true"]
    );
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
        weak.upgrade()
            .unwrap()
            .set_selected_item(boxed(&destination.upgrade().unwrap()));
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
    assert!(nav
        .container_from_menu_item(&Some(first.clone()))
        .unwrap()
        .ptr_eq(&first_container));
    assert!(reference_equals(
        &nav.menu_item_from_container(&first_container),
        &Some(first.clone())
    ));
    nav.set_selected_item(Some(first.clone()));
    source.remove_at(0);
    assert!(nav.selected_item().is_none());
    assert!(nav.container_from_menu_item(&Some(first)).is_none());
    nav.set_menu_items_source(None);
    assert!(nav.container_from_menu_item(&boxed(&original)).is_some());
    source.clear();
    assert!(
        nav.container_from_menu_item(&boxed(&original)).is_some(),
        "old source is disconnected"
    );
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
    assert!(nav
        .container_from_menu_item(&heading)
        .unwrap()
        .cast::<NavigationViewItemHeader>()
        .is_some());
    assert!(nav
        .container_from_menu_item(&separator)
        .unwrap()
        .cast::<NavigationViewItemSeparator>()
        .is_some());
    let container = nav
        .container_from_menu_item(&data)
        .unwrap()
        .cast::<NavigationViewItem>()
        .unwrap();
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
    assert!(nav
        .container_from_menu_item(&boxed(&child))
        .unwrap()
        .ptr_eq(&child));
    parent.menu_items().clear();
    assert!(nav.selected_item().is_none());
    assert!(!parent.is_child_selected());
    child.set_is_selected(true);
    assert!(
        nav.selected_item().is_none(),
        "removed child no longer controls selection"
    );
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
        let parent = args
            .expanding_item_container()
            .cast::<NavigationViewItem>()
            .unwrap();
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
    layout(&nav, 500.0);
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

#[test]
fn live_header_content_search_overlay_and_pane_visibility_use_actual_presenters() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let scene = attach(&nav, 800.0);
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
    let search = AutoSuggestBox::new();
    nav.set_auto_suggest_box(Some(search.clone()));
    assert!(reference_equals(
        &part::<ContentPresenter>(&nav, "PART_ContentPresenter").content(),
        &Some(Control::boxed(body))
    ));
    assert!(reference_equals(
        &part::<ContentPresenter>(&nav, "PaneHeaderPresenter").content(),
        &Some(Control::boxed(pane_header))
    ));
    assert!(reference_equals(
        &part::<ContentPresenter>(&nav, "PaneCustomContentPresenter").content(),
        &Some(Control::boxed(custom))
    ));
    assert!(reference_equals(
        &part::<ContentPresenter>(&nav, "PaneFooterPresenter").content(),
        &Some(Control::boxed(footer))
    ));
    assert!(reference_equals(
        &part::<ContentPresenter>(&nav, "ContentOverlayPresenter").content(),
        &Some(Control::boxed(overlay))
    ));
    assert!(reference_equals(
        &part::<ContentPresenter>(&nav, "AutoSuggestBoxPresenter").content(),
        &Some(Control::boxed(search))
    ));
    assert_eq!(
        part::<TextBlock>(&nav, "PaneTitleTextBlock")
            .text()
            .as_deref(),
        Some("Navigation")
    );
    nav.set_always_show_header(false);
    assert!(!part::<ContentPresenter>(&nav, "HeaderPresenter").is_visible());
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftMinimal);
    assert!(part::<ContentPresenter>(&nav, "HeaderPresenter").is_visible());
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Top);
    assert!(!part::<ContentPresenter>(&nav, "HeaderPresenter").is_visible());
    nav.set_is_pane_visible(false);
    assert!(!part::<ContentPresenter>(&nav, "TopPanePresenter").is_visible());
    scene.close();
}

#[test]
fn replacing_template_disconnects_old_buttons_and_preserves_selected_containers() {
    let _app = start();
    let nav = NavigationView::new();
    let a = item("A");
    nav.menu_items().add(boxed(&a));
    nav.set_selected_item(boxed(&a));
    nav.apply_template();
    let old_toggle = part::<Button>(&nav, "TogglePaneButton");
    let old_back = part::<Button>(&nav, "NavigationViewBackButton");
    let old_split = part::<SplitView>(&nav, "RootSplitView");
    nav.set_is_back_enabled(true);
    let backs = Rc::new(Cell::new(0));
    let seen = backs.clone();
    nav.back_requested(move |_| seen.set(seen.get() + 1));
    old_back.on_click();
    assert_eq!(backs.get(), 1);
    let replacement =
        FuncControlTemplate::for_type::<NavigationView>(|_, _| Border::new().upcast());
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
fn selection_peer_reports_native_container_and_does_not_keep_control_alive() {
    let _app = start();
    let nav = NavigationView::new();
    let a = item("A");
    nav.menu_items().add(boxed(&a));
    nav.set_selected_item(boxed(&a));
    let peer = ControlAutomationPeer::create_peer_for_element(&nav);
    let selection = peer.get_provider::<dyn ISelectionProvider>().unwrap();
    assert!(!selection.can_select_multiple() && !selection.is_selection_required());
    assert_eq!(selection.get_selection().len(), 1);
    assert!(
        selection.get_selection()[0].ptr_eq(&ControlAutomationPeer::create_peer_for_element(&a))
    );
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

#[test]
fn native_style_adaptations_preserve_local_priority_and_clear_their_own_values() {
    let _app = start();
    let nav = NavigationView::new();
    let a = item("A");
    let b = item("B");
    let local = ControlTheme::for_type::<NavigationViewItem>();
    a.set_theme(Some(local.clone()));
    nav.menu_items().add(boxed(&a));
    nav.menu_items().add(boxed(&b));
    nav.apply_template();
    let menu_theme = ControlTheme::for_type::<NavigationViewItemBase>();
    nav.set_menu_item_container_style(Some(menu_theme.clone()));
    assert_eq!(a.theme(), Some(local.clone()));
    assert_eq!(b.theme(), Some(menu_theme));
    nav.set_menu_item_container_style(None);
    assert_eq!(a.theme(), Some(local));
    assert!(b.theme().is_none());
    let toggle = part::<Button>(&nav, "TogglePaneButton");
    let toggle_theme = ControlTheme::for_type::<Button>();
    nav.set_pane_toggle_button_style(Some(toggle_theme.clone()));
    assert_eq!(toggle.theme(), Some(toggle_theme));
    nav.set_pane_toggle_button_style(None);
    assert!(toggle.theme().is_none());
}

#[test]
fn footer_source_switching_and_typed_header_template_update_native_parts() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let source = Rc::new(FerroList::<Option<BoxedValue>>::new());
    let footer = item("Footer source");
    source.add(boxed(&footer));
    nav.set_footer_menu_items_source(Some(source.clone().into()));
    assert!(nav
        .container_from_menu_item(&boxed(&footer))
        .unwrap()
        .ptr_eq(&footer));
    nav.footer_menu_items().add(boxed(&item("Manual footer")));
    assert_eq!(nav.footer_menu_items().count(), 1);
    nav.set_selected_item(boxed(&footer));
    source.clear();
    assert!(nav.selected_item().is_none());
    nav.set_footer_menu_items_source(None);
    assert!(nav
        .container_from_menu_item(&nav.footer_menu_items().get_at(0))
        .is_some());
    nav.set_header(Some(Rc::new("Name".to_owned())));
    nav.set_header_template(Some(FuncDataTemplate::for_type::<String>(
        |value, _| {
            let text = TextBlock::new();
            text.set_text(Some(&format!("Header: {value}")));
            Some(text.upcast())
        },
        false,
    )));
    let window = Window::new();
    window.set_width(800.0);
    window.set_height(500.0);
    window.set_content(Some(Control::boxed(nav.clone())));
    window.show();
    window.layout_manager().execute_layout_pass();
    assert_eq!(
        part::<ContentPresenter>(&nav, "HeaderPresenter")
            .child()
            .unwrap()
            .cast::<TextBlock>()
            .unwrap()
            .text()
            .as_deref(),
        Some("Header: Name")
    );
    window.close();
}

#[test]
fn focus_following_raises_invocation_once_and_keyboard_navigation_moves_native_focus() {
    let _app = start_with_focus();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    let a = item("A");
    let b = item("B");
    nav.menu_items().add(boxed(&a));
    nav.menu_items().add(boxed(&b));
    let window = Window::new();
    window.set_width(800.0);
    window.set_height(500.0);
    window.set_content(Some(Control::boxed(nav.clone())));
    window.show();
    window.layout_manager().execute_layout_pass();
    // Window activation can focus the first item. Move away before enabling
    // selection-following so the next focus change is deterministic.
    b.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
    nav.set_selection_follows_focus(NavigationViewSelectionFollowsFocus::Enabled);
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
    window.close();
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
        [
            NavigationViewSelectionFollowsFocus::Disabled as i32,
            NavigationViewSelectionFollowsFocus::Enabled as i32
        ],
        [0, 1]
    );
    let args = ferroui_controls::NavigationViewItemInvokedEventArgs::new();
    assert!(
        args.invoked_item().is_none()
            && args.invoked_item_container().is_none()
            && !args.is_settings_invoked()
    );
    let nav = NavigationView::new();
    nav.apply_template();
    let modes = Rc::new(RefCell::new(Vec::new()));
    let seen = modes.clone();
    nav.display_mode_changed(move |args| seen.borrow_mut().push(args.display_mode()));
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftMinimal);
    assert_eq!(
        &*modes.borrow(),
        &[
            NavigationViewDisplayMode::Expanded,
            NavigationViewDisplayMode::Minimal
        ]
    );
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
        assert!(args
            .invoked_item_container()
            .unwrap()
            .ptr_eq(&weak_settings.upgrade().unwrap()));
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
    layout(&nav, 1100.0);
    part::<Button>(&nav, "TogglePaneButton").on_click();
    assert!(!nav.is_pane_open());
    layout(&nav, 1200.0);
    assert!(!nav.is_pane_open());
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Auto);
    assert!(
        nav.is_pane_open(),
        "a mode assignment resets the force-close flag"
    );
    nav.set_is_pane_visible(false);
    assert!(!nav.is_pane_open());
    nav.set_is_pane_visible(true);
    assert!(nav.is_pane_open());
    nav.set_is_pane_toggle_button_visible(false);
    assert!(!part::<Button>(&nav, "TogglePaneButton").is_visible());
    nav.set_is_back_button_visible(NavigationViewBackButtonVisible::Collapsed);
    assert!(!part::<Button>(&nav, "NavigationViewBackButton").is_visible());
    nav.set_is_settings_visible(false);
    assert!(!nav.settings_item().is_visible());
    nav.set_is_settings_visible(true);
    assert!(nav.settings_item().is_visible());
    nav.set_selected_item(nav.settings_item().content());
    assert!(nav.settings_item().is_selected());
    assert!(nav
        .container_from_menu_item(&nav.selected_item())
        .unwrap()
        .ptr_eq(&nav.settings_item()));
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
    layout(&nav, 800.0);
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
    assert!(
        !foreign.is_expanded(),
        "an active empty source takes precedence over HasUnrealizedChildren"
    );
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

#[test]
fn closed_left_and_minimal_panes_propagate_native_item_context_and_expansion_memory() {
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
    nav.menu_items()
        .add(Some(Control::boxed(separator.clone())));
    let scene = attach(&nav, 900.0);
    header.apply_template();
    parent.apply_template();
    separator.apply_template();
    let header_content = part::<ContentPresenter>(&header, "PART_ContentPresenter");
    let item_content = part::<ContentPresenter>(&parent, "PART_ContentPresenter");
    let line = part::<Border>(&separator, "NavigationViewItemSeparatorRootGrid");
    assert!(header_content.is_visible() && item_content.is_visible());
    nav.expand(&parent);
    nav.set_is_pane_open(false);
    assert_eq!(nav.display_mode(), NavigationViewDisplayMode::Expanded);
    assert!(!header_content.is_visible() && !item_content.is_visible());
    assert!(!parent.is_expanded());
    nav.set_is_pane_open(true);
    assert!(parent.is_expanded());
    assert!(header_content.is_visible() && item_content.is_visible());
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::LeftMinimal);
    assert!(!parent.is_expanded());
    nav.set_is_pane_open(true);
    assert!(parent.is_expanded(), "Minimal mode remembers expansion too");
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Top);
    assert_eq!(line.width(), 1.0);
    assert_eq!(line.height(), 20.0);
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    assert!(line.width().is_nan());
    assert_eq!(line.height(), 1.0);
    scene.close();
}

#[test]
fn focus_following_reaches_popup_hierarchy_without_invoking_ancestors() {
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
    let window = Window::new();
    window.set_width(800.0);
    window.set_height(500.0);
    window.set_content(Some(Control::boxed(nav.clone())));
    window.show();
    window.layout_manager().execute_layout_pass();
    nav.expand(&parent);
    window.layout_manager().execute_layout_pass();
    child.apply_template();
    assert!(part::<Popup>(&parent, "PART_ChildrenPopup").is_open());
    assert!(action.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    let weak_anchor = anchor.downgrade();
    let weak_child = child.downgrade();
    nav.item_invoked(move |args| {
        let anchor = weak_anchor.upgrade().unwrap();
        let child = weak_child.upgrade().unwrap();
        assert!(args.invoked_item_container().unwrap().ptr_eq(&child));
        seen.borrow_mut().push(format!(
            "invoke:{}:{}",
            anchor.is_selected(),
            child.is_selected()
        ));
    });
    let seen = events.clone();
    let weak_anchor = anchor.downgrade();
    let weak_child = child.downgrade();
    nav.selection_changed(move |args| {
        let anchor = weak_anchor.upgrade().unwrap();
        let child = weak_child.upgrade().unwrap();
        assert!(args.selected_item_container().unwrap().ptr_eq(&child));
        seen.borrow_mut().push(format!(
            "select:{}:{}",
            anchor.is_selected(),
            child.is_selected()
        ));
    });
    nav.set_selection_follows_focus(NavigationViewSelectionFollowsFocus::Enabled);
    assert!(child.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
    assert_eq!(
        &*events.borrow(),
        &["invoke:true:false", "select:false:true"]
    );
    assert!(parent.is_expanded() && child.is_expanded());
    assert!(parent.is_child_selected() && !parent.is_selected());
    child.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
    assert_eq!(events.borrow().len(), 2);
    assert!(action.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
    assert!(!disabled.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
    assert!(child.is_selected() && !action.is_selected());
    assert_eq!(events.borrow().len(), 2);
    // A click invokes first, then focuses. Following that focus must not invoke
    // the already selected child a second time.
    part::<Button>(&child, "NavigationViewItemPresenter").on_click();
    assert_eq!(
        &*events.borrow(),
        &[
            "invoke:true:false",
            "select:false:true",
            "invoke:false:true"
        ]
    );
    nav.set_selection_follows_focus(NavigationViewSelectionFollowsFocus::Disabled);
    anchor.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
    assert!(child.is_selected());
    assert_eq!(events.borrow().len(), 3);
    window.close();
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
        weak_owner
            .upgrade()
            .unwrap()
            .set_selected_item(boxed(&weak_anchor.upgrade().unwrap()));
    });
    nav.apply_template();
    // Rebuilding the control template replaces subscriptions. The redirect above
    // leaves Child unselected, exposing duplicate handlers rather than hiding
    // them behind the already-selected guard.
    for _ in 0..3 {
        nav.set_template(Some(FuncControlTemplate::for_type::<NavigationView>(
            |_, _| Border::new().upcast(),
        )));
        nav.apply_template();
    }
    // A retained realized child can receive focus before it has a visual route
    // through the owner. Its direct subscription must still find NavigationView.
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
    // The source and every container intentionally outlive their former owner.
    raise_item_focus(&child);
    assert!(!press(&child, Key::Home).handled());
    source.add(boxed(&item("Later")));
    assert_eq!(invocations.get(), 2);
    assert!(weak_owner.upgrade().is_none());
}

#[test]
fn popup_directional_keys_follow_vertical_tree_and_top_roots_follow_flow_direction() {
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
    let window = Window::new();
    window.set_width(900.0);
    window.set_height(500.0);
    window.set_content(Some(Control::boxed(nav.clone())));
    window.show();
    window.layout_manager().execute_layout_pass();
    nav.expand(&parent);
    window.layout_manager().execute_layout_pass();
    assert!(part::<Popup>(&parent, "PART_ChildrenPopup").is_open());
    assert!(first.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
    assert!(press(&first, Key::Down).handled());
    assert!(
        last.is_focused(),
        "hidden, disabled and header rows are skipped"
    );
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
    assert!(
        first.is_focused(),
        "Down enters the expanded Top root's popup"
    );
    assert!(press(&first, Key::Right).handled());
    assert!(
        !first.is_expanded(),
        "nested RTL collapse remains item-owned"
    );
    assert!(parent.is_expanded());
    assert_eq!(invocations.get(), 0);
    assert!(nav.selected_item().is_none());
    window.close();
}

#[test]
fn clearing_an_attached_template_disconnects_retained_parts_and_restores_native_content() {
    let _app = start();
    let nav = NavigationView::new();
    nav.set_pane_display_mode(NavigationViewPaneDisplayMode::Left);
    nav.set_is_back_enabled(true);
    let selected = item("Selected");
    nav.menu_items().add(boxed(&selected));
    nav.set_selected_item(boxed(&selected));
    let content = Border::new();
    nav.set_content(Some(Control::boxed(content.clone())));
    let window = Window::new();
    window.set_width(800.0);
    window.set_height(500.0);
    window.set_content(Some(Control::boxed(nav.clone())));
    window.show();
    window.layout_manager().execute_layout_pass();
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
    window.layout_manager().execute_layout_pass();
    assert!(selected.is_selected());
    assert!(part::<ContentPresenter>(&nav, "PART_ContentPresenter")
        .child()
        .unwrap()
        .ptr_eq(&content));
    part::<Button>(&nav, "NavigationViewBackButton").on_click();
    assert_eq!(back_count.get(), 1);
    old_back.on_click();
    assert_eq!(back_count.get(), 1);
    window.close();
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
    assert!(reference_equals(
        &changes.borrow()[0].selected_item(),
        &second
    ));
    assert!(changes.borrow()[0].selected_item_container().is_none());

    // Materializing before the dispatcher callback produces the native
    // container immediately and consumes the pending notification exactly once.
    let realized: Option<BoxedValue> = Some(Rc::new("Realized before idle".to_owned()));
    nav.set_selected_item(realized.clone());
    nav.menu_items().add(realized.clone());
    assert_eq!(changes.borrow().len(), 2);
    assert!(reference_equals(
        &changes.borrow()[1].selected_item(),
        &realized
    ));
    assert!(changes.borrow()[1]
        .selected_item_container()
        .unwrap()
        .is_selected());
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(changes.borrow().len(), 2);

    // A newer clear must also prevent the older callback from raising an event.
    nav.set_selected_item(Some(Rc::new("Superseded by clear".to_owned())));
    nav.set_selected_item(None);
    assert_eq!(changes.borrow().len(), 3);
    assert!(changes.borrow()[2].selected_item().is_none());
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(changes.borrow().len(), 3);

    // Suppressed actions restore the original unresolved selection, including
    // its outstanding notification rather than silently discarding it.
    let action = item("Action");
    action.set_selects_on_invoked(false);
    nav.menu_items().add(boxed(&action));
    nav.set_selected_item(second.clone());
    nav.set_selected_item(boxed(&action));
    assert!(reference_equals(&nav.selected_item(), &second));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(changes.borrow().len(), 4);
    assert!(reference_equals(
        &changes.borrow()[3].selected_item(),
        &second
    ));
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
    let replacement = FuncDataTemplate::for_type::<String>(
        |data, _| Some(item(&format!("New {data}")).upcast()),
        false,
    );
    let weak = nav.downgrade();
    let changed = Rc::new(Cell::new(false));
    let change_once = changed.clone();
    let old = FuncDataTemplate::for_type::<String>(
        move |data, _| {
            if !change_once.replace(true) {
                weak.upgrade()
                    .unwrap()
                    .set_menu_item_template(Some(replacement.clone()));
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
    assert_eq!(
        container
            .content()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "New Model"
    );
    assert!(reference_equals(&container.data_context(), &model));
    nav.set_selected_item(model.clone());
    assert!(container.is_selected());
    assert!(reference_equals(
        &nav.menu_item_from_container(&container),
        &model
    ));
}

#[test]
fn retemplating_custom_presenters_clears_null_data_factories_before_content() {
    let _app = start();
    let nav = NavigationView::new();
    let template = || {
        FuncControlTemplate::for_type::<NavigationView>(|_, scope| {
            let root = StackPanel::new();
            for name in ["HeaderPresenter", "PART_ContentPresenter"] {
                let presenter = ContentPresenter::new();
                presenter.set_name(Some(name.into()));
                root.children()
                    .add(presenter.register_in_name_scope(&**scope));
            }
            root.upcast()
        })
    };
    let header = Border::new();
    let content = Border::new();
    let null_builds = Rc::new(Cell::new(0));
    let factory = |control: &Control| {
        let weak = control.to_ref().downgrade();
        let null_builds = null_builds.clone();
        FuncDataTemplate::new(
            |_| true,
            move |data, _| {
                if data.is_none() {
                    null_builds.set(null_builds.get() + 1);
                }
                weak.upgrade()
            },
            false,
        )
    };
    nav.set_template(Some(template()));
    nav.set_header(Some(Rc::new("Header model".to_owned())));
    nav.set_header_template(Some(factory(&header)));
    nav.set_content(Some(Rc::new("Content model".to_owned())));
    nav.set_content_template(Some(factory(&content)));
    let window = attach(&nav, 800.0);
    let old_header = part::<ContentPresenter>(&nav, "HeaderPresenter");
    let old_content = part::<ContentPresenter>(&nav, "PART_ContentPresenter");
    assert!(old_header.child().unwrap().ptr_eq(&header));
    assert!(old_content.child().unwrap().ptr_eq(&content));

    nav.set_template(Some(template()));
    nav.apply_template();
    window.update_layout();
    for presenter in [&old_header, &old_content] {
        assert!(presenter.content_template().is_none());
        assert!(presenter.content().is_none());
        assert!(presenter.child().is_none());
    }
    assert!(part::<ContentPresenter>(&nav, "HeaderPresenter")
        .child()
        .unwrap()
        .ptr_eq(&header));
    assert!(part::<ContentPresenter>(&nav, "PART_ContentPresenter")
        .child()
        .unwrap()
        .ptr_eq(&content));
    assert_eq!(null_builds.get(), 0);

    nav.set_template(None);
    nav.apply_template();
    assert!(header.visual_parent().is_none());
    assert!(content.visual_parent().is_none());
    assert_eq!(null_builds.get(), 0);
    window.close();
}

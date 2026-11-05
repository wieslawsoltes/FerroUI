//! Tests of `BreadcrumbBar` (WinUI; docs/porting/winui-controls.md, G2 of #418): the API tests of
//! WinUI (`controls/dev/Breadcrumb/APITests/BreadcrumbTests.cs`) and the behaviour of
//! `BreadcrumbBar.cpp`, `BreadcrumbBarItem.cpp` and `BreadcrumbLayout.cpp` (the crumbling into the
//! ellipsis, its flyout, the clicks, the keys).

use crate::automation::peers::{AutomationControlType, ControlAutomationPeer};
use crate::automation::provider::IInvokeProvider;
use crate::automation::{AccessibilityView, AutomationProperties};
use crate::input_test_support::part;
use crate::primitives::BreadcrumbLayoutPanel;
use crate::templates::{FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::presenters::ContentPresenter;
use crate::{BreadcrumbBar, BreadcrumbBarItem, Button, Control, ItemsSource, StackPanel, TextBlock, Window};
use ferroui_base::collections::FerroList;
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::layout::ILayoutManager;
use ferroui_base::{BoxedValue, Ref, StyledElement};
use std::cell::RefCell;
use std::rc::Rc;

fn start() -> UnitTestApplicationScope {
    crate::register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

fn named<T: ferroui_base::ObjectType + ferroui_base::Upcast<StyledElement>>(element: Ref<T>, name: &str) -> Ref<T> {
    let styled: &StyledElement = (*element).upcast();
    styled.set_name(Some(name.to_string()));
    element
}

/// The part of `Controls/BreadcrumbBar.xaml`: the panel `PART_ItemsRepeater`.
fn bar_template() -> Rc<FuncControlTemplate> {
    FuncControlTemplate::new(|_, scope| named(BreadcrumbLayoutPanel::new(), "PART_ItemsRepeater").register_in_name_scope(&**scope).upcast::<Control>())
}

/// The part of the item template: the button `PART_ItemButton` showing the content, 50 wide.
fn item_template() -> Rc<FuncControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let button = named(Button::new(), "PART_ItemButton").register_in_name_scope(&**scope);
        let presenter = ContentPresenter::new();
        let content = crate::ContentControl::content_property().as_property();
        presenter.bind_binding(ContentPresenter::content_property().as_property(), &*TemplateBinding::new(content));
        button.set_content(Some(Control::boxed(presenter)));
        button.set_width(50.0);
        button.set_height(20.0);
        button.upcast::<Control>()
    })
}

fn bar(items: &[&str]) -> Ref<BreadcrumbBar> {
    let bar = BreadcrumbBar::new();
    bar.set_template(Some(bar_template()));
    bar.set_items_source(Some(ItemsSource::from_strs(items.iter().copied())));
    bar
}

fn show(bar: &BreadcrumbBar, width: f64) -> Ref<Window> {
    let window = Window::new();
    let panel = StackPanel::new();
    panel.set_width(width);
    panel.children().add(&bar.to_ref());
    window.set_content(Some(Control::boxed(panel)));
    window.show();
    for item in bar.realized_items() {
        item.set_template(Some(item_template()));
    }
    window.layout_manager().execute_layout_pass();
    window
}

fn items(bar: &BreadcrumbBar) -> Vec<Ref<BreadcrumbBarItem>> {
    bar.realized_items()
}

fn text(item: &BreadcrumbBarItem) -> String {
    item.content().and_then(|content| content.downcast_ref::<String>().cloned()).unwrap_or_default()
}

fn shown(item: &BreadcrumbBarItem) -> bool {
    item.bounds().width > 0.0
}

#[test]
fn the_defaults_are_empty_and_an_empty_bar_has_only_the_ellipsis() {
    // VerifyBreadcrumbDefaultAPIValues and VerifyDefaultBreadcrumb
    let _app = start();
    let bar = BreadcrumbBar::new();
    assert!(bar.items_source().is_none());
    assert!(bar.item_template().is_none());
    bar.set_template(Some(bar_template()));
    let _window = show(&bar, 400.0);
    let items = items(&bar);
    assert_eq!(1, items.len());
    assert!(items[0].is_ellipsis_item());
}

#[test]
fn the_items_are_the_ellipsis_then_the_levels_with_the_last_current() {
    let _app = start();
    let bar = bar(&["Home", "Documents", "Design"]);
    let _window = show(&bar, 400.0);
    let items = items(&bar);
    assert_eq!(4, items.len());
    assert!(items[0].is_ellipsis_item());
    assert_eq!(Some("More".to_string()), AutomationProperties::get_name(&items[0]));
    assert_eq!(vec!["Home", "Documents", "Design"], items[1..].iter().map(|item| text(item)).collect::<Vec<_>>());
    assert!(items[3].is_last_item() && !items[2].is_last_item());
    assert!(items[3].pseudo_classes().contains(":last"));
    assert!(items[0].pseudo_classes().contains(":ellipsis"));
    // Everything fits: the ellipsis is hidden, and raw for automation.
    assert!(!shown(&items[0]));
    assert!(items[1..].iter().all(|item| shown(item)));
    assert_eq!(AccessibilityView::Raw, AutomationProperties::get_accessibility_view(&items[0]));
    assert_eq!(1, AutomationProperties::get_position_in_set(&items[1]));
    assert_eq!(3, AutomationProperties::get_size_of_set(&items[3]));
}

#[test]
fn items_that_do_not_fit_crumble_into_the_ellipsis() {
    let _app = start();
    // Each item is 50 wide: 160 holds the ellipsis and two items.
    let bar = bar(&["A", "B", "C", "D", "E"]);
    let _window = show(&bar, 160.0);
    let items = items(&bar);
    let panel = part::<BreadcrumbLayoutPanel>(&bar, "PART_ItemsRepeater");
    assert!(panel.ellipsis_is_rendered());
    assert!(shown(&items[0]));
    assert_eq!(4, panel.first_rendered_item_index_after_ellipsis());
    assert_eq!(vec![false, false, false, true, true], items[1..].iter().map(|item| shown(item)).collect::<Vec<_>>());
    assert_eq!(2, panel.visible_items_count());
    assert_eq!(AccessibilityView::Content, AutomationProperties::get_accessibility_view(&items[0]));
    assert_eq!(1, AutomationProperties::get_position_in_set(&items[4]));
    assert_eq!(2, AutomationProperties::get_size_of_set(&items[5]));
    // The ellipsis comes first, then the shown items.
    assert_eq!(0.0, items[0].bounds().x);
    assert_eq!(50.0, items[4].bounds().x);
    assert_eq!(vec!["A", "B", "C"], bar.hidden_elements().iter().map(|item| item.as_ref().unwrap().downcast_ref::<String>().unwrap().clone()).collect::<Vec<_>>());
}

#[test]
fn clicking_an_item_raises_item_clicked_with_its_index() {
    let _app = start();
    let bar = bar(&["Home", "Documents", "Design"]);
    let _window = show(&bar, 400.0);
    let clicked = Rc::new(RefCell::new(Vec::new()));
    let seen = clicked.clone();
    let _subscription = bar.item_clicked(move |_, args| {
        let item = args.item().and_then(|item| item.downcast_ref::<String>().cloned()).unwrap_or_default();
        seen.borrow_mut().push((args.index(), item));
    });
    let items = items(&bar);
    part::<Button>(&items[2], "PART_ItemButton").raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    // Enter on an item clicks it.
    let mut args = KeyEventArgs::new();
    args.key = Key::Enter;
    args.set_routed_event(Some(InputElement::key_down_event()));
    items[1].raise_event(&args);
    assert_eq!(vec![(1, "Documents".to_string()), (0, "Home".to_string())], *clicked.borrow());
}

#[test]
fn the_ellipsis_flyout_lists_the_hidden_items_and_clicking_one_raises_its_index() {
    let _app = start();
    let bar = bar(&["A", "B", "C", "D", "E"]);
    let _window = show(&bar, 160.0);
    let clicked = Rc::new(RefCell::new(Vec::new()));
    let seen = clicked.clone();
    let _subscription = bar.item_clicked(move |_, args| seen.borrow_mut().push(args.index()));
    let ellipsis = items(&bar)[0].clone();
    part::<Button>(&ellipsis, "PART_ItemButton").raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    let flyout = ellipsis.ellipsis_flyout().expect("the flyout of the ellipsis");
    assert!(flyout.is_open());
    let drop_down = ellipsis.ellipsis_drop_down_items();
    // The hidden items in reverse order.
    assert_eq!(vec!["C", "B", "A"], drop_down.iter().map(|item| text(item)).collect::<Vec<_>>());
    assert!(drop_down.iter().all(|item| item.is_ellipsis_drop_down_item()));
    assert!(drop_down[0].pseudo_classes().contains(":ellipsisdropdown"));
    assert_eq!(1, AutomationProperties::get_position_in_set(&drop_down[0]));
    assert_eq!(3, AutomationProperties::get_size_of_set(&drop_down[0]));
    drop_down[1].on_click_event();
    assert!(!flyout.is_open());
    assert_eq!(vec![1], *clicked.borrow());
}

#[test]
fn a_change_of_the_items_source_is_respected() {
    // VerifyCollectionChangeGetsRespected
    let _app = start();
    let list: FerroList<Option<BoxedValue>> = FerroList::new();
    list.add(Some(Rc::new("Node 1".to_string()) as BoxedValue));
    list.add(Some(Rc::new("Node 2".to_string()) as BoxedValue));
    let bar = BreadcrumbBar::new();
    bar.set_template(Some(bar_template()));
    bar.set_items_source(Some(ItemsSource::from(Rc::new(list.clone()))));
    let _window = show(&bar, 400.0);
    assert_eq!(3, items(&bar).len());
    list.add(Some(Rc::new("Node 3".to_string()) as BoxedValue));
    let items = items(&bar);
    assert_eq!(4, items.len());
    assert!(items[3].is_last_item() && !items[2].is_last_item());
    bar.set_items_source(Some(ItemsSource::from_strs(["x"])));
    assert_eq!(2, self::items(&bar).len());
}

#[test]
fn a_template_that_is_a_breadcrumb_item_is_not_wrapped() {
    // VerifyCustomItemTemplate
    let _app = start();
    let wrapping = bar(&["Node 1", "Node 2"]);
    wrapping.set_item_template(Some(FuncDataTemplate::new(|_| true, |_, _| Some(TextBlock::new().upcast::<Control>()), false)));
    let unwrapped = bar(&["Node 1", "Node 2"]);
    unwrapped.set_item_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let item = BreadcrumbBarItem::new();
            item.set_tag(Some(Rc::new("from the template".to_string()) as BoxedValue));
            Some(item.upcast::<Control>())
        },
        false,
    )));
    let _window = show(&wrapping, 400.0);
    let _window2 = show(&unwrapped, 400.0);
    let wrapped_item = items(&wrapping)[1].clone();
    assert!(wrapped_item.content_template().is_some());
    assert!(wrapped_item.tag().is_none());
    let item = items(&unwrapped)[1].clone();
    assert_eq!(Some("from the template".to_string()), item.tag().and_then(|tag| tag.downcast_ref::<String>().cloned()));
    assert_eq!("Node 1", text(&item));
}

#[test]
fn the_arrow_keys_move_the_focus_through_the_shown_items() {
    let _app = crate::input_test_support::start_with_focus();
    crate::register_types();
    let bar = bar(&["A", "B", "C", "D", "E"]);
    let _window = show(&bar, 160.0);
    let items = items(&bar);
    for item in &items {
        item.set_focusable(true);
    }
    items[4].focus();
    let press = |key: Key| {
        let mut args = KeyEventArgs::new();
        args.key = key;
        args.set_routed_event(Some(InputElement::key_down_event()));
        items[4].raise_event(&args);
        args.handled()
    };
    // From the first item after the ellipsis, Left goes to the ellipsis.
    assert!(press(Key::Left));
    assert!(items[0].is_focused());
    let mut args = KeyEventArgs::new();
    args.key = Key::Right;
    args.set_routed_event(Some(InputElement::key_down_event()));
    items[0].raise_event(&args);
    // From the ellipsis, Right goes to the first shown item.
    assert!(args.handled());
    assert!(items[4].is_focused());
}

#[test]
fn the_item_peer_is_a_button_that_invokes_the_item() {
    let _app = start();
    let bar = bar(&["Home", "Design"]);
    let _window = show(&bar, 400.0);
    let clicked = Rc::new(RefCell::new(Vec::new()));
    let seen = clicked.clone();
    let _subscription = bar.item_clicked(move |_, args| seen.borrow_mut().push(args.index()));
    let item = items(&bar)[1].clone();
    let peer = ControlAutomationPeer::create_peer_for_element(&item);
    assert_eq!(AutomationControlType::Button, peer.get_automation_control_type());
    assert_eq!("BreadcrumbBarItem", peer.get_class_name());
    assert_eq!("breadcrumb bar item", peer.get_localized_control_type());
    peer.get_provider::<dyn IInvokeProvider>().unwrap().invoke().unwrap();
    assert_eq!(vec![0], *clicked.borrow());
}

//! Tests of `BreadcrumbBar` and `BreadcrumbBarItem` (WinUI; docs/porting/winui-controls.md,
//! section 10.11): the API tests of WinUI (`controls/dev/Breadcrumb/APITests/BreadcrumbTests.cs`)
//! and the behaviour of `BreadcrumbBar.cpp`, `BreadcrumbBarItem.cpp` and `BreadcrumbLayout.cpp` its
//! interaction tests drive (`InteractionTests/BreadcrumbBarTests.cs`: the ellipsis and its flyout,
//! the clicks and the keyboard).

use crate::automation::peers::{AutomationControlType, ControlAutomationPeer};
use crate::automation::provider::IInvokeProvider;
use crate::automation::{AccessibilityView, AutomationProperties};
use crate::input_test_support::start_with_focus;
use crate::primitives::{BreadcrumbBarPanel, Popup, TemplatedControl};
use crate::templates::{FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::{
    BreadcrumbBar, BreadcrumbBarItem, BreadcrumbBarItemClickedEventArgs, Button, Control, Grid, ItemsSource, StackPanel,
    TextBlock, Window,
};
use ferroui_base::collections::FerroList;
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers, NavigationMethod};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::FlowDirection;
use ferroui_base::styling::{ControlTheme, Setter};
use ferroui_base::{BoxedValue, Ref, StyledElement};
use std::cell::RefCell;
use std::rc::Rc;

/// The width of every item of the template of these tests.
const ITEM_WIDTH: f64 = 100.0;

fn named<T: ferroui_base::ObjectType + ferroui_base::Upcast<StyledElement>>(element: Ref<T>, name: &str) -> Ref<T> {
    let styled: &StyledElement = (*element).upcast();
    styled.set_name(Some(name.to_string()));
    element
}

/// The parts of the template of `BreadcrumbBarItem` in `Controls/BreadcrumbBar.xaml`: the button of
/// the crumb and the flyout of the ellipsis with its items panel, in a root 100 wide.
fn item_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<BreadcrumbBarItem>(|_, scope| {
        let root = named(Grid::new(), "PART_LayoutRoot").register_in_name_scope(&**scope);
        root.set_width(ITEM_WIDTH);
        root.set_height(20.0);
        let button = named(Button::new(), "PART_ItemButton").register_in_name_scope(&**scope);
        root.children().add(&button);
        let popup = named(Popup::new(), "PART_EllipsisFlyout").register_in_name_scope(&**scope);
        let panel = named(StackPanel::new(), "PART_EllipsisItemsRepeater").register_in_name_scope(&**scope);
        popup.set_child(panel.upcast::<Control>());
        root.children().add(&popup);
        root.upcast::<Control>()
    })
}

/// The template of `BreadcrumbBar`: the panel `PART_ItemsRepeater`.
fn bar_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<BreadcrumbBar>(|_, scope| {
        named(BreadcrumbBarPanel::new(), "PART_ItemsRepeater").register_in_name_scope(&**scope).upcast::<Control>()
    })
}

fn theme_resource(theme: Ref<ControlTheme>) -> Option<BoxedValue> {
    Some(Rc::new(theme))
}

fn strings(items: &[&str]) -> ItemsSource {
    ItemsSource::from_strs(items.iter().copied())
}

/// A bar of `width` with `items`, shown in a window whose resources hold the item theme.
fn show(items: Option<ItemsSource>, width: f64) -> (Ref<Window>, Ref<BreadcrumbBar>) {
    crate::register_types();
    let bar = BreadcrumbBar::new();
    bar.set_template(Some(bar_template()));
    bar.set_width(width);
    bar.set_items_source(items);
    let window = Window::new();
    window.resources().add(
        BreadcrumbBarItem::TYPE,
        theme_resource(ControlTheme::with_setters(
            BreadcrumbBarItem::TYPE,
            [
                Setter::new(TemplatedControl::template_property(), Some(item_template())),
                // IsTabStop of the style of the item.
                Setter::new(InputElement::focusable_property(), true),
            ],
        )),
    );
    window.set_content(Some(Control::boxed(bar.to_ref())));
    window.show();
    window.layout_manager().execute_layout_pass();
    (window, bar)
}

fn layout(window: &Window) {
    window.layout_manager().execute_layout_pass();
}

fn panel(bar: &BreadcrumbBar) -> Ref<BreadcrumbBarPanel> {
    bar.items_repeater().expect("the panel of the template")
}

fn element(bar: &BreadcrumbBar, index: i32) -> Ref<BreadcrumbBarItem> {
    bar.try_get_element(index).expect("an element of the panel")
}

fn text(value: Option<BoxedValue>) -> String {
    value.and_then(|value| value.downcast_ref::<String>().cloned()).unwrap_or_default()
}

/// The contents of the elements of the panel after the ellipsis item.
fn contents(bar: &BreadcrumbBar) -> Vec<String> {
    let count = panel(bar).children().count() as i32;
    (1..count).map(|index| text(element(bar, index).content())).collect()
}

fn track_clicks(bar: &BreadcrumbBar) -> (Rc<RefCell<Vec<(i32, String)>>>, ferroui_base::reactive::Subscription) {
    let clicks = Rc::new(RefCell::new(Vec::new()));
    let sink = clicks.clone();
    let subscription = bar.item_clicked(move |_, args: &BreadcrumbBarItemClickedEventArgs| {
        sink.borrow_mut().push((args.index(), text(args.item())));
    });
    (clicks, subscription)
}

fn press(control: &InputElement, key: Key) -> KeyEventArgs {
    let mut args = KeyEventArgs::new();
    args.key = key;
    args.key_modifiers = KeyModifiers::NONE;
    args.set_routed_event(Some(InputElement::key_down_event()));
    control.raise_event(&args);
    args
}

/// VerifyBreadcrumbDefaultAPIValues
#[test]
fn the_default_values_are_those_of_winui() {
    crate::register_types();
    let bar = BreadcrumbBar::new();
    assert!(bar.items_source().is_none());
    assert!(bar.item_template().is_none());
}

/// VerifyDefaultBreadcrumb: the panel holds the ellipsis item and an item for each item of the
/// source; the last one is the current item.
#[test]
fn the_items_become_breadcrumb_bar_items_after_the_ellipsis_item() {
    let _app = start_with_focus();
    let (_window, bar) = show(Some(strings(&["Root", "Node A", "Node B"])), 1000.0);
    assert_eq!(4, panel(&bar).children().count());
    assert!(element(&bar, 0).is_ellipsis_item());
    assert!(element(&bar, 0).content().is_none());
    assert_eq!(vec!["Root", "Node A", "Node B"], contents(&bar));
    assert!(element(&bar, 3).is_last_item());
    assert!(!element(&bar, 1).is_last_item());
    assert!(element(&bar, 3).classes().contains(":lastitem"));
    assert!(element(&bar, 0).classes().contains(":ellipsis"));
    assert_eq!(Some("More".to_string()), AutomationProperties::get_name(&element(&bar, 0)));
}

/// VerifyObjectItemsSource: an item that is a breadcrumb bar item is its own element.
#[test]
fn an_item_that_is_a_breadcrumb_bar_item_is_its_own_element() {
    let _app = start_with_focus();
    let own = BreadcrumbBarItem::new();
    own.set_content(Some(Rc::new("Own".to_string()) as BoxedValue));
    let items = ItemsSource::from_items([Some(Control::boxed(own.clone())), Some(Rc::new("Text".to_string()) as BoxedValue)]);
    let (_window, bar) = show(Some(items), 1000.0);
    assert!(own.ptr_eq(&element(&bar, 1)));
    assert_eq!("Text", text(element(&bar, 2).content()));
}

/// VerifyCustomItemTemplate and VerifyDropdownItemTemplateWithNoControl: a template whose root is a
/// breadcrumb bar item gives the element; another template is the content template of a new item.
#[test]
fn the_item_template_gives_the_elements() {
    let _app = start_with_focus();
    let (window, bar) = show(Some(strings(&["Root", "Node"])), 1000.0);
    bar.set_item_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let item = BreadcrumbBarItem::new();
            item.set_tag(Some(Rc::new("from the template".to_string()) as BoxedValue));
            Some(item.upcast::<Control>())
        },
        false,
    )));
    layout(&window);
    assert_eq!("from the template", text(element(&bar, 1).tag()));
    assert_eq!("Node", text(element(&bar, 2).content()));
    assert_eq!("Node", text(element(&bar, 2).data_context()));

    let template: Rc<dyn crate::templates::IDataTemplate> =
        FuncDataTemplate::new(|_| true, |_, _| Some(TextBlock::new().upcast::<Control>()), false);
    bar.set_item_template(Some(template.clone()));
    layout(&window);
    let item = element(&bar, 1);
    assert_eq!("Root", text(item.content()));
    assert!(item.content_template().is_some_and(|t| Rc::ptr_eq(&t, &template)));
}

/// VerifyCollectionChangeGetsRespected: a change of the collection changes the elements, and the
/// last element is the current item.
#[test]
fn a_change_of_the_collection_changes_the_elements() {
    let _app = start_with_focus();
    let list = Rc::new(FerroList::<String>::new());
    list.add_range(["Node 1".to_string(), "Node 2".to_string()]);
    let (window, bar) = show(Some(list.clone().into()), 1000.0);
    let first = element(&bar, 1);
    assert!(element(&bar, 2).is_last_item());
    list.add_range(["Node 3".to_string(), "Node 4".to_string()]);
    layout(&window);
    assert_eq!(vec!["Node 1", "Node 2", "Node 3", "Node 4"], contents(&bar));
    // The unchanged elements are kept.
    assert!(first.ptr_eq(&element(&bar, 1)));
    assert!(!element(&bar, 2).is_last_item());
    assert!(element(&bar, 4).is_last_item());
    list.remove_at(3);
    list.remove_at(2);
    layout(&window);
    assert_eq!(vec!["Node 1", "Node 2"], contents(&bar));
    assert!(element(&bar, 2).is_last_item());
}

/// AddItemsAndCompressBreadcrumbTest: the items that do not fit are hidden behind the ellipsis item,
/// which joins the automation view, and the shown items get their positions.
#[test]
fn the_items_that_do_not_fit_are_hidden_behind_the_ellipsis() {
    let _app = start_with_focus();
    let (window, bar) = show(Some(strings(&["Root", "A", "B", "C", "D"])), 1000.0);
    let panel = panel(&bar);
    assert!(!panel.ellipsis_is_rendered());
    assert_eq!(5, panel.visible_items_count());
    assert_eq!(AccessibilityView::Raw, AutomationProperties::get_accessibility_view(&element(&bar, 0)));
    assert_eq!(0.0, element(&bar, 0).bounds().width);

    // Room for the ellipsis and two items: C and D are shown.
    bar.set_width(3.5 * ITEM_WIDTH);
    layout(&window);
    assert!(panel.ellipsis_is_rendered());
    assert_eq!(4, panel.first_rendered_item_index_after_ellipsis());
    assert_eq!(2, panel.visible_items_count());
    assert_eq!(AccessibilityView::Content, AutomationProperties::get_accessibility_view(&element(&bar, 0)));
    assert_eq!(ITEM_WIDTH, element(&bar, 0).bounds().width);
    for hidden in 1..4 {
        assert_eq!(0.0, element(&bar, hidden).bounds().width);
    }
    assert_eq!(ITEM_WIDTH, element(&bar, 4).bounds().x);
    assert_eq!(1, AutomationProperties::get_position_in_set(&element(&bar, 4)));
    assert_eq!(2, AutomationProperties::get_position_in_set(&element(&bar, 5)));
    assert_eq!(2, AutomationProperties::get_size_of_set(&element(&bar, 5)));
    let hidden: Vec<String> = bar.hidden_elements().into_iter().map(text).collect();
    assert_eq!(vec!["Root", "A", "B"], hidden);
}

/// AddItemsAndInvokeBreadcrumbBarItemTest, KeyboardNavigationItemInvokationTest and
/// CheckCurrentItemDoesNotRaiseInvokeEvent: a click of the button, Enter and the invoke of the peer
/// raise ItemClicked with the item and its index; the current item has no button to click.
#[test]
fn clicks_raise_item_clicked_with_the_item_and_its_index() {
    let _app = start_with_focus();
    let (_window, bar) = show(Some(strings(&["Root", "Node A", "Node B"])), 1000.0);
    let (clicks, _subscription) = track_clicks(&bar);
    let button = element(&bar, 1).item_button().expect("the button of the item");
    button.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(Button::click_event()));
    let args = press(&element(&bar, 2), Key::Enter);
    assert!(args.handled());
    let peer = ControlAutomationPeer::create_peer_for_element(&element(&bar, 3));
    peer.get_provider::<dyn IInvokeProvider>().expect("the invoke pattern").invoke().unwrap();
    assert_eq!(
        vec![(0, "Root".to_string()), (1, "Node A".to_string()), (2, "Node B".to_string())],
        *clicks.borrow()
    );
}

/// InvokeEllipsisItemTest and InvokeDropDownItemTest: the ellipsis item lists the hidden items from
/// the last to the first, and a click of one closes the flyout and raises ItemClicked with its index.
#[test]
fn the_ellipsis_flyout_lists_the_hidden_items_and_clicks_them() {
    let _app = start_with_focus();
    let (_window, bar) = show(Some(strings(&["Root", "A", "B", "C"])), 2.5 * ITEM_WIDTH);
    let (clicks, _subscription) = track_clicks(&bar);
    let ellipsis = element(&bar, 0);
    assert!(panel(&bar).ellipsis_is_rendered());
    press(&ellipsis, Key::Enter);
    let flyout = ellipsis.ellipsis_flyout().unwrap();
    assert!(flyout.is_open());
    let items: Vec<Ref<BreadcrumbBarItem>> = ellipsis
        .ellipsis_items_repeater()
        .unwrap()
        .children()
        .snapshot()
        .iter()
        .map(|child| child.cast::<BreadcrumbBarItem>().unwrap())
        .collect();
    let texts: Vec<String> = items.iter().map(|item| text(item.content())).collect();
    assert_eq!(vec!["B", "A", "Root"], texts);
    assert!(items.iter().all(|item| item.is_ellipsis_drop_down_item()));
    assert!(items[0].classes().contains(":ellipsisdropdown"));
    assert_eq!(1, AutomationProperties::get_position_in_set(&items[0]));
    assert_eq!(3, AutomationProperties::get_size_of_set(&items[0]));

    press(&items[1], Key::Space);
    assert!(!flyout.is_open());
    assert_eq!(vec![(1, "A".to_string())], *clicks.borrow());
}

/// KeyboardNavigationLeftToRightTest, KeyboardNavigationRightToLeftTest and
/// KeyboardNavigationForCrumbledLeftToRightTest: Right moves to the next item, from the ellipsis to
/// the first shown item, and Left back; right to left the keys are mirrored.
#[test]
fn the_arrow_keys_move_the_focus_between_the_shown_items() {
    let _app = start_with_focus();
    let (window, bar) = show(Some(strings(&["Root", "A", "B", "C"])), 3.5 * ITEM_WIDTH);
    // The ellipsis, B and C are shown.
    assert_eq!(3, panel(&bar).first_rendered_item_index_after_ellipsis());
    let ellipsis = element(&bar, 0);
    assert!(ellipsis.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
    assert!(press(&ellipsis, Key::Right).handled());
    assert!(element(&bar, 3).is_focused());
    assert!(press(&element(&bar, 3), Key::Right).handled());
    assert!(element(&bar, 4).is_focused());
    assert!(!press(&element(&bar, 4), Key::Right).handled());
    press(&element(&bar, 4), Key::Left);
    assert!(element(&bar, 3).is_focused());
    press(&element(&bar, 3), Key::Left);
    assert!(ellipsis.is_focused());

    bar.set_flow_direction(FlowDirection::RightToLeft);
    layout(&window);
    assert!(ellipsis.classes().contains(":ellipsisrtl"));
    press(&ellipsis, Key::Left);
    assert!(element(&bar, 3).is_focused());
}

/// The peer of an item: a button, with WinUI's localized control type.
#[test]
fn the_peer_of_an_item_is_a_breadcrumb_bar_item_button() {
    let _app = start_with_focus();
    let (_window, bar) = show(Some(strings(&["Root"])), 1000.0);
    let peer = ControlAutomationPeer::create_peer_for_element(&element(&bar, 1));
    assert_eq!(AutomationControlType::Button, peer.get_automation_control_type());
    assert_eq!("breadcrumb bar item", peer.get_localized_control_type());
    assert_eq!("BreadcrumbBarItem", peer.get_class_name());
}

/// An empty source leaves the ellipsis item only; without a source the panel is empty.
#[test]
fn without_items_the_panel_holds_the_ellipsis_item_only() {
    let _app = start_with_focus();
    let (window, bar) = show(Some(strings(&[])), 1000.0);
    assert_eq!(1, panel(&bar).children().count());
    bar.set_items_source(None);
    layout(&window);
    assert_eq!(0, panel(&bar).children().count());
}

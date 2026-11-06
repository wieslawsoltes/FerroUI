//! Tests of `PagerControl` (WinUI; docs/porting/winui-controls.md, G2 of #418): the API tests of
//! WinUI (`controls/dev/PagerControl/APITests/PagerControlTests.cs`) and the behaviour of
//! `PagerControl.cpp` its interaction tests drive (the number panel, the edge buttons, the
//! display modes).

use crate::automation::peers::{AutomationControlType, ControlAutomationPeer};
use crate::automation::provider::ISelectionProvider;
use crate::automation::AutomationProperties;
use crate::input_test_support::part;
use crate::presenters::ItemsPresenter;
use crate::shapes::Rectangle;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    Button, ComboBox, Control, Grid, ItemsControl, ItemsSource, NumberBox, PagerControl, PagerControlButtonVisibility,
    PagerControlDisplayMode, StackPanel, SymbolIcon, Window,
};
use ferroui_base::input::ICommand;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::layout::ILayoutManager;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{BoxedValue, Ref, StyledElement};
use std::cell::{Cell, RefCell};
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

/// The parts of `Controls/PagerControl.xaml`: the four navigation buttons, the combo box and the
/// number box, and the number panel with the indicator of the current page; the combo box and
/// the number panel show the lists of the template settings, as the bindings of the theme do.
fn pager_control_template() -> Rc<FuncControlTemplate> {
    FuncControlTemplate::new(|parent, scope| {
        let pager = parent.cast::<PagerControl>().expect("a pager");
        let settings = pager.template_settings();
        let root = named(Grid::new(), "RootGrid").register_in_name_scope(&**scope);
        let row = StackPanel::new();
        for name in ["FirstPageButton", "PreviousPageButton"] {
            row.children().add(&named(Button::new(), name).register_in_name_scope(&**scope));
        }
        let combo_box = named(ComboBox::new(), "ComboBoxDisplay").register_in_name_scope(&**scope);
        combo_box.set_items_source(Some(ItemsSource::from(Rc::new(settings.pages()))));
        row.children().add(&combo_box);
        row.children().add(&named(NumberBox::new(), "NumberBoxDisplay").register_in_name_scope(&**scope));
        let panel = named(ItemsControl::new(), "NumberPanelItemsRepeater").register_in_name_scope(&**scope);
        panel.set_items_source(Some(ItemsSource::from(Rc::new(settings.number_panel_items()))));
        // The items control shows its items in a presenter, as its control theme does.
        panel.set_template(Some(FuncControlTemplate::new(|_, scope| {
            named(ItemsPresenter::new(), "PART_ItemsPresenter").register_in_name_scope(&**scope).upcast::<Control>()
        })));
        row.children().add(&panel);
        row.children().add(&named(Rectangle::new(), "NumberPanelCurrentPageIndicator").register_in_name_scope(&**scope));
        for name in ["NextPageButton", "LastPageButton"] {
            row.children().add(&named(Button::new(), name).register_in_name_scope(&**scope));
        }
        root.children().add(&row);
        root.upcast::<Control>()
    })
}

fn pager() -> Ref<PagerControl> {
    let pager = PagerControl::new();
    pager.set_template(Some(pager_control_template()));
    pager
}

fn show(pager: &PagerControl) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(pager.to_ref())));
    window.show();
    window.layout_manager().execute_layout_pass();
    window
}

fn pages(pager: &PagerControl) -> Vec<i32> {
    let pages = pager.template_settings().pages();
    (0..pages.count()).map(|index| pages.get(index)).collect()
}

/// The number panel as text: the page of each button, `...` for an ellipsis.
fn number_panel(pager: &PagerControl) -> Vec<String> {
    let items = pager.template_settings().number_panel_items();
    (0..items.count())
        .map(|index| {
            let item = items.get(index);
            if let Some(button) = item.cast::<Button>() {
                let content = button.content().expect("the page number");
                content.downcast_ref::<i32>().expect("an i32").to_string()
            } else {
                assert!(item.cast::<SymbolIcon>().is_some(), "a button or an ellipsis");
                "...".to_string()
            }
        })
        .collect()
}

fn has(pager: &PagerControl, pseudo_class: &str) -> bool {
    pager.pseudo_classes().contains(pseudo_class)
}

fn click(pager: &PagerControl, name: &str) {
    let button = part::<Button>(pager, name);
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

#[test]
fn the_defaults_are_those_of_winui() {
    let _app = start();
    let pager = PagerControl::new();
    assert_eq!(PagerControlDisplayMode::Auto, pager.display_mode());
    assert_eq!(0, pager.number_of_pages());
    assert_eq!(0, pager.selected_page_index());
    assert_eq!(PagerControlButtonVisibility::Visible, pager.first_button_visibility());
    assert_eq!(PagerControlButtonVisibility::Visible, pager.last_button_visibility());
    assert!(pager.button_panel_always_show_first_last_page_index());
    assert_eq!("", pager.prefix_text());
    assert!(pages(&pager).is_empty());
    // Each pager has its own template settings.
    assert!(!Ref::ptr_eq(&pager.template_settings(), &PagerControl::new().template_settings()));
}

#[test]
fn applying_the_template_sets_the_localized_texts_and_names() {
    let _app = start();
    let pager = pager();
    let _window = show(&pager);
    assert_eq!("Page", pager.prefix_text());
    assert_eq!("of", pager.suffix_text());
    assert_eq!(Some("First page".to_string()), AutomationProperties::get_name(&part::<Button>(&pager, "FirstPageButton")));
    assert_eq!(Some("Next page".to_string()), AutomationProperties::get_name(&part::<Button>(&pager, "NextPageButton")));
    assert_eq!(Some("Page".to_string()), AutomationProperties::get_name(&part::<ComboBox>(&pager, "ComboBoxDisplay")));
    // An application's texts are kept.
    let pager = self::pager();
    pager.set_prefix_text("Seite");
    let _window = show(&pager);
    assert_eq!("Seite", pager.prefix_text());
}

#[test]
fn an_empty_pager_does_not_crash() {
    // VerifyEmptyPagerDoesNotCrash
    let _app = start();
    let pager = pager();
    let _window = show(&pager);
    assert_eq!(0, pager.selected_page_index());
}

#[test]
fn the_combo_box_lists_the_page_numbers() {
    // VerifyComboBoxItemsListNormal
    let _app = start();
    let pager = pager();
    pager.set_number_of_pages(5);
    pager.set_display_mode(PagerControlDisplayMode::ComboBox);
    let _window = show(&pager);
    assert_eq!(vec![1, 2, 3, 4, 5], pages(&pager));
    pager.set_number_of_pages(100);
    assert_eq!((1..=100).collect::<Vec<_>>(), pages(&pager));
    pager.set_number_of_pages(3);
    assert_eq!(vec![1, 2, 3], pages(&pager));
}

#[test]
fn the_combo_box_lists_a_hundred_pages_while_their_number_is_not_known() {
    // VerifyComboBoxItemsInfiniteItems
    let _app = start();
    let pager = pager();
    pager.set_number_of_pages(5);
    pager.set_display_mode(PagerControlDisplayMode::ComboBox);
    let _window = show(&pager);
    pager.set_number_of_pages(-1);
    assert_eq!((1..=100).collect::<Vec<_>>(), pages(&pager));
    pager.set_number_of_pages(200);
    pager.set_number_of_pages(-1);
    assert_eq!((1..=200).collect::<Vec<_>>(), pages(&pager));
}

#[test]
fn the_buttons_of_the_number_panel_state_their_position_in_the_set() {
    // VerifyNumberPanelButtonUIABehavior
    let _app = start();
    let pager = pager();
    pager.set_number_of_pages(5);
    pager.set_display_mode(PagerControlDisplayMode::ButtonPanel);
    let _window = show(&pager);
    let items = pager.template_settings().number_panel_items();
    assert_eq!(5, items.count());
    for index in 0..5 {
        let button = items.get(index);
        assert_eq!(index as i32 + 1, AutomationProperties::get_position_in_set(&button));
        assert_eq!(5, AutomationProperties::get_size_of_set(&button));
        assert_eq!(Some(format!("Page {}", index + 1)), AutomationProperties::get_name(&button));
    }
}

#[test]
fn the_selected_index_changed_event_reports_the_previous_and_the_new_index() {
    // VerifySelectedIndexChangedEventArgs
    let _app = start();
    let pager = pager();
    let last = Rc::new(Cell::new((-2, -2)));
    let seen = last.clone();
    let _subscription = pager.selected_index_changed(move |_, args| {
        seen.set((args.previous_page_index(), args.new_page_index()));
    });
    let _window = show(&pager);
    assert_eq!((-1, 0), last.get());
    pager.set_number_of_pages(10);
    assert_eq!((-1, 0), last.get());
    pager.set_selected_page_index(9);
    assert_eq!((0, 9), last.get());
    pager.set_selected_page_index(4);
    assert_eq!((9, 4), last.get());
}

#[test]
fn the_selected_page_stays_in_the_range_of_the_pages() {
    let _app = start();
    let pager = pager();
    let _window = show(&pager);
    pager.set_number_of_pages(5);
    pager.set_selected_page_index(20);
    assert_eq!(4, pager.selected_page_index());
    pager.set_selected_page_index(-3);
    assert_eq!(0, pager.selected_page_index());
    pager.set_selected_page_index(4);
    pager.set_number_of_pages(3);
    assert_eq!(2, pager.selected_page_index());
    // Without a known number of pages only the lower bound holds.
    pager.set_number_of_pages(-1);
    pager.set_selected_page_index(500);
    assert_eq!(500, pager.selected_page_index());
}

#[test]
fn the_auto_display_mode_switches_to_a_number_box_from_ten_pages() {
    let _app = start();
    let pager = pager();
    pager.set_number_of_pages(9);
    let _window = show(&pager);
    assert!(has(&pager, ":combobox") && !has(&pager, ":numberbox"));
    pager.set_number_of_pages(10);
    assert!(has(&pager, ":numberbox") && !has(&pager, ":combobox"));
    pager.set_number_of_pages(-1);
    assert!(has(&pager, ":numberbox"));
    assert!(has(&pager, ":infinite"));
    pager.set_display_mode(PagerControlDisplayMode::ButtonPanel);
    assert!(has(&pager, ":numberpanel") && !has(&pager, ":numberbox"));
    pager.set_number_of_pages(4);
    assert!(!has(&pager, ":infinite"));
}

#[test]
fn the_edge_buttons_are_disabled_on_the_first_and_the_last_page() {
    let _app = start();
    let pager = pager();
    pager.set_number_of_pages(5);
    let _window = show(&pager);
    assert!(has(&pager, ":firstdisabled") && has(&pager, ":previousdisabled"));
    assert!(!has(&pager, ":nextdisabled") && !has(&pager, ":lastdisabled"));
    pager.set_selected_page_index(2);
    assert!(!has(&pager, ":firstdisabled") && !has(&pager, ":nextdisabled"));
    pager.set_selected_page_index(4);
    assert!(has(&pager, ":nextdisabled") && has(&pager, ":lastdisabled"));
    assert!(!has(&pager, ":firstdisabled"));
    // Without a known number of pages the next button stays enabled.
    pager.set_number_of_pages(-1);
    pager.set_selected_page_index(7);
    assert!(!has(&pager, ":nextdisabled"));
}

#[test]
fn the_button_visibilities_set_the_states_of_the_buttons() {
    let _app = start();
    let pager = pager();
    pager.set_number_of_pages(5);
    pager.set_first_button_visibility(PagerControlButtonVisibility::HiddenOnEdge);
    pager.set_last_button_visibility(PagerControlButtonVisibility::Hidden);
    pager.set_next_button_visibility(PagerControlButtonVisibility::HiddenOnEdge);
    let _window = show(&pager);
    assert!(has(&pager, ":firsthidden"));
    assert!(has(&pager, ":lastcollapsed"));
    pager.set_selected_page_index(1);
    assert!(!has(&pager, ":firsthidden"));
    pager.set_selected_page_index(4);
    // NextPageButtonHidden replaces NextPageButtonDisabled in its group.
    assert!(has(&pager, ":nexthidden") && !has(&pager, ":nextdisabled"));
    pager.set_last_button_visibility(PagerControlButtonVisibility::Visible);
    assert!(!has(&pager, ":lastcollapsed"));
}

#[test]
fn the_number_panel_shows_all_pages_up_to_seven() {
    let _app = start();
    let pager = pager();
    pager.set_display_mode(PagerControlDisplayMode::ButtonPanel);
    pager.set_number_of_pages(7);
    let _window = show(&pager);
    assert_eq!(vec!["1", "2", "3", "4", "5", "6", "7"], number_panel(&pager));
}

#[test]
fn the_number_panel_leaves_pages_out_with_ellipses() {
    let _app = start();
    let pager = pager();
    pager.set_display_mode(PagerControlDisplayMode::ButtonPanel);
    pager.set_number_of_pages(20);
    let _window = show(&pager);
    // 1 2 3 4 5 ... n
    assert_eq!(vec!["1", "2", "3", "4", "5", "...", "20"], number_panel(&pager));
    // 1 ... x-1 x x+1 ... n (x is the selected page)
    pager.set_selected_page_index(9);
    assert_eq!(vec!["1", "...", "9", "10", "11", "...", "20"], number_panel(&pager));
    // 1 ... n-4 n-3 n-2 n-1 n
    pager.set_selected_page_index(17);
    assert_eq!(vec!["1", "...", "16", "17", "18", "19", "20"], number_panel(&pager));
    // Without the first and the last page.
    pager.set_button_panel_always_show_first_last_page_index(false);
    pager.set_selected_page_index(9);
    assert_eq!(vec!["9", "10", "11"], number_panel(&pager));
}

#[test]
fn the_number_panel_without_a_known_number_of_pages() {
    let _app = start();
    let pager = pager();
    pager.set_display_mode(PagerControlDisplayMode::ButtonPanel);
    pager.set_number_of_pages(-1);
    let _window = show(&pager);
    assert_eq!(vec!["1", "2", "3", "4", "5"], number_panel(&pager));
    pager.set_selected_page_index(6);
    assert_eq!(vec!["1", "...", "6", "7", "8"], number_panel(&pager));
}

#[test]
fn a_button_of_the_number_panel_selects_its_page() {
    let _app = start();
    let pager = pager();
    pager.set_display_mode(PagerControlDisplayMode::ButtonPanel);
    pager.set_number_of_pages(5);
    let _window = show(&pager);
    let button = pager.template_settings().number_panel_items().get(3).cast::<Button>().unwrap();
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(3, pager.selected_page_index());
}

#[test]
fn the_indicator_moves_under_the_button_of_the_selected_page() {
    let _app = start();
    let pager = pager();
    pager.set_display_mode(PagerControlDisplayMode::ButtonPanel);
    pager.set_number_of_pages(5);
    let window = show(&pager);
    pager.set_selected_page_index(2);
    window.layout_manager().execute_layout_pass();
    let panel = part::<ItemsControl>(&pager, "NumberPanelItemsRepeater");
    let container = panel.container_from_index(2).unwrap();
    let indicator = part::<Rectangle>(&pager, "NumberPanelCurrentPageIndicator");
    let offset = container.translate_point(ferroui_base::Point::new(0.0, 0.0), &panel).unwrap();
    assert_eq!(offset.x, indicator.margin().left);
    assert_eq!(container.bounds().width, indicator.width());
}

struct CountingCommand(Cell<u32>);

impl ICommand for CountingCommand {
    fn can_execute(&self, _parameter: Option<&BoxedValue>) -> bool {
        true
    }

    fn execute(&self, _parameter: Option<&BoxedValue>) {
        self.0.set(self.0.get() + 1);
    }

    fn can_execute_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

#[test]
fn the_navigation_buttons_move_and_execute_their_commands() {
    let _app = start();
    let pager = pager();
    pager.set_number_of_pages(10);
    let next = Rc::new(CountingCommand(Cell::new(0)));
    pager.set_next_button_command(Some(next.clone()));
    let _window = show(&pager);
    click(&pager, "NextPageButton");
    assert_eq!(1, pager.selected_page_index());
    assert_eq!(1, next.0.get());
    click(&pager, "LastPageButton");
    assert_eq!(9, pager.selected_page_index());
    click(&pager, "PreviousPageButton");
    assert_eq!(8, pager.selected_page_index());
    click(&pager, "FirstPageButton");
    assert_eq!(0, pager.selected_page_index());
}

#[test]
fn the_combo_box_and_the_number_box_select_the_page() {
    let _app = start();
    let pager = pager();
    pager.set_number_of_pages(8);
    let _window = show(&pager);
    let combo_box = part::<ComboBox>(&pager, "ComboBoxDisplay");
    let number_box = part::<NumberBox>(&pager, "NumberBoxDisplay");
    assert_eq!(1.0, number_box.value());
    assert_eq!(8.0, number_box.maximum());
    combo_box.set_selected_index(5);
    assert_eq!(5, pager.selected_page_index());
    assert_eq!(6.0, number_box.value());
    number_box.set_numeric_value(3.0);
    assert_eq!(2, pager.selected_page_index());
    assert_eq!(2, combo_box.selected_index());
    pager.set_number_of_pages(-1);
    assert_eq!(f64::INFINITY, number_box.maximum());
}

#[test]
fn the_peer_is_a_menu_with_the_selection_pattern() {
    // VerifyAutomationPeerBehavior (the landmark is set by the style of the WinUI theme).
    let _app = start();
    let pager = pager();
    pager.set_number_of_pages(5);
    let _window = show(&pager);
    let peer = ControlAutomationPeer::create_peer_for_element(&pager);
    assert_eq!(AutomationControlType::Menu, peer.get_automation_control_type());
    assert_eq!("PagerControl", peer.get_class_name());
    let provider = peer.get_provider::<dyn ISelectionProvider>().unwrap();
    assert!(!provider.can_select_multiple());
    assert!(provider.is_selection_required());
    assert!(provider.get_selection().is_empty());
}

#[test]
fn a_dropped_handler_is_not_called() {
    let _app = start();
    let pager = pager();
    let calls = Rc::new(RefCell::new(0));
    let seen = calls.clone();
    let subscription = pager.selected_index_changed(move |_, _| *seen.borrow_mut() += 1);
    let _window = show(&pager);
    // Applying the template raises it (more than once: the maximum of the number box, 0 pages,
    // coerces its value to 0, which selects page -1, coerced to 0, as in WinUI).
    let raised = *calls.borrow();
    assert!(raised >= 1);
    pager.set_number_of_pages(3);
    pager.set_selected_page_index(1);
    assert_eq!(raised + 1, *calls.borrow());
    drop(subscription);
    pager.set_selected_page_index(2);
    assert_eq!(raised + 1, *calls.borrow());
}

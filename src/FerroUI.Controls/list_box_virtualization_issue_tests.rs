//! Tests of issues of the list box with a virtualizing items panel.
//!
//! Observable collections of the reference tests are notifying lists and
//! plain lists are non-notifying items sources.
//!
//! The three `opening_split_view_pane_*` tests of the reference run in the
//! test application with the styled window services; here the theme is the
//! test theme plus the themes of the list box, the list box item, the scroll
//! viewer, the scroll bar and the repeat button, which mirror the reference
//! simple theme (`testing/test_theme_split_view.rs`).

use crate::presenters::{ItemsPresenter, ScrollContentPresenter};
use crate::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
    IDataTemplate, ITemplateOf,
};
use crate::test_support::{string_of, test_scope, TestRoot, TestScope};
use crate::testing::{create_split_view_list_box_theme, TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    unbox_item, Border, Control, Grid, ItemsControl, ItemsSource, ListBox, ListBoxItem, Panel, RowDefinitions,
    ScrollViewer, SelectionMode, SplitView, SplitViewDisplayMode, TextBlock, VirtualizingStackPanel, Window,
};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::collections::FerroList;
use ferroui_base::data::{BindingMode, IndexerBinding};
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::KeyboardNavigation;
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::platform::{DefaultPlatformSettings, IPlatformRenderInterface};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions, Ref, Vector};
use std::cell::Cell;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

/// An item with an identity. Every test creates items with distinct ids, so
/// that comparing the ids is the reference equality of the reference.
#[derive(PartialEq)]
struct Item {
    id: i32,
}

impl Item {
    fn new(id: i32) -> Rc<Self> {
        Rc::new(Self { id })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn start() -> TestScope {
    let scope = test_scope();
    FerroLocator::current_mutable()
        .bind::<PlatformHotkeyConfiguration>()
        .to_constant(Rc::new(PlatformHotkeyConfiguration::default()));
    scope
}

fn list_box_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ListBox>(|parent, scope| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        let property = ItemsControl::items_panel_property().as_property();
        presenter.bind_binding(property, &IndexerBinding::new(parent.clone().upcast(), property, BindingMode::OneWay));

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
        scroll_viewer.set_template(Some(scroll_viewer_template()));
        scroll_viewer.set_content(Some(Control::boxed(presenter.register_in_name_scope(&**scope))));
        scroll_viewer.register_in_name_scope(&**scope).upcast()
    })
}

fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

/// A test root with the platform settings that the test application of the
/// reference supplies.
fn test_root() -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.set_platform_settings(Some(Rc::new(DefaultPlatformSettings::new())));
    root
}

fn prepare(target: &Ref<ListBox>) -> Ref<TestRoot> {
    target.set_height(100.0);
    target.set_width(100.0);
    let root = test_root();
    root.set_child(target.clone());
    root.execute_initial_layout_pass();
    root
}

/// An items panel template that creates a virtualizing stack panel, with the
/// given cache length when there is one.
fn virtualizing_items_panel(cache_length: Option<f64>) -> Rc<dyn ITemplateOf<Option<Ref<Panel>>>> {
    FuncTemplate::new(move || {
        let panel = VirtualizingStackPanel::new();
        if let Some(cache_length) = cache_length {
            panel.set_cache_length(cache_length);
        }
        Some(panel.upcast::<Panel>())
    })
}

/// A data template for items of type `T` that creates a text block of height
/// 50.
fn text_block_template<T: 'static>() -> Option<Rc<dyn IDataTemplate>> {
    Some(FuncDataTemplate::for_type::<T>(
        |_, _| {
            let text_block = TextBlock::new();
            text_block.set_height(50.0);
            Some(text_block.upcast())
        },
        false,
    ))
}

/// The letters of `text` as strings.
fn characters(text: &str) -> Vec<String> {
    text.chars().map(|c| c.to_string()).collect()
}

/// A plain list of strings as an items source.
fn strings(values: &[String]) -> Option<ItemsSource> {
    Some(ItemsSource::from_strs(values.iter().map(String::as_str)))
}

/// A notifying list of the items "Item 0", "Item 1", ...
fn numbered_list(count: usize) -> Rc<FerroList<String>> {
    Rc::new(FerroList::from_items((0..count).map(|x| format!("Item {x}"))))
}

fn panel_of(target: &ListBox) -> Ref<Panel> {
    target.presenter().expect("no presenter").panel().expect("no panel")
}

fn panel_child(target: &ListBox, index: usize) -> Ref<ListBoxItem> {
    panel_of(target).children().get(index).cast::<ListBoxItem>().expect("the child is not a list box item")
}

fn visible_children(panel: &Panel) -> Vec<Ref<Control>> {
    panel.children().to_vec().into_iter().filter(|c| c.is_visible()).collect()
}

fn realized_list_box_items(target: &ListBox) -> Vec<Ref<ListBoxItem>> {
    target
        .get_realized_containers()
        .into_iter()
        .map(|x| x.cast::<ListBoxItem>().expect("the container is not a list box item"))
        .collect()
}

/// The content of the container as a string, when it is one.
fn content_string(container: &ListBoxItem) -> Option<String> {
    container.content().as_ref().and_then(string_of)
}

// ---------------------------------------------------------------------------
// Split view helpers
// ---------------------------------------------------------------------------

/// An item that its data template shows as a border of the given size.
#[derive(PartialEq)]
struct SizedItem {
    width: f64,
    height: f64,
}

/// The global clock of the split view tests: never pulses.
struct MockGlobalClock {
    subject: LightweightSubject<TimeSpan>,
    play_state: Cell<PlayState>,
}

impl IObservable<TimeSpan> for MockGlobalClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl IClock for MockGlobalClock {
    fn play_state(&self) -> PlayState {
        self.play_state.get()
    }

    fn set_play_state(&self, value: PlayState) {
        self.play_state.set(value)
    }
}

impl IGlobalClock for MockGlobalClock {}

/// The test application and the text services it runs with.
struct TestApplication {
    // Dropped in this order: the application ends before the text services.
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

/// Starts the test application with the styled window services and a mock
/// global clock.
fn start_styled_window() -> TestApplication {
    let text = TextTestScope::new();
    let render_interface = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .expect("the text services have a render interface");
    let clock: Rc<dyn IGlobalClock> =
        Rc::new(MockGlobalClock { subject: LightweightSubject::new(), play_state: Cell::new(PlayState::Run) });
    let app = UnitTestApplication::start(
        TestServices::styled_window()
            .with_theme(create_split_view_list_box_theme)
            .with_global_clock(clock)
            .with_render_interface(render_interface),
    );
    TestApplication { _app: app, _text: text }
}

const SIZES: [(f64, f64); 6] =
    [(196.0, 331.0), (186.0, 258.0), (196.0, 321.0), (186.0, 296.0), (150.0, 340.0), (196.0, 319.0)];

fn create_sized_list_box() -> Ref<ListBox> {
    let items = SIZES.iter().map(|&(width, height)| {
        let item: BoxedValue = Rc::new(SizedItem { width, height });
        Some(item)
    });

    let target = ListBox::new();
    target.set_items_source(Some(ItemsSource::from_items(items)));
    target.set_item_template(Some(FuncDataTemplate::for_type::<SizedItem>(
        |item, _| {
            let border = Border::new();
            border.set_width(item.width);
            border.set_height(item.height);
            Some(border.upcast())
        },
        false,
    )));
    target.set_items_panel(virtualizing_items_panel(None));
    target.set_selection_mode(SelectionMode::SINGLE | SelectionMode::ALWAYS_SELECTED);
    target
}

fn create_split_view_window(list_box: &Ref<ListBox>) -> (Ref<Window>, Ref<SplitView>) {
    let split_view = SplitView::new();
    split_view.set_display_mode(SplitViewDisplayMode::CompactInline);
    split_view.set_compact_pane_length(0.0);
    split_view.set_open_pane_length(300.0);
    split_view.set_pane(Some(Control::boxed(list_box.clone())));
    split_view.set_content(Some(Control::boxed(TextBlock::new())));

    let grid = Grid::new();
    grid.set_row_definitions(RowDefinitions::parse("30,*").expect("valid row definitions"));
    grid.children().add(TextBlock::new());
    grid.children().add(split_view.clone());

    let window = Window::new();
    window.set_width(800.0);
    window.set_height(804.0);
    window.set_content(Some(Control::boxed(grid)));
    Grid::set_row(&split_view, 1);
    window.show();
    (window, split_view)
}

fn scroll_while_pane_is_closed(list_box: &ListBox, window: &Window) {
    assert_eq!(0.0, list_box.bounds().width);

    for index in 1..=3 {
        list_box.set_selected_index(index);
        window.layout_manager().execute_layout_pass();
    }
}

fn open_pane(split_view: &SplitView, list_box: &ListBox, window: &Window) {
    // Disable the theme animation so that the assertions observe the layout
    // of the opened pane directly.
    let pane_roots: Vec<Ref<Panel>> = split_view
        .get_visual_descendants()
        .filter_map(|x| x.cast::<Panel>())
        .filter(|x| x.name().as_deref() == Some("PART_PaneRoot"))
        .collect();
    assert_eq!(1, pane_roots.len());
    pane_roots[0].set_transitions(None);

    split_view.set_is_pane_open(true);
    window.layout_manager().execute_layout_pass();

    assert_eq!(300.0, list_box.bounds().width);
}

fn assert_visible_children_are_realized(list_box: &ListBox) {
    let panel = panel_of(list_box);
    assert!(panel.is::<VirtualizingStackPanel>());
    let realized = list_box.get_realized_containers();

    for child in visible_children(&panel) {
        assert_ne!(-1, list_box.index_from_container(&child));
        assert!(realized.contains(&child));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn opening_split_view_pane_after_scrolling_list_box_does_not_show_unrealized_containers() {
    let _app = start_styled_window();

    let target = create_sized_list_box();
    let (window, split_view) = create_split_view_window(&target);

    // Scroll the selected item into view while the split view pane is
    // effectively hidden.
    scroll_while_pane_is_closed(&target, &window);

    open_pane(&split_view, &target, &window);

    assert_visible_children_are_realized(&target);
    let realized = target.get_realized_containers();

    let selected_container = target
        .container_from_index(target.selected_index())
        .and_then(|x| x.cast::<ListBoxItem>())
        .expect("no list box item");
    assert!(realized.contains(&selected_container.clone().upcast::<Control>()));
    let scroll = target.scroll().and_then(|x| x.cast::<ScrollViewer>()).expect("no scroll viewer");
    assert!(selected_container.bounds().bottom() > scroll.offset().y);
    assert!(selected_container.bounds().top() < scroll.offset().y + scroll.viewport().height);
}

#[test]
fn opening_split_view_pane_after_scrolling_keeps_tab_once_container_indexed() {
    let _app = start_styled_window();

    let target = create_sized_list_box();
    let (window, split_view) = create_split_view_window(&target);

    scroll_while_pane_is_closed(&target, &window);

    let tab_once_container =
        target.container_from_index(0).and_then(|x| x.cast::<ListBoxItem>()).expect("no list box item");
    KeyboardNavigation::set_tab_once_active_element(&target, Some(tab_once_container.upcast()));

    open_pane(&split_view, &target, &window);

    let active_container = KeyboardNavigation::get_tab_once_active_element(&target)
        .and_then(|x| x.cast::<ListBoxItem>())
        .expect("no list box item");
    let active_container: Ref<Control> = active_container.upcast();
    let active_index = target.index_from_container(&active_container);

    assert!((0..=5).contains(&active_index));
    assert_eq!(Some(active_container), target.container_from_index(active_index));
}

#[test]
fn opening_split_view_pane_after_scrolling_own_container_items_does_not_show_unrealized_containers() {
    let _app = start_styled_window();

    let items = SIZES.iter().map(|&(width, height)| {
        let border = Border::new();
        border.set_width(width);
        border.set_height(height);
        let item = ListBoxItem::new();
        item.set_content(Some(Control::boxed(border)));
        Some(Control::boxed(item))
    });
    let target = ListBox::new();
    target.set_items_source(Some(ItemsSource::from_items(items)));
    target.set_items_panel(virtualizing_items_panel(None));
    target.set_selection_mode(SelectionMode::SINGLE | SelectionMode::ALWAYS_SELECTED);
    let (window, split_view) = create_split_view_window(&target);

    scroll_while_pane_is_closed(&target, &window);
    open_pane(&split_view, &target, &window);

    assert_visible_children_are_realized(&target);
}


#[test]
fn removing_first_item_after_scrolling_to_end_should_allow_scrolling_to_start() {
    let _app = start();
    let items = Rc::new(FerroList::from_items(0..100));
    let target = ListBox::new();
    target.set_template(Some(list_box_template()));
    target.set_items_source(Some(items.clone().into()));
    target.set_item_template(text_block_template::<i32>());
    target.set_items_panel(virtualizing_items_panel(None));

    let _root = prepare(&target);
    target.scroll_into_view(99);

    items.remove_at(0);
    target.scroll_into_view(0);

    let first_container =
        target.container_from_index(0).and_then(|x| x.cast::<ListBoxItem>()).expect("no list box item");
    assert_eq!(Some(1), unbox_item::<i32>(&first_container.content()));
}

#[test]
fn replaced_items_source_should_not_show_old_selected_item_when_scrolled_back() {
    let _app = start();
    let letters = characters("ABCDEFGHIJ");
    let numbers = characters("0123456789");

    let target = ListBox::new();
    target.set_template(Some(list_box_template()));
    target.set_items_source(strings(&letters));
    target.set_item_template(text_block_template::<String>());
    target.set_height(100.0); // Show 2 items
    target.set_items_panel(virtualizing_items_panel(Some(0.0)));

    let _root = prepare(&target);

    // 1. Select a list box item
    target.set_selected_index(0);
    assert!(panel_child(&target, 0).is_selected());

    // 2. Scroll until the selected list box item is no longer visible
    target.scroll_into_view(letters.len() as i32 - 1); // Scroll down to the last item

    // Verify that the first item is no longer realized
    let realized_containers = realized_list_box_items(&target);
    assert!(!realized_containers.iter().any(|x| content_string(x).as_deref() == Some("A")));

    // 3. Change the items source
    target.set_items_source(strings(&numbers));

    // 4. Scroll to the top
    target.scroll_into_view(0);

    // 5. The previously selected list box item should NOT appear in the list box
    let realized_items: Vec<Option<String>> =
        realized_list_box_items(&target).iter().map(|x| content_string(x)).collect();

    for item in &realized_items {
        assert!(!item.as_ref().is_some_and(|item| letters.contains(item)), "{item:?} is an old item");
    }
    assert_eq!(Some("0"), realized_items[0].as_deref());
}

#[test]
fn adding_items_at_top_should_not_create_ghost_items() {
    let _app = start();
    let items: Rc<FerroList<Rc<Item>>> = Rc::new(FerroList::new());
    for i in 0..100 {
        items.add(Item::new(i));
    }

    let target = ListBox::new();
    target.set_template(Some(list_box_template()));
    target.set_items_source(Some(items.clone().into()));
    target.set_height(100.0); // Show 2 items
    target.set_items_panel(virtualizing_items_panel(None));

    let _root = prepare(&target);

    // Scroll to some position
    let scroll_viewer = target.visual_children().get(0).cast::<ScrollViewer>().expect("no scroll viewer");
    scroll_viewer.set_offset(Vector::new(0.0, 500.0)); // Scrolled down
    target.update_layout();

    // Add items at the top multiple times
    for i in 0..5 {
        for j in 0..10 {
            items.insert(0, Item::new(1000 + (i * 100 + j)));
        }

        target.update_layout();

        // Randomly select something
        target.set_selected_index(items.count() as i32 - 1);
        target.update_layout();

        // Scroll a bit
        scroll_viewer.scroll_to_end();
        scroll_viewer.scroll_to_end();
        target.update_layout();

        // Check for ghost items during the process
        let p = panel_of(&target);
        let visible = visible_children(&p);
        let realized_containers = realized_list_box_items(&target);
        let realized_controls: Vec<Ref<Control>> = realized_containers.iter().map(|x| x.clone().upcast()).collect();

        // Only visible children should be considered. Invisible children may
        // be recycled items kept for reuse.
        assert_eq!(realized_containers.len(), visible.len());
        for child in &visible {
            assert!(realized_controls.contains(child));
        }

        let realized_items: Vec<Rc<Item>> = realized_containers
            .iter()
            .map(|x| unbox_item::<Rc<Item>>(&x.content()).expect("the content is not an item"))
            .collect();

        // Check for duplicates in realized items
        let mut duplicate_ids: Vec<i32> = Vec::new();
        for item in &realized_items {
            if realized_items.iter().filter(|x| x.id == item.id).count() > 1 && !duplicate_ids.contains(&item.id) {
                duplicate_ids.push(item.id);
            }
        }
        assert!(duplicate_ids.is_empty(), "duplicate ids: {duplicate_ids:?}");

        // Check if all realized items are actually in the items source
        for item in &realized_items {
            assert!(items.contains(item));
        }

        // Check if realized items are in the correct order
        let mut last_index = -1;
        for item in &realized_items {
            let current_index = items.index_of(item).map_or(-1, |x| x as i32);
            assert!(
                current_index > last_index,
                "Item {} is at index {current_index}, but previous item was at index {last_index}",
                item.id
            );
            last_index = current_index;
        }

        // New check: verify that all visual children of the panel are
        // accounted for in the realized containers
        let panel = panel_of(&target);
        let visual_children = panel.children().to_vec();

        // Realized containers should match exactly the visual children of the
        // panel (the virtualizing stack panel manages its children such that
        // they should be the realized containers).
        // We also check if all children are visible, if not they might be
        // "ghosts"
        for child in &visual_children {
            assert!(
                child.is_visible(),
                "Child {:?} should be visible",
                child
                    .clone()
                    .cast::<ListBoxItem>()
                    .and_then(|x| unbox_item::<Rc<Item>>(&x.content()))
                    .map(|x| x.id)
            );
        }

        assert_eq!(realized_containers.len(), visual_children.len());
        for child in &visual_children {
            assert!(realized_controls.contains(child));
        }
    }
}

#[test]
fn realized_containers_should_only_include_visible_items_with_cache_length_zero() {
    let _app = start();
    let letters = characters("ABCDEFGHIJ");

    let target = ListBox::new();
    target.set_items_panel(virtualizing_items_panel(Some(0.0)));
    target.set_template(Some(list_box_template()));
    target.set_items_source(strings(&letters));
    target.set_item_template(text_block_template::<String>());
    target.set_height(100.0); // Show 2 items (100 / 50 = 2)

    let _root = prepare(&target);

    // At the top, only 2 items should be visible (items at index 0 and 1)
    let realized_containers = realized_list_box_items(&target);

    // With a cache length of 0, we should only have the visible items realized
    assert_eq!(2, realized_containers.len());
    assert_eq!(Some("A"), content_string(&realized_containers[0]).as_deref());
    assert_eq!(Some("B"), content_string(&realized_containers[1]).as_deref());
}

#[test]
fn ghost_item_test_focus_management() {
    let _app = start();
    let items = numbered_list(100);

    let target = ListBox::new();
    target.set_template(Some(list_box_template()));
    target.set_items_source(Some(items.clone().into()));
    target.set_item_template(text_block_template::<String>());
    target.set_height(100.0); // Show 2 items
    target.set_items_panel(virtualizing_items_panel(Some(0.0)));

    let _root = prepare(&target);

    // 1. Get the first container and focus it
    let container: Ref<Control> = panel_child(&target, 0).upcast();
    KeyboardNavigation::set_tab_once_active_element(&target, Some(container.clone().upcast()));

    // 2. Scroll down so the first item is recycled
    target.scroll_into_view(10);
    target.update_layout();

    // 3. Verify it is now the focused element in the panel
    let panel = panel_of(&target).cast::<VirtualizingStackPanel>().expect("no virtualizing stack panel");

    let realized_containers = target.get_realized_containers();

    // The focused container should still be in the children, but NOT in the
    // realized containers
    assert!(panel.children().contains(&container));
    assert!(!realized_containers.contains(&container));

    // Now scroll back to top.
    target.scroll_into_view(0);
    target.update_layout();

    // Check if we have two containers for the same item or other weirdness
    let visible = visible_children(&panel);
    // If it was a ghost, it might still be there or we might have two items
    // for the same thing
    assert_eq!(target.get_realized_containers().len(), visible.len());

    // 4. Test: Re-insert at top might cause issues if the focused element is
    // not updated correctly
    items.insert(0, "New Item".to_string());
    target.update_layout();

    let visible = visible_children(&panel);
    assert_eq!(target.get_realized_containers().len(), visible.len());

    // 5. Remove the focused item while it's recycled
    target.scroll_into_view(10);
    target.update_layout();
    assert!(panel.children().contains(&container));

    items.remove_at(1); // Item 0 was at index 1 because of the insert of "New Item" at 0
    target.update_layout();

    // The container should be removed from the children because the element
    // is recycled when its item is removed
    assert!(!panel.children().contains(&container));
    assert!(!container.is_visible());
}

#[test]
fn ghost_item_test_scroll_to_management() {
    let _app = start();
    let items = numbered_list(100);

    let target = ListBox::new();
    target.set_template(Some(list_box_template()));
    target.set_items_source(Some(items.clone().into()));
    target.set_item_template(text_block_template::<String>());
    target.set_height(100.0); // Show 2 items
    target.set_items_panel(virtualizing_items_panel(Some(0.0)));

    let _root = prepare(&target);

    // 1. Scroll into view to trigger the scroll-to element
    // We use a high index and don't update the layout immediately if we want
    // to catch it in between?
    // Actually scrolling into view runs the layout internally.
    target.scroll_into_view(50);

    let panel = panel_of(&target).cast::<VirtualizingStackPanel>().expect("no virtualizing stack panel");

    // 2. Remove the item we just scrolled to
    items.remove_at(50);
    target.update_layout();

    // If it was kept as the scroll-to element and not recycled, it might be a
    // ghost.
    let visible = visible_children(&panel);
    assert_eq!(target.get_realized_containers().len(), visible.len());
}

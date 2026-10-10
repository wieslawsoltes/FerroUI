//! The real application of the sample, headless: the application ([`App`]: `App.xaml`, the
//! Fluent theme and the resources of the hamburger menu) with its main window, which renders
//! through the compositor of the test with Skia and takes the input of a mouse
//! (`sample_testing::Shell`).
//!
//! Not ports: the upstream sample has no tests. Each test selects a page of the hamburger
//! menu and looks at what its list realizes, shows and recycles; the last one visits every
//! page and compares the errors the bindings reported with the accepted ones.

use crate::view_models::MainWindowViewModel;
use crate::views::{ChatPageView, ExpanderPageView, PlaygroundPageView};
use crate::{App, MainWindow, SAMPLE};
use control_samples::HamburgerMenu;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ObjectType, Ref, Vector, Visual};
use ferroui_controls::{Button, Control, Expander, ListBox, ListBoxItem, ScrollViewer, TabItem, TextBlock, TextBox};
use sample_testing::Shell;
use std::rc::Rc;
use std::time::Duration;

/// The size of the window of the test.
const SIZE: (f64, f64) = (1000.0, 700.0);

/// The pages of the hamburger menu, in the order of the document.
const PAGES: [&str; 3] = ["Playground", "Chat", "Expanders"];

/// The reports of the bindings of the window that are accepted (`(place, report, count)`).
/// The managed original makes each of them too: the template of the hamburger menu binds
/// members of the selected item of the menu, the menu has no selected item while it sorts
/// its pages (it clears its items and adds them again when it is loaded), and a null in the
/// middle of a path is an error of the binding upstream too. The pages of the sample report
/// nothing; the bindings of the flyout of the playground are created when the flyout opens.
const ACCEPTED: &[(&str, &str, usize)] = &[
    // HamburgerMenu.xaml: the scroll bars of the content take the attached properties of the selected item.
    ("MainWindow", "ScrollViewer.HorizontalScrollBarVisibility <- $templatedParent.SelectedItem.HorizontalScrollBarVisibility at SelectedItem: Value is null.", 1),
    ("MainWindow", "ScrollViewer.VerticalScrollBarVisibility <- $templatedParent.SelectedItem.VerticalScrollBarVisibility at SelectedItem: Value is null.", 1),
    // HamburgerMenu.xaml: the title is the header of the selected item (with an empty fallback value).
    ("MainWindow", "TextBlock.Text <- $parent[TabControl].SelectedItem.Header at SelectedItem: Value is null.", 1),
];

/// The application of the sample with its main window shown.
struct Demo {
    window: Ref<MainWindow>,
    view_model: Rc<MainWindowViewModel>,
    shell: Shell,
}

impl Demo {
    fn start() -> Demo {
        let shell = Shell::start(&SAMPLE, SIZE.0, SIZE.1, || App::new().upcast());
        let window = MainWindow::new();
        window.show();
        shell.settle();
        let view_model =
            from_markup_value::<Rc<MainWindowViewModel>>(&window.data_context()).expect("the view model of the window");
        Demo { window, view_model, shell }
    }

    fn menu(&self) -> Ref<HamburgerMenu> {
        shown::<HamburgerMenu>(&self.window).into_iter().next().expect("the hamburger menu of the window")
    }

    /// Selects the page `index` of [`PAGES`] and runs the frames of the change. The menu
    /// sorts its pages by their headers when it is loaded, so the page is found by its
    /// header.
    fn select_page(&self, index: usize) {
        let menu = self.menu();
        let tabs: Vec<Ref<TabItem>> = menu.items().view().to_vec().iter().filter_map(from_markup_value::<Ref<TabItem>>).collect();
        assert_eq!(PAGES.len(), tabs.len(), "the pages of the menu");
        let header = |tab: &Ref<TabItem>| from_markup_value::<String>(&tab.header());
        let position = tabs.iter().position(|tab| header(tab).as_deref() == Some(PAGES[index])).unwrap_or_else(|| panic!("the menu has no page {}", PAGES[index]));
        menu.set_selected_index(position as i32);
        self.shell.settle();
        let item = from_markup_value::<Ref<TabItem>>(&menu.selected_item()).expect("the selected page");
        assert_eq!(Some(PAGES[index].to_string()), header(&item), "the header of the selected page");
    }

    /// The view of the class `T` the selected page shows.
    fn view<T: ObjectType>(&self) -> Ref<T> {
        shown::<T>(&self.window).into_iter().next().unwrap_or_else(|| panic!("the view of the selected page"))
    }

    /// The last frame of the window has more than a background where `visual` is.
    fn assert_drawn(&self, visual: &Visual, context: &str) {
        assert!(self.shell.frames(0) > 0, "{context}: the compositor drew the window");
        let frame = self.shell.last_frame(0);
        let rect = Shell::frame_rect_of(&self.window, visual);
        let colors = frame.colors(rect);
        assert!(colors > 2, "{context}: {colors} colours are drawn in {rect:?}");
    }

    /// Presses and releases the left button of the mouse in the middle of `visual`.
    fn click(&self, visual: &Visual) {
        self.shell.click(0, Shell::bounds_of(&self.window, visual).center());
    }

    /// The reports of the bindings since the last call, as the guard takes them.
    fn binding_reports(&self) -> Vec<(String, String, usize)> {
        self.shell.take_binding_reports().into_iter().map(|(report, count)| ("MainWindow".to_string(), report.line(), count)).collect()
    }
}

impl Drop for Demo {
    fn drop(&mut self) {
        self.window.close();
    }
}

/// The elements of the class `T` below `root` that are shown.
fn shown<T: ObjectType>(root: &Visual) -> Vec<Ref<T>> {
    root.get_visual_descendants().filter(|visual| visual.is_effectively_visible()).filter_map(|visual| visual.cast::<T>()).collect()
}

/// The texts of the text blocks below `root` that are shown.
fn texts(root: &Visual) -> Vec<String> {
    shown::<TextBlock>(root).into_iter().filter_map(|text_block| text_block.text()).collect()
}

/// Changes the text of a text box the way typing does: the current value of the property,
/// which a binding of the property writes to its source.
fn type_text(text_box: &Ref<TextBox>, text: &str) {
    text_box.set_current_value(TextBox::text_property(), Some(text.to_string()));
}

fn scroll_viewer_of(list: &Ref<ListBox>) -> Ref<ScrollViewer> {
    list.find_descendant_of_type::<ScrollViewer>(false).expect("the scroll viewer of the list box")
}

/// The indexes of the items the list box has realized containers for, in their order.
fn realized_indexes(list: &Ref<ListBox>) -> Vec<i32> {
    let mut indexes: Vec<i32> = list.get_realized_containers().iter().map(|container| list.index_from_container(container)).collect();
    indexes.sort_unstable();
    indexes
}

/// The list box realizes containers for a part of its `count` items only, and its panel has
/// no more children than that.
fn assert_virtualized(list: &Ref<ListBox>, count: i32, context: &str) -> Vec<Ref<Control>> {
    assert_eq!(count, list.item_count(), "{context}: the items of the list box");
    let containers = list.get_realized_containers();
    assert!(!containers.is_empty(), "{context}: the list box realized containers");
    assert!((containers.len() as i32) < count, "{context}: {} of {count} items are realized", containers.len());
    let children = list.items_panel_root().expect("the panel of the list box").children().count();
    assert!(children >= containers.len() && (children as i32) < count, "{context}: the panel has {children} children");
    containers
}

/// The page "Playground": a list box over a thousand items that realizes what is in view,
/// recycles its containers when it scrolls, follows the selection of the view model, and
/// the controls beside it.
#[test]
fn the_playground_realizes_what_is_in_view_and_recycles_its_containers() {
    let demo = Demo::start();
    demo.select_page(0);
    let view = demo.view::<PlaygroundPageView>();
    let view_model = demo.view_model.playground();
    let list = view.get_control::<ListBox>("list");
    let before = assert_virtualized(&list, 1000, "at the top");
    assert_eq!(0, realized_indexes(&list)[0]);
    // `DisplayMemberBinding="{Binding Header}"`.
    let shown_texts = texts(&list);
    assert!(shown_texts.iter().any(|text| text == "Item 0"), "the list shows its first items: {shown_texts:?}");
    demo.assert_drawn(&list, "the playground at the top");
    let top = demo.shell.last_frame(0);

    // The view counts what is realized, twice a second, while it is in the tree: the timer
    // of the view is scheduled (the clock of the dispatcher does not advance in a test, so
    // the test fires it).
    let item_count = view.get_control::<TextBlock>("itemCount");
    assert_eq!(None, item_count.text());
    let timer = Dispatcher::timers_for_unit_tests()
        .into_iter()
        .find(|timer| timer.interval() == Duration::from_millis(500))
        .expect("the timer of the view runs while the view is in the tree");
    Dispatcher::force_fire_timer_for_unit_tests(&timer);
    let children = list.items_panel_root().expect("the panel").children().count();
    assert_eq!(Some(format!("Realized {} of {children}", list.get_realized_containers().len())), item_count.text());

    // Scrolling to the middle realizes other items, in containers the list had already.
    let scroll_viewer = scroll_viewer_of(&list);
    let extent = scroll_viewer.extent();
    assert!(extent.height > scroll_viewer.viewport().height * 10.0, "the extent is the extent of all the items: {extent:?}");
    scroll_viewer.set_offset(Vector::new(0.0, extent.height / 2.0));
    demo.shell.settle();
    let after = assert_virtualized(&list, 1000, "in the middle");
    let indexes = realized_indexes(&list);
    assert!(indexes[0] > 100 && *indexes.last().expect("an index") < 900, "the items in the middle are realized: {indexes:?}");
    assert!(!texts(&list).iter().any(|text| text == "Item 0"), "the first item is no longer shown");
    let recycled = after.iter().filter(|container| before.contains(container)).count();
    assert!(recycled > 0, "the containers of the items that left the view are used again");
    let rect = Shell::frame_rect_of(&demo.window, &list);
    assert!(demo.shell.last_frame(0).difference(&top, rect) > 0, "the list is drawn with the other items");

    // Pressing an item with the mouse selects it in the model of the view model.
    let pressed = after[after.len() / 2].clone();
    let pressed_index = list.index_from_container(&pressed);
    demo.click(&pressed);
    assert_eq!(pressed_index, view_model.selection().selected_index());
    assert!(pressed.cast::<ListBoxItem>().is_some_and(|item| item.is_selected()));

    // The text box of the index writes to the view model, and selecting that index through
    // the view model scrolls the item into view (`AutoScrollToSelectedItem`).
    let scroll_to_index = view.get_control::<TextBox>("scrollToIndex");
    assert_eq!(Some(String::from("500")), scroll_to_index.text());
    type_text(&scroll_to_index, "20");
    demo.shell.settle();
    assert_eq!(20, view_model.scroll_to_index());
    view_model.execute_scroll_to_index();
    demo.shell.settle();
    assert!(view_model.selection().is_selected(20));
    let container = list.container_from_index(20).expect("the selected item is scrolled into view");
    assert!(container.cast::<ListBoxItem>().is_some_and(|item| item.is_selected()));

    // The button below the text boxes, pressed with the mouse, removes the selected items
    // (the selection mode of the list is multiple: the pressed item and the item 20).
    let selected = view_model.selection().count();
    assert_eq!(2, selected, "the selection mode of the view model is the one of the list box");
    let delete = shown::<Button>(&view)
        .into_iter()
        .find(|button| from_markup_value::<String>(&button.content()).is_some_and(|content| content.trim() == "Delete Selected"))
        .expect("the delete button");
    demo.click(&delete);
    assert_eq!(1000 - selected, view_model.items().count());
    assert_eq!(1000 - selected as i32, list.item_count());
    assert_virtualized(&list, 1000 - selected as i32, "after the removal");
}

/// The page "Chat": a list box over the messages of the chat file, whose items have the
/// heights of their texts.
#[test]
fn the_chat_shows_messages_of_varying_height() {
    let demo = Demo::start();
    demo.select_page(1);
    let view = demo.view::<ChatPageView>();
    let messages = demo.view_model.chat().messages();
    let list = shown::<ListBox>(&view).into_iter().next().expect("the list box of the chat");
    let containers = assert_virtualized(&list, 37, "at the top");
    assert_eq!(0, realized_indexes(&list)[0]);

    // The template of an item shows the sender, the message and the time of the message.
    let first = messages.get(0);
    let shown_texts = texts(&containers[0]);
    for expected in [first.sender(), first.message(), first.timestamp().to_string()] {
        assert!(shown_texts.contains(&expected), "the first item shows '{expected}': {shown_texts:?}");
    }
    // A long message is wrapped, so the items differ in height.
    let mut heights: Vec<i64> = containers.iter().map(|container| container.bounds().height.round() as i64).collect();
    heights.sort_unstable();
    heights.dedup();
    assert!(heights.len() > 1, "the items have the heights of their messages: {heights:?}");
    demo.assert_drawn(&list, "the chat at the top");

    // Scrolling to the end realizes the last message and lets go of the first.
    let scroll_viewer = scroll_viewer_of(&list);
    scroll_viewer.set_offset(Vector::new(0.0, scroll_viewer.extent().height));
    demo.shell.settle();
    scroll_viewer.set_offset(Vector::new(0.0, scroll_viewer.extent().height));
    demo.shell.settle();
    let indexes = realized_indexes(&list);
    assert_eq!(Some(&36), indexes.last(), "the last message is realized: {indexes:?}");
    assert!(indexes[0] > 0, "the first message is no longer realized: {indexes:?}");
    let last = messages.get(36);
    assert!(texts(&list).contains(&last.message()), "the list shows the last message");
    demo.assert_drawn(&list, "the chat at the end");
}

/// The page "Expanders": a list box over a hundred expanders, each bound to its item in both
/// directions.
#[test]
fn the_expanders_follow_their_items() {
    let demo = Demo::start();
    demo.select_page(2);
    let view = demo.view::<ExpanderPageView>();
    let items = demo.view_model.expanders().items();
    let list = shown::<ListBox>(&view).into_iter().next().expect("the list box of the expanders");
    let before = assert_virtualized(&list, 100, "collapsed").len();
    let expander_of = |index: i32| -> Ref<Expander> {
        let container = list.container_from_index(index).unwrap_or_else(|| panic!("the container of the item {index}"));
        shown::<Expander>(&container).into_iter().next().unwrap_or_else(|| panic!("the expander of the item {index}"))
    };

    let first = expander_of(0);
    assert_eq!(Some(String::from("Item 0")), from_markup_value::<String>(&first.header()));
    assert!(!first.is_expanded());
    let collapsed_height = first.bounds().height;
    demo.assert_drawn(&list, "the expanders, collapsed");

    // Pressing the header of the first expander with the mouse expands it and its item.
    let bounds = Shell::bounds_of(&demo.window, &first);
    demo.shell.click(0, ferroui_base::Point::new(bounds.x + bounds.width / 2.0, bounds.y + collapsed_height / 2.0));
    assert!(first.is_expanded(), "the header takes the press");
    assert!(items.get(0).is_expanded(), "the binding wrote to the item");
    assert!(first.bounds().height >= collapsed_height + 300.0, "the expander shows its content of 300: {:?}", first.bounds());

    // Expanding an item expands its expander, and fewer items fit in the view.
    items.get(1).set_is_expanded(true);
    demo.shell.settle();
    assert!(expander_of(1).is_expanded(), "the binding read from the item");
    let after = list.get_realized_containers().len();
    assert!(after < before, "{after} items are realized with two of them expanded, {before} were before");

    // An item that leaves the view and comes back keeps its state: the state is the item's.
    let scroll_viewer = scroll_viewer_of(&list);
    scroll_viewer.set_offset(Vector::new(0.0, scroll_viewer.extent().height / 2.0));
    demo.shell.settle();
    assert!(list.container_from_index(0).is_none(), "the first item left the view");
    assert!(shown::<Expander>(&list).iter().all(|expander| !expander.is_expanded()), "a recycled container shows its new item");
    scroll_viewer.set_offset(Vector::new(0.0, 0.0));
    demo.shell.settle();
    assert!(expander_of(0).is_expanded());
    assert!(expander_of(1).is_expanded());
    assert!(!items.get(2).is_expanded());
}

/// The bindings of the window report no error, on every page.
#[test]
fn the_bindings_of_the_window_report_the_accepted_errors() {
    let demo = Demo::start();
    for index in 0..PAGES.len() {
        demo.select_page(index);
    }
    let found = demo.binding_reports();
    sample_testing::assert_accepted(&found, ACCEPTED);
}

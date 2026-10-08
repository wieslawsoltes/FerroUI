//! Tests of the performance monitor pages of the navigation, content, tabbed, drawer and
//! carousel page galleries.
//!
//! Not ports: the upstream sample has no tests. The pages have no heap figure (there is no
//! garbage-collected heap); what is tested is what they measure: the pages created, the pages
//! that are alive and the operations of their buttons.

use super::support::*;
use crate::pages::{
    CarouselPagePerformancePage, ContentPagePerformancePage, DrawerPagePerformancePage,
    NavigationPagePerformancePage, TabbedPagePerformancePage,
};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{IntoRef, Ref, StyledElement};
use ferroui_controls::{
    Button, CheckBox, ComboBox, ContentPage, Control, DrawerPage, NavigationPage, StackPanel, TextBlock, Window,
};

fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

fn show(control: impl IntoRef<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(1100.0);
    window.set_height(800.0);
    window.set_content(Some(Control::boxed(control)));
    window.show();
    run_jobs();
    window
}

/// The text of the text block `name` of the page.
fn text(page: &Control, name: &str) -> String {
    page.get_control::<TextBlock>(name).text().unwrap_or_default()
}

/// The buttons the document of the page declares.
fn buttons(page: &StyledElement) -> Vec<Ref<Button>> {
    page.get_logical_descendants().filter_map(|element| element.cast::<Button>()).collect()
}

/// Clicks the button of the page with the text `content`.
fn click(page: &StyledElement, content: &str) {
    let button = buttons(page)
        .into_iter()
        .find(|button| {
            button.content().is_some_and(|value| value.downcast_ref::<String>().is_some_and(|text| text == content))
        })
        .unwrap_or_else(|| panic!("no button {content}"));
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    run_jobs();
}

/// The texts of the entries of the log of the page.
fn log(page: &Control) -> Vec<String> {
    let entries = page.get_control::<StackPanel>("LogPanel").children().to_vec();
    entries.into_iter().filter_map(|entry| entry.cast::<TextBlock>()).filter_map(|entry| entry.text()).collect()
}

#[test]
fn the_navigation_page_monitor_counts_the_pages_of_the_stack() {
    let _app = start_catalog_application();
    let page = NavigationPagePerformancePage::new();
    let demo_nav = page.get_control::<NavigationPage>("DemoNav");
    // The clock of the tests does not tick: a push with a transition would not complete.
    demo_nav.set_page_transition(None);
    let window = show(page.clone());

    assert_eq!(1, demo_nav.stack_depth());
    assert_eq!("Stack Depth: 1", text(&page, "StackDepthText"));
    assert_eq!("Live Page Instances: 1", text(&page, "LiveInstancesText"));
    assert_eq!("Total Pages Created: 1", text(&page, "TotalCreatedText"));
    // There is no heap to measure.
    assert_eq!("Managed Heap: not available", text(&page, "ManagedMemoryText"));
    assert_eq!("", text(&page, "MemoryDeltaText"));
    assert_eq!(1, page.get_control::<StackPanel>("StackItemsPanel").children().count());

    click(&page, "Push 5 Pages");
    assert_eq!(6, demo_nav.stack_depth());
    assert_eq!("Stack Depth: 6", text(&page, "StackDepthText"));
    assert_eq!("Live Page Instances: 6", text(&page, "LiveInstancesText"));
    assert_eq!("Total Pages Created: 6", text(&page, "TotalCreatedText"));
    assert_eq!(6, page.get_control::<StackPanel>("StackItemsPanel").children().count());
    let last_op = text(&page, "LastOpTimeText");
    assert!(last_op.starts_with("Last Op: ") && last_op.ends_with(" ms"), "{last_op}");

    click(&page, "Pop Page");
    assert_eq!(5, demo_nav.stack_depth());
    assert_eq!(5, page.get_control::<StackPanel>("StackItemsPanel").children().count());

    // The entries of the log: the action, the detail, the depth and the time; no heap size.
    let entries = log(&page);
    assert_eq!(3, entries.len());
    assert!(entries[0].contains("[Init]  Pushed root page  \u{2014}  depth 1, "), "{}", entries[0]);
    assert!(entries[1].contains("[Push \u{00D7}5]  Pushed pages 2\u{2013}6  \u{2014}  depth 6, "), "{}", entries[1]);
    assert!(entries[2].contains("[Pop]  Popped \"Page 6\"  \u{2014}  depth 5, "), "{}", entries[2]);
    assert!(entries.iter().all(|entry| entry.ends_with(" ms") && !entry.contains("heap")));

    click(&page, "Clear");
    assert!(log(&page).is_empty());

    // The automatic refresh starts and stops with the check box.
    let auto_refresh = page.get_control::<CheckBox>("AutoRefreshCheck");
    auto_refresh.set_is_checked(Some(true));
    auto_refresh.set_is_checked(Some(false));
    auto_refresh.set_is_checked(Some(true));
    window.close();
}

#[test]
fn the_navigation_page_monitor_shows_that_popped_pages_are_freed() {
    let _app = start_catalog_application();
    let page = NavigationPagePerformancePage::new();
    let demo_nav = page.get_control::<NavigationPage>("DemoNav");
    demo_nav.set_page_transition(None);
    let window = show(page.clone());

    click(&page, "Push 5 Pages");
    assert_eq!("Live Page Instances: 6", text(&page, "LiveInstancesText"));
    click(&page, "Pop to Root");
    assert_eq!(1, demo_nav.stack_depth());

    // Nothing is collected: the popped pages are gone once nothing refers to them.
    click(&page, "Force GC + Refresh");
    assert_eq!("Live Page Instances: 1", text(&page, "LiveInstancesText"));
    assert_eq!("Total Pages Created: 6", text(&page, "TotalCreatedText"));
    window.close();
}

#[test]
fn the_content_page_monitor_pushes_pages_of_the_selected_weight() {
    let _app = start_catalog_application();
    let page = ContentPagePerformancePage::new();
    let nav_page = page.get_control::<NavigationPage>("NavPage");
    nav_page.set_page_transition(None);
    let window = show(page.clone());

    // The root page is pushed when the page is loaded.
    assert_eq!(1, nav_page.stack_depth());
    assert_eq!("Stack Depth: 1", text(&page, "StackDepthText"));
    assert_eq!("Total Pages Created: 1", text(&page, "TotalCreatedText"));
    assert_eq!("Managed Heap: not available", text(&page, "ManagedMemoryText"));

    page.get_control::<ComboBox>("WeightCombo").set_selected_index(2);
    click(&page, "Push Page");
    assert_eq!(2, nav_page.stack_depth());
    let pushed = nav_page.current_page().and_then(|page| page.cast::<ContentPage>()).expect("the pushed page");
    let allocation = pushed.tag().expect("the allocation of the page");
    assert_eq!(Some(2_097_152), allocation.downcast_ref::<Vec<u8>>().map(Vec::len));
    let texts: Vec<String> = pushed
        .get_logical_descendants()
        .filter_map(|element| element.cast::<TextBlock>())
        .filter_map(|text| text.text())
        .collect();
    assert!(texts.iter().any(|text| text.contains("Weight: ~2 MB")), "{texts:?}");

    click(&page, "Pop Page");
    assert_eq!(1, nav_page.stack_depth());
    let entries = log(&page);
    assert!(entries.last().is_some_and(|entry| entry.contains("[Pop]  Popped \"Page 2\"")), "{entries:?}");

    click(&page, "Force GC + Refresh");
    assert!(log(&page).last().is_some_and(|entry| entry.contains("[GC]  Refreshed")));
    window.close();
}

#[test]
fn the_tabbed_page_monitor_adds_and_removes_tabs() {
    let _app = start_catalog_application();
    let page = TabbedPagePerformancePage::new();
    let window = show(page.clone());

    // Five tabs are added when the page is loaded.
    assert_eq!("Tab count: 5", text(&page, "TabCountText"));
    assert_eq!("Live instances: 5 / 5 tracked", text(&page, "LiveCountText"));
    assert_eq!("Heap: not available", text(&page, "HeapText"));
    assert_eq!("Total allocated: not available", text(&page, "AllocText"));

    click(&page, "Add 20 Tabs");
    assert_eq!("Tab count: 25", text(&page, "TabCountText"));
    assert_eq!("Live instances: 25 / 25 tracked", text(&page, "LiveCountText"));
    let last_op = text(&page, "LastOpTimeText");
    assert!(last_op.starts_with("Last Op: ") && last_op.ends_with(" ms"), "{last_op}");

    click(&page, "Remove Last 5");
    assert_eq!("Tab count: 20", text(&page, "TabCountText"));
    click(&page, "Clear All");
    assert_eq!("Tab count: 0", text(&page, "TabCountText"));
    window.close();
}

#[test]
fn the_tabbed_page_monitor_shows_that_removed_tabs_are_freed() {
    let _app = start_catalog_application();
    let page = TabbedPagePerformancePage::new();
    let window = show(page.clone());

    click(&page, "Remove Last 5");
    assert_eq!("Tab count: 0", text(&page, "TabCountText"));
    click(&page, "Force GC");
    assert_eq!("Live instances: 0 / 5 tracked", text(&page, "LiveCountText"));
    window.close();
}

#[test]
fn the_carousel_page_monitor_adds_and_removes_pages() {
    let _app = start_catalog_application();
    let page = CarouselPagePerformancePage::new();
    let window = show(page.clone());

    // Five pages are added when the page is loaded.
    assert_eq!("Page count: 5", text(&page, "PageCountText"));
    assert_eq!("Live instances: 5 / 5 tracked", text(&page, "LiveCountText"));
    assert_eq!("Heap: not available", text(&page, "HeapText"));
    assert_eq!("Total allocated: not available", text(&page, "AllocText"));

    click(&page, "Add 5 Pages");
    assert_eq!("Page count: 10", text(&page, "PageCountText"));
    click(&page, "Remove Last 5");
    assert_eq!("Page count: 5", text(&page, "PageCountText"));
    click(&page, "Refresh Stats");
    assert_eq!("Page count: 5", text(&page, "PageCountText"));
    click(&page, "Clear All");
    assert_eq!("Page count: 0", text(&page, "PageCountText"));
    window.close();
}

#[test]
fn the_drawer_page_monitor_swaps_the_detail_page() {
    let _app = start_catalog_application();
    let page = DrawerPagePerformancePage::new();
    let window = show(page.clone());

    // The detail page of the document is tracked when the page is loaded.
    assert_eq!("Current Detail: Home", text(&page, "CurrentDetailText"));
    assert_eq!("Detail Swaps: 0", text(&page, "SwapCountText"));
    assert_eq!("Live Page Instances: 1", text(&page, "LiveInstancesText"));
    assert_eq!("Total Pages Created: 1", text(&page, "TotalCreatedText"));
    assert_eq!("Managed Heap: not available", text(&page, "ManagedMemoryText"));
    assert_eq!(1, page.get_control::<StackPanel>("HistoryPanel").children().count());

    click(&page, "Swap Detail (New Page)");
    assert_eq!("Current Detail: Page 1", text(&page, "CurrentDetailText"));
    assert_eq!("Detail Swaps: 1", text(&page, "SwapCountText"));
    assert_eq!("Total Pages Created: 2", text(&page, "TotalCreatedText"));
    assert_eq!(2, page.get_control::<StackPanel>("HistoryPanel").children().count());

    // The history shows the last ten details.
    click(&page, "Swap 5 Times");
    click(&page, "Swap 5 Times");
    assert_eq!("Current Detail: Page 11", text(&page, "CurrentDetailText"));
    assert_eq!("Detail Swaps: 11", text(&page, "SwapCountText"));
    assert_eq!("Total Pages Created: 12", text(&page, "TotalCreatedText"));
    assert_eq!(10, page.get_control::<StackPanel>("HistoryPanel").children().count());

    // An item of the menu of the drawer swaps to the page of its tag and closes the drawer.
    click(&page, "Toggle Drawer");
    let profile = buttons(&page)
        .into_iter()
        .find(|button| button.tag().is_some_and(|tag| tag.downcast_ref::<String>().is_some_and(|tag| tag == "Profile")))
        .expect("the menu item of the profile");
    profile.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    run_jobs();
    assert_eq!("Current Detail: Profile", text(&page, "CurrentDetailText"));
    assert!(!page.get_control::<DrawerPage>("DrawerPageControl").is_open());

    let entries = log(&page);
    assert!(entries[0].contains("[Init]  Initial detail page: Home  \u{2014} "), "{}", entries[0]);
    assert!(entries.last().is_some_and(|entry| entry.contains("[Swap]  Detail \u{2192} \"Profile\"")));
    window.close();
}

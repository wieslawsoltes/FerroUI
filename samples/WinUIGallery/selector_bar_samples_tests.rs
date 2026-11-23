//! The SelectorBar page: the source's three examples with their code behind.

use crate::navigation_view_samples::page_type;
use crate::selector_bar_samples::{collection, slide};
use crate::{examples, tests::start};
use ferroui_base::Ref;
use ferroui_controls::navigation::NavigationTransitionInfo;
use ferroui_controls::{Control, Frame, ItemsControl, SelectorBar, SelectorBarItem, SymbolIcon};

fn page() -> Ref<Control> {
    examples::create(examples::find("SelectorBar").unwrap()).unwrap_or_else(|error| panic!("SelectorBar: {error}"))
}

#[test]
fn selector_bar_basic_example_has_its_items_and_icons() {
    let _application = start();
    let root = page();
    let bar = root.get_control::<SelectorBar>("SelectorBar1");
    assert_eq!(bar.items().count(), 3);
    assert!(bar.selected_item().is_none());
    let item = root.get_control::<SelectorBarItem>("SelectorBarItemShared");
    assert_eq!(item.text().as_deref(), Some("Shared"));
    assert!(item.icon().and_then(|icon| icon.cast::<SymbolIcon>()).is_some());
}

#[test]
fn selector_bar_navigates_the_frame_with_a_slide_in_its_direction() {
    let _application = start();
    let root = page();
    let bar = root.get_control::<SelectorBar>("SelectorBar2");
    let frame = root.get_control::<Frame>("SelectorContentFrame");
    assert_eq!(bar.selected_index(), 0);
    assert_eq!(frame.current_source_page_type(), page_type("SamplePage1"));

    bar.set_selected_index(2);
    assert_eq!(frame.current_source_page_type(), page_type("SamplePage3"));
    bar.set_selected_index(4);
    assert_eq!(frame.current_source_page_type(), page_type("SamplePage5"));
    assert!(!frame.can_go_back());

    assert_eq!(slide(0, 2), NavigationTransitionInfo::SlideFromRight);
    assert_eq!(slide(2, 1), NavigationTransitionInfo::SlideFromLeft);
    assert_eq!(slide(0, 0), NavigationTransitionInfo::SlideFromLeft);
}

#[test]
fn selector_bar_shows_the_collection_of_the_selected_item() {
    let _application = start();
    let root = page();
    let bar = root.get_control::<SelectorBar>("SelectorBar3");
    let items = root.get_control::<ItemsControl>("ItemsView3");
    assert_eq!(items.items().count(), 5);
    bar.set_selected_index(1);
    assert_eq!(items.items().count(), 7);
    bar.set_selected_index(2);
    assert_eq!(items.items().count(), 4);
    assert_eq!(collection(2).1, 4);
}

//! The SplitView page: the source's example with its code behind (`split_view_samples.rs`).

use crate::split_view_samples::{nav_links, nav_links_list_item_click, NavLink};
use crate::{examples, tests::start};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{
    ComboBox, Control, ListBox, Slider, SplitView, SplitViewDisplayMode, SplitViewPanePlacement, Symbol, TextBlock,
};
use std::rc::Rc;

fn page() -> Ref<Control> {
    examples::create(examples::find("SplitView").unwrap()).unwrap_or_else(|error| panic!("SplitView: {error}"))
}

#[test]
fn split_view_nav_links_write_the_page_of_the_clicked_link() {
    let _application = start();
    let root = page();
    let links: Vec<_> = nav_links().iter().map(|link| (link.label, link.symbol)).collect();
    assert_eq!(
        links,
        [
            ("People", Symbol::People),
            ("Globe", Symbol::Globe),
            ("Message", Symbol::Message),
            ("Mail", Symbol::Mail)
        ]
    );
    let items: Vec<BoxedValue> = root
        .get_control::<ListBox>("NavLinksList")
        .items_source()
        .unwrap()
        .to_vec()
        .into_iter()
        .flatten()
        .collect();
    assert_eq!(items.len(), 4);
    assert!(items[1].downcast_ref::<Rc<NavLink>>().is_some());
    nav_links_list_item_click(&root, &items[1]);
    assert_eq!(
        root.get_control::<TextBlock>("content").text().as_deref(),
        Some("Globe Page")
    );
}

#[test]
fn split_view_options_set_the_pane_placement_mode_background_and_lengths() {
    let _application = start();
    let root = page();
    let split = root.get_control::<SplitView>("splitView");
    let list = root.get_control::<ListBox>("NavLinksList");
    // The markup: CompactOverlay until the box's first selection changes it.
    assert_eq!(split.display_mode(), SplitViewDisplayMode::CompactOverlay);
    assert!(split.is_pane_open());
    // The TwoWay x:Bind of IsPaneOpen.
    let toggle = root.get_control::<ToggleButton>("togglePaneButton");
    toggle.set_is_checked(Some(false));
    assert!(!split.is_pane_open());
    split.set_is_pane_open(true);
    assert_eq!(toggle.is_checked(), Some(true));
    // PanePlacement_Toggled and UpdateNavLinkItemLayout.
    assert!(!list.classes().contains("RightIconLayout"));
    root.get_control::<ToggleButton>("PanePlacement")
        .set_is_checked(Some(true));
    assert_eq!(split.pane_placement(), SplitViewPanePlacement::Right);
    assert!(list.classes().contains("RightIconLayout"));
    root.get_control::<ToggleButton>("PanePlacement")
        .set_is_checked(Some(false));
    assert_eq!(split.pane_placement(), SplitViewPanePlacement::Left);
    assert!(!list.classes().contains("RightIconLayout"));
    // displayModeCombobox_SelectionChanged.
    let modes = root.get_control::<ComboBox>("displayModeCombobox");
    modes.set_selected_index(2);
    assert_eq!(split.display_mode(), SplitViewDisplayMode::Overlay);
    modes.set_selected_index(1);
    assert_eq!(split.display_mode(), SplitViewDisplayMode::CompactInline);
    // paneBackgroundCombobox_SelectionChanged: the state of the selected name.
    let backgrounds = root.get_control::<ComboBox>("paneBackgroundCombobox");
    backgrounds.set_selected_index(2);
    assert!(split.classes().contains("Blue") && !split.classes().contains("Red"));
    backgrounds.set_selected_index(0);
    assert!(!split.classes().contains("Blue"));
    // The OneWay x:Bind of the pane lengths.
    root.get_control::<Slider>("openPaneLengthSlider")
        .set_range_value(320.0);
    assert_eq!(split.open_pane_length(), 320.0);
    root.get_control::<Slider>("compactPaneLengthSlider")
        .set_range_value(64.0);
    assert_eq!(split.compact_pane_length(), 64.0);
}

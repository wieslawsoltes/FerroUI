//! Tests of the SemanticZoom page (semantic_zoom_samples.rs).

use crate::semantic_zoom_samples::{groups, ControlInfoDataGroup, ControlInfoDataItem};
use crate::{examples, tests::start};
use ferroui_base::Ref;
use ferroui_controls::{CollectionViewGroup, Control, GridView, ListView, SemanticZoom};
use std::rc::Rc;

fn page() -> Ref<Control> {
    let sample = examples::find("SemanticZoom").expect("the SemanticZoom page");
    examples::create(sample).expect("the page loads")
}

#[test]
fn semantic_zoom_shows_the_controls_grouped_and_the_list_of_the_groups() {
    let _application = start();
    let root = page();
    let zoom = root.get_control::<SemanticZoom>("Control1");
    let grid = root.get_control::<GridView>("ZoomedInGridView");
    let list = root.get_control::<ListView>("ZoomedOutListView");
    assert_eq!(zoom.height(), 500.0);
    assert!(zoom.zoomed_in_view().is_some_and(|view| view == grid.clone().upcast::<Control>()));
    assert!(zoom.zoomed_out_view().is_some_and(|view| view == list.clone().upcast::<Control>()));
    assert!(grid.is_grouping(), "the grid shows the View of cvsGroups");

    let expected = groups();
    assert!(expected.len() > 3);
    let zoomed_out: Vec<Rc<CollectionViewGroup>> = list
        .items_source()
        .expect("View.CollectionGroups")
        .to_vec()
        .into_iter()
        .flatten()
        .filter_map(|item| item.downcast_ref::<Rc<CollectionViewGroup>>().cloned())
        .collect();
    assert_eq!(zoomed_out.len(), expected.len());
    for (group, data) in zoomed_out.iter().zip(&expected) {
        let header = group.group().and_then(|header| header.downcast_ref::<Rc<ControlInfoDataGroup>>().cloned());
        assert_eq!(header.expect("the group of the source").title, data.title);
        assert_eq!(group.group_items().count(), data.items.count(), "ItemsPath=\"Items\" of {}", data.title);
    }
    let first = grid.items_view().get_at(0).expect("an item");
    let first = first.downcast_ref::<Rc<ControlInfoDataItem>>().expect("a control of the gallery");
    assert!(!first.title.is_empty());

    // The zoomed-out view of the groups, and back.
    zoom.toggle_active_view();
    assert!(!zoom.is_zoomed_in_view_active());
    zoom.toggle_active_view();
    assert!(zoom.is_zoomed_in_view_active());
}

/// In a window the templates show the titles: the controls and the headers of the groups in the
/// grid, the group of each entry of the list (`Group.Title` of `CollectionViewGroup`).
#[cfg(feature = "theme")]
#[test]
fn semantic_zoom_templates_show_the_titles_of_the_controls_and_the_groups() {
    use ferroui_controls::{TextBlock, Window};
    let _application = crate::tests::start_winui();
    let root = page();
    let zoom = root.get_control::<SemanticZoom>("Control1");
    let list = root.get_control::<ListView>("ZoomedOutListView");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    let texts = |control: &Ref<Control>| -> Vec<String> {
        control
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<TextBlock>())
            .filter_map(|text| text.text())
            .filter(|text| !text.is_empty())
            .collect()
    };
    let expected = groups();
    let grid_texts = texts(&zoom.zoomed_in_view().unwrap());
    assert!(grid_texts.contains(&expected[0].title), "the header of the first group: {grid_texts:?}");
    zoom.toggle_active_view();
    window.update_layout();
    let list_texts = texts(&list.clone().upcast());
    assert!(list_texts.contains(&expected[0].title), "the first group of the list: {list_texts:?}");
}

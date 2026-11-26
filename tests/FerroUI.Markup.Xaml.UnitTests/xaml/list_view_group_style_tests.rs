//! The groups of a list view in XAML (#1258): `ListView.GroupStyle` takes `GroupStyle` elements;
//! the members of #1424 (`ItemsPath`, `ItemsUpdatingScrollMode`, the check boxes of `Multiple`).

use crate::support::app::xaml_test_base;
use crate::support::loader::load_as;
use ferroui_base::Ref;
use ferroui_controls::{
    CollectionViewSource, ItemsStackPanel, ItemsUpdatingScrollMode, ListView, ListViewSelectionMode,
};

#[test]
fn a_list_view_takes_its_group_styles_from_xaml() {
    let _app = xaml_test_base();
    let list = load_as::<Ref<ListView>>(
        r#"<ListView xmlns='https://github.com/ferroui'>
             <ListView.GroupStyle>
               <GroupStyle HidesIfEmpty='True' />
             </ListView.GroupStyle>
           </ListView>"#,
    );
    let styles = list.group_style();
    assert_eq!(styles.count(), 1);
    assert!(styles.get(0).hides_if_empty());
}

/// `CollectionViewSource.ItemsPath` from XAML (#1424).
#[test]
fn a_collection_view_source_takes_its_items_path_from_xaml() {
    let _app = xaml_test_base();
    let source = load_as::<Ref<CollectionViewSource>>(
        r#"<CollectionViewSource xmlns='https://github.com/ferroui' IsSourceGrouped='True' ItemsPath='Items' />"#,
    );
    assert!(source.is_source_grouped());
    assert_eq!(source.items_path(), "Items");
}

/// `ItemsStackPanel.ItemsUpdatingScrollMode` and the check boxes of `Multiple` from XAML (#1424).
#[test]
fn a_list_view_takes_its_scroll_mode_and_check_boxes_from_xaml() {
    let _app = xaml_test_base();
    let panel = load_as::<Ref<ItemsStackPanel>>(
        r#"<ItemsStackPanel xmlns='https://github.com/ferroui' ItemsUpdatingScrollMode='KeepLastItemInView' />"#,
    );
    assert_eq!(panel.items_updating_scroll_mode(), ItemsUpdatingScrollMode::KeepLastItemInView);
    let list = load_as::<Ref<ListView>>(
        r#"<ListView xmlns='https://github.com/ferroui' SelectionMode='Multiple' IsMultiSelectCheckBoxEnabled='False' />"#,
    );
    assert_eq!(list.selection_mode(), ListViewSelectionMode::Multiple);
    assert!(!list.is_multi_select_check_box_enabled());
}

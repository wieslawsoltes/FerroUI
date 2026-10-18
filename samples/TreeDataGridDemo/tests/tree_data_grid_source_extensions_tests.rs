//! Port of `TreeDataGridDemo.Tests/TreeDataGridSourceExtensionsTests.cs`: the displayed row of a
//! model follows the filter and the sort of the source, and models are found by identity.

use crate::internal_tree_data_grid_source_extensions::TreeDataGridSourceExtensions;
use ferroui_base::BoxedValue;
use ferroui_controls_treedatagrid::TreeDataGridSourceExtensions as _;
use ferroui_treedatagrid_core::models::{IColumn, ListSortDirection};
use ferroui_treedatagrid_core::{Accessor, FlatTreeDataGridSource, ITreeDataGridSource, Models};
use std::rc::Rc;

/// `private sealed record Item(string Name)`: two items with the same name are equal, as two
/// records with the same values are, and still two objects.
#[derive(Debug, PartialEq)]
struct Item {
    name: String,
}

impl Item {
    fn new(name: &str) -> Rc<Self> {
        Rc::new(Self { name: name.to_string() })
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

/// The item as an untyped value (`object`).
fn boxed(item: &Rc<Item>) -> BoxedValue {
    item.clone() as BoxedValue
}

#[test]
fn find_displayed_row_index_uses_reference_identity_and_handles_null() {
    let displayed = Item::new("Same value");
    let equal_but_different = Item::new("Same value");
    let source = FlatTreeDataGridSource::new(Rc::new(vec![displayed.clone()]) as Models<Item>);

    assert_eq!(*displayed, *equal_but_different);
    assert_eq!(0, source.find_displayed_row_index(Some(&boxed(&displayed))));
    assert_eq!(-1, source.find_displayed_row_index(Some(&boxed(&equal_but_different))));
    assert_eq!(-1, source.find_displayed_row_index(None));
}

#[test]
fn find_displayed_row_index_tracks_filtering_and_sorting() {
    let gamma = Item::new("Gamma");
    let alpha = Item::new("Alpha");
    let beta = Item::new("Beta");
    let source = FlatTreeDataGridSource::new(Rc::new(vec![gamma.clone(), alpha.clone(), beta.clone()]) as Models<Item>)
        .with_text_column(Some(Rc::new("Name".to_string())), Accessor::member("Name", |x: &Rc<Item>| x.name()), |_| {});

    assert_eq!(1, source.find_displayed_row_index(Some(&boxed(&alpha))));

    let filtered_out = gamma.clone();
    source.set_filter(Some(Rc::new(move |x: &Rc<Item>| !Rc::ptr_eq(x, &filtered_out))));

    assert_eq!(-1, source.find_displayed_row_index(Some(&boxed(&gamma))));
    assert_eq!(0, source.find_displayed_row_index(Some(&boxed(&alpha))));
    assert_eq!(1, source.find_displayed_row_index(Some(&boxed(&beta))));

    let column = source.columns().get(0);
    let untyped: &dyn IColumn = &*column;
    ITreeDataGridSource::sort_by(&*source, Some(untyped), ListSortDirection::Descending);

    assert_eq!(0, source.find_displayed_row_index(Some(&boxed(&beta))));
    assert_eq!(1, source.find_displayed_row_index(Some(&boxed(&alpha))));
}

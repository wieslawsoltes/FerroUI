//! Port of `ViewModels/DragDropPageViewModel.cs`.

use crate::models::DragDropItem;
use ferroui_base::collections::FerroList;
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_controls::GridLength;
use ferroui_controls_treedatagrid::models::{CheckBoxColumn, HierarchicalExpanderColumn, IViewColumn, TextColumn};
use ferroui_controls_treedatagrid::TreeDataGridSourceRef;
use ferroui_treedatagrid_core::models::ColumnRef;
use ferroui_treedatagrid_core::{Accessor, HierarchicalTreeDataGridSource, Models};
use std::rc::Rc;

/// The drag and drop page: a hierarchical source of random items whose rows can be dragged and
/// dropped, with check boxes that allow or forbid the drag of a row and the drop into it.
pub struct DragDropPageViewModel {
    // The items of the source, which the drops of rows rearrange.
    #[allow(dead_code)]
    data: Rc<FerroList<Rc<DragDropItem>>>,
    source: Rc<HierarchicalTreeDataGridSource<DragDropItem>>,
}

impl PartialEq for DragDropPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

impl DragDropPageViewModel {
    pub fn new() -> Rc<Self> {
        let data = DragDropItem::create_random_items();
        let source = HierarchicalTreeDataGridSource::new(data.clone() as Models<DragDropItem>);
        let columns = source.columns();
        columns.add(HierarchicalExpanderColumn::new(
            TextColumn::new(
                header("Name"),
                Accessor::member("Name", |x: &Rc<DragDropItem>| x.name()),
                Some(GridLength::STAR),
                None,
            ) as Rc<dyn IViewColumn<DragDropItem>>,
            |x: &Rc<DragDropItem>| Some(x.children() as Models<DragDropItem>),
            None,
            None,
        ) as ColumnRef<DragDropItem>);
        columns.add(CheckBoxColumn::new(
            header("Allow Drag"),
            Accessor::member("AllowDrag", |x: &Rc<DragDropItem>| x.allow_drag())
                .with_setter(|o: &Rc<DragDropItem>, x: bool| o.set_allow_drag(x)),
            None,
            None,
        ) as ColumnRef<DragDropItem>);
        columns.add(CheckBoxColumn::new(
            header("Allow Drop"),
            Accessor::member("AllowDrop", |x: &Rc<DragDropItem>| x.allow_drop())
                .with_setter(|o: &Rc<DragDropItem>, x: bool| o.set_allow_drop(x)),
            None,
            None,
        ) as ColumnRef<DragDropItem>);

        source.row_selection().expect("the row selection of a new source").base().set_single_select(false);
        Rc::new(Self { data, source })
    }

    pub fn source(&self) -> Rc<HierarchicalTreeDataGridSource<DragDropItem>> {
        self.source.clone()
    }
}

ferro_markup_type!(class DragDropPageViewModel {
    this: Rc<DragDropPageViewModel>,
    handles: [DragDropPageViewModel, Rc<DragDropPageViewModel>, Option<Rc<DragDropPageViewModel>>],
    properties: [
        Source: TreeDataGridSourceRef {
            get: |this: &Rc<DragDropPageViewModel>| TreeDataGridSourceRef::of::<DragDropItem, _>(this.source())
        },
    ],
});

#[cfg(test)]
mod tests {
    // Not ports: the reference has no tests of this page.
    use super::*;
    use ferroui_treedatagrid_core::ITreeDataGridSource;

    #[test]
    fn the_source_has_the_items_and_the_three_columns() {
        let target = DragDropPageViewModel::new();

        assert_eq!(target.data.count(), target.source().rows().count());
        assert_eq!(3, target.source().columns().count());
        assert!(!target.source().row_selection().unwrap().base().single_select());
    }
}

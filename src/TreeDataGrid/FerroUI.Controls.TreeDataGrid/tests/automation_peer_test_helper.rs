//! `AutomationPeerTestHelper.cs`: the helpers of the automation peer tests: the creation of a peer
//! of an exact type and a grid over a flat source of `FlatRowModel`s in a test window.
//!
//! The reference's classic `FlatTreeDataGridSource<FlatRowModel>` with classic text columns is the
//! core's flat source holding the control's view text columns; its notifying list of items is a
//! notifying list of the framework.

#![allow(dead_code)]

use super::support::{observable, run_jobs, source_ref, test_window, tree_data_grid_row_template, tree_data_grid_template};
use crate::models::TextColumn;
use crate::primitives::{TreeDataGridCell, TreeDataGridRow};
use crate::TreeDataGrid;
use ferroui_base::collections::FerroList;
use ferroui_base::layout::Layoutable;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{BoxedValue, ObjectType, Ref, StaticType};
use ferroui_controls::automation::peers::ControlAutomationPeer;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{Control, Window};
use ferroui_treedatagrid_core::models::ColumnRef as CoreColumnRef;
use ferroui_treedatagrid_core::selection::{ITreeDataGridSelection, TreeDataGridCellSelectionModel};
use ferroui_treedatagrid_core::{Accessor, FlatTreeDataGridSource, ITreeDataGridSource, ITreeDataGridSourceOf, Models};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `CreatePeer<TPeer>(control)`: the peer of `control`, which must be exactly of class `P`
/// (`Assert.IsType<TPeer>`).
pub(super) fn create_peer<P: ObjectType>(control: &Control) -> Ref<P> {
    let peer = ControlAutomationPeer::create_peer_for_element(control);
    assert!(
        std::ptr::eq(peer.get_type(), <P as StaticType>::TYPE),
        "expected a peer of type {}",
        std::any::type_name::<P>()
    );
    peer.cast::<P>().expect("a peer of the expected type")
}

/// The optional arguments of [`create_flat_target`] (the reference's default parameter values).
#[derive(Clone, Copy, Debug)]
pub(super) struct FlatTargetOptions {
    pub single_select: bool,
    pub item_count: usize,
    pub use_cell_selection: bool,
    pub run_layout: bool,
}

impl Default for FlatTargetOptions {
    fn default() -> Self {
        Self { single_select: true, item_count: 20, use_cell_selection: false, run_layout: true }
    }
}

/// The grid, its source, the items of the source and the window of [`create_flat_target`].
pub(super) type FlatTarget =
    (Ref<TreeDataGrid>, Rc<FlatTreeDataGridSource<FlatRowModel>>, Rc<FerroList<Rc<FlatRowModel>>>, Ref<Window>);

/// `CreateFlatTarget(singleSelect, itemCount, useCellSelection, runLayout)`: a grid over a flat
/// source of `item_count` models with an "ID" and a "Title" column, in a test window whose rows
/// have the test row template and whose cells are 10 pixels high.
pub(super) fn create_flat_target(options: FlatTargetOptions) -> FlatTarget {
    let items = observable((0..options.item_count).map(|x| FlatRowModel::new(x as i32, Some(format!("Item {x}")))));

    let source = FlatTreeDataGridSource::new(items.clone() as Models<FlatRowModel>);
    source.row_selection().expect("the row selection of the source").base().set_single_select(options.single_select);
    source.columns().add(TextColumn::<FlatRowModel, i32>::new(
        Some(header("ID")),
        Accessor::member("Id", |x: &Rc<FlatRowModel>| x.id()),
        None,
        None,
    ) as CoreColumnRef<FlatRowModel>);
    source.columns().add(TextColumn::<FlatRowModel, Option<String>>::new(
        Some(header("Title")),
        Accessor::member("Title", |x: &Rc<FlatRowModel>| x.title()).with_setter(|o: &Rc<FlatRowModel>, v| o.set_title(v)),
        None,
        None,
    ) as CoreColumnRef<FlatRowModel>);

    if options.use_cell_selection {
        let typed = source.clone() as Rc<dyn ITreeDataGridSourceOf<FlatRowModel>>;
        let selection = TreeDataGridCellSelectionModel::new(&typed);
        source.set_selection(Some(selection as Rc<dyn ITreeDataGridSelection>));
    }

    let target = TreeDataGrid::new();
    target.set_template(Some(tree_data_grid_template(0.0)));
    target.set_source(source_ref::<FlatRowModel, _>(&source));

    let root = test_window(Some(target.clone().upcast()), None);
    root.styles().add(Style::with_setters(
        Selectors::is::<TreeDataGridRow>(),
        [Setter::new(TemplatedControl::template_property(), Some(tree_data_grid_row_template()))],
    ));
    root.styles().add(Style::with_setters(
        Selectors::is::<TreeDataGridCell>(),
        [Setter::new(Layoutable::height_property(), 10.0)],
    ));

    if options.run_layout {
        root.update_layout();
        run_jobs();
    }

    (target, source, items, root)
}

fn header(text: &str) -> BoxedValue {
    Rc::new(text.to_string()) as BoxedValue
}

/// `FlatRowModel`: a model with an `Id` and a `Title`.
pub(super) struct FlatRowModel {
    id: Cell<i32>,
    title: RefCell<Option<String>>,
}

impl PartialEq for FlatRowModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl FlatRowModel {
    pub fn new(id: i32, title: Option<String>) -> Rc<Self> {
        Rc::new(Self { id: Cell::new(id), title: RefCell::new(title) })
    }

    pub fn id(&self) -> i32 {
        self.id.get()
    }

    pub fn set_id(&self, value: i32) {
        self.id.set(value);
    }

    pub fn title(&self) -> Option<String> {
        self.title.borrow().clone()
    }

    pub fn set_title(&self, value: Option<String>) {
        *self.title.borrow_mut() = value;
    }
}

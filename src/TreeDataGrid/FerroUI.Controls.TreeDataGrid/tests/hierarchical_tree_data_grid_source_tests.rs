//! `HierarchicalTreeDataGridSourceTests.cs`: the hierarchical source with the control's expander
//! and text columns: its rows and cells, expansion (by the source, by the rows and bound to the
//! models), the expander column rules, the visibility of the expanders, its selection, the
//! reassignment of its items, filtering and the row events.
//!
//! The reference tests its classic `HierarchicalTreeDataGridSource<Node>` with the classic
//! columns; the port has one data layer, so the source is the core's
//! `HierarchicalTreeDataGridSource` holding the control's view columns
//! (`crate::models::HierarchicalExpanderColumn`, `crate::models::TextColumn`). Cells the reference
//! realizes with `target.Rows.RealizeCell(...)` are realized by the rows of the presentation of the
//! source (`TreeDataGridSourcePresentation`), and the reference's debug list of nodes (which
//! counts its subscribers) is the tests' `ListDebug`. A `[Theory]` over `sorted` runs its body
//! for both values.

use super::support::{assert_panics, presentation_of, same_model, start_app, ListDebug, TestNotifier};
use crate::models::{
    CellRef, ExpanderCell, FilteredExpanderColumn, HierarchicalExpanderColumn, IViewColumn, IViewExpanderColumn,
    ModelFilter, TextColumn,
};
use crate::presentation::{TreeDataGridPresentation, TreeDataGridSourcePresentation};
use crate::{TreeDataGridRowModel, TreeDataGridRowModelEventArgs};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::data::core::{Maybe, Value};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::{ferro_model, BoxedValue};
use ferroui_treedatagrid_core::models::{
    as_hierarchical_row, ColumnRef, Comparison, HierarchicalRow, IRows, RowEventArgs, RowRef, RowsChangedArgs,
};
use ferroui_treedatagrid_core::selection::{
    downcast_models, ITreeDataGridSelection, TreeSelectionModelSelectionChangedEventArgsOf,
};
use ferroui_treedatagrid_core::{
    index_path, Accessor, HierarchicalTreeDataGridSource, ITreeDataGridSource, IndexPath, Models, RowEventHandler,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn selection_can_be_explicitly_disabled() {
    let target = HierarchicalTreeDataGridSource::<Node>::new(Rc::new(Vec::<Rc<Node>>::new()) as Models<Node>);

    target.set_selection(None);

    assert!(target.selection().is_none());
}

/// `HierarchicalTreeDataGridSourceTests.RowsAndCells`.
mod rows_and_cells {
    use super::*;

    #[test]
    fn creates_cells_for_root_models() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);

            assert_state(&target, &data, 5, sorted, &[]);
        }
    }

    #[test]
    fn expanding_root_node_creates_child_cells() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);

            target.expand(&IndexPath::new(0));

            assert_state(&target, &data, 10, sorted, &[IndexPath::new(0)]);
        }
    }

    #[test]
    fn collapsing_root_node_removes_child_cells() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);

            target.expand(&IndexPath::new(0));

            assert_eq!(10, target.rows().count());

            target.collapse(&IndexPath::new(0));

            assert_state(&target, &data, 5, sorted, &[]);
        }
    }

    #[test]
    fn supports_adding_root_row() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);

            assert_eq!(5, target.rows().count());

            let _raised = count_rows_changes(&*target.rows());

            data.list().add(Node::create(100, Some("New Node 1"), None));

            assert_state(&target, &data, 6, sorted, &[]);
        }
    }

    #[test]
    fn supports_inserting_root_row() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);

            assert_eq!(5, target.rows().count());

            let _raised = count_rows_changes(&*target.rows());

            data.list().insert(1, Node::create(100, Some("New Node 1"), None));

            assert_state(&target, &data, 6, sorted, &[]);
        }
    }

    #[test]
    fn supports_removing_root_row() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);

            assert_eq!(5, target.rows().count());

            let _raised = count_rows_changes(&*target.rows());

            data.list().remove_at(1);

            assert_state(&target, &data, 4, sorted, &[]);
        }
    }

    #[test]
    fn supports_removing_root_row_with_earlier_row_expanded_to_grandchildren() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            child_at(&data, &index_path![0, 0])
                .set_children(Some(ListDebug::from_items([Node::create(100, Some("Node 0-0-0"), None)])));

            let target = create_target(data.models(), sorted, false);

            target.expand(&IndexPath::new(0));
            target.expand(&index_path![0, 0]);

            assert_eq!(11, target.rows().count());

            let _raised = count_rows_changes(&*target.rows());

            data.list().remove_at(1);

            assert_state(&target, &data, 10, sorted, &[IndexPath::new(0), index_path![0, 0]]);
        }
    }

    #[test]
    fn supports_removing_root_row_with_later_row_expanded() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);

            let target = create_target(data.models(), sorted, false);

            target.expand(&IndexPath::new(4));

            assert_eq!(10, target.rows().count());

            let _raised = count_rows_changes(&*target.rows());

            data.list().remove_at(1);

            assert_state(&target, &data, 9, sorted, &[IndexPath::new(3)]);
        }
    }

    #[test]
    fn removing_expanded_root_row_unsubscribes_from_collection_changed() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);
            let to_remove = data.list().get(1);

            target.expand(&IndexPath::new(1));
            assert_eq!(1, to_remove.children().expect("children").collection_changed_subscriber_count());

            data.list().remove_at(1);
            assert_eq!(0, to_remove.children().expect("children").collection_changed_subscriber_count());
        }
    }

    #[test]
    fn removing_expanded_root_row_with_expanded_child_unsubscribes_from_collection_changed() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);
            let to_remove = child_at(&data, &index_path![1, 1]);

            to_remove.set_children(Some(ListDebug::from_items([Node::new()])));

            target.expand(&index_path![1, 1]);
            assert_eq!(1, to_remove.children().expect("children").collection_changed_subscriber_count());

            data.list().remove_at(1);
            assert_eq!(0, to_remove.children().expect("children").collection_changed_subscriber_count());
        }
    }

    #[test]
    fn supports_adding_child_row() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);

            target.expand(&IndexPath::new(0));

            assert_eq!(10, target.rows().count());

            let _raised = count_rows_changes(&*target.rows());

            children_of(&data.list().get(0)).list().add(Node::create(100, Some("New Node 1"), None));

            assert_state(&target, &data, 11, sorted, &[IndexPath::new(0)]);
        }
    }

    #[test]
    fn supports_adding_child_to_expanded_then_unexpanded_root_node() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);

            target.expand(&IndexPath::new(0));
            target.collapse(&IndexPath::new(0));

            children_of(&data.list().get(0)).list().add(Node::create(100, Some("New Node 1"), None));

            assert_state(&target, &data, 5, sorted, &[]);
        }
    }

    #[test]
    fn supports_inserting_child_row() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);

            target.expand(&IndexPath::new(0));

            assert_eq!(10, target.rows().count());

            let _raised = count_rows_changes(&*target.rows());

            children_of(&data.list().get(0)).list().insert(1, Node::create(100, Some("New Node 1"), None));

            assert_state(&target, &data, 11, sorted, &[IndexPath::new(0)]);
        }
    }

    #[test]
    fn supports_removing_child_row() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);

            target.expand(&IndexPath::new(0));
            assert_eq!(10, target.rows().count());

            let _raised = count_rows_changes(&*target.rows());

            children_of(&data.list().get(0)).list().remove_at(3);

            assert_state(&target, &data, 9, sorted, &[IndexPath::new(0)]);
        }
    }

    #[test]
    fn supports_removing_child_rows_at_start() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);

            target.expand(&IndexPath::new(0));
            assert_eq!(10, target.rows().count());

            let _raised = count_rows_changes(&*target.rows());

            children_of(&data.list().get(0)).list().remove_range(0, 2);

            assert_state(&target, &data, 8, sorted, &[IndexPath::new(0)]);
        }
    }

    #[test]
    fn supports_replacing_root_row() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);

            assert_eq!(5, target.rows().count());

            let _raised = count_rows_changes(&*target.rows());

            data.list().set(2, Node::create(100, Some("Replaced"), None));

            assert_state(&target, &data, 5, sorted, &[]);
        }
    }

    #[test]
    fn supports_moving_root_row() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);

            assert_eq!(5, target.rows().count());

            let _raised = count_rows_changes(&*target.rows());

            data.list().move_item(2, 4);

            assert_state(&target, &data, 5, sorted, &[]);
        }
    }

    #[test]
    fn setting_sort_updates_rows() {
        let _app = start_app();
        let data = create_data(5, 5);
        let target = create_target(data.models(), false, false);

        target.expand(&IndexPath::new(0));

        assert_eq!(10, target.rows().count());

        target.sort(Some(by_descending_id()));

        assert_state(&target, &data, 10, true, &[IndexPath::new(0)]);
    }

    #[test]
    fn clearing_sort_updates_rows() {
        let _app = start_app();
        let data = create_data(5, 5);
        let target = create_target(data.models(), true, false);

        target.expand(&IndexPath::new(0));

        assert_eq!(10, target.rows().count());

        target.sort(None);

        assert_state(&target, &data, 10, false, &[IndexPath::new(0)]);
    }
}

/// `HierarchicalTreeDataGridSourceTests.Expansion`.
mod expansion {
    use super::*;

    #[test]
    fn expanding_updates_cell_is_expanded() {
        let _app = start_app();
        let data = create_data(5, 5);
        let target = create_target(data.models(), false, false);
        let (_presentation, cell) = realize_cell(&target, 0, 0);
        let expander = expander_cell(&cell);
        let raised = counter();

        {
            let raised = raised.clone();
            expander.property_changed().add(Rc::new(move |name: &str| {
                if name == "IsExpanded" {
                    bump(&raised);
                }
            }));
        }

        target.expand(&IndexPath::new(0));

        assert!(expander.is_expanded());
        assert_eq!(1, raised.get());
    }

    #[test]
    fn expanding_previously_expanded_node_creates_expanded_descendent() {
        let _app = start_app();
        let data = create_data(5, 5);
        let target = create_target(data.models(), false, false);

        child_at(&data, &index_path![0, 0])
            .set_children(Some(ListDebug::from_items([Node::create(100, Some("Grandchild"), None)])));

        // Expand first root node.
        target.expand(&IndexPath::new(0));

        assert_state(&target, &data, 10, false, &[IndexPath::new(0)]);

        // Expand first child node.
        target.expand(&index_path![0, 0]);

        // Grandchild should now be visible.
        assert_state(&target, &data, 11, false, &[IndexPath::new(0), index_path![0, 0]]);

        // Collapse root node.
        target.collapse(&IndexPath::new(0));
        assert_state(&target, &data, 5, false, &[]);

        // And expand again. Grandchild should now be visible once more.
        target.expand(&IndexPath::new(0));
        assert_state(&target, &data, 11, false, &[IndexPath::new(0), index_path![0, 0]]);
    }

    #[test]
    fn shows_expander_for_row_with_children() {
        let _app = start_app();
        let data = create_data(5, 5);
        let target = create_target(data.models(), false, false);
        let (_presentation, cell) = realize_cell(&target, 0, 0);
        let expander = expander_cell(&cell);

        assert!(expander.show_expander());
    }

    #[test]
    fn hides_expander_for_row_without_children() {
        let _app = start_app();
        let data: Models<Node> = Rc::new(vec![Node::create(0, Some("Node 0"), None)]);
        let target = create_target(data, false, false);
        let (_presentation, cell) = realize_cell(&target, 0, 0);
        let expander = expander_cell(&cell);

        assert!(!expander.show_expander());
    }

    #[test]
    fn attempting_to_expand_node_that_has_no_children_hides_expander() {
        let _app = start_app();
        let data = Node::create(0, Some("Node 0"), None);

        // Here we return true from hasChildren selector, but there are actually no children.
        // This may happen if calculating the children is expensive.
        let target = HierarchicalTreeDataGridSource::from_item(data)
            .with_column(HierarchicalExpanderColumn::new(
                id_column(),
                node_children,
                Some(Accessor::new(|_: &Rc<Node>| true)),
                None,
            ) as ColumnRef<Node>)
            .with_column(caption_column());

        let (_presentation, cell) = realize_cell(&target, 0, 0);
        let expander = cell.as_expander_cell().expect("an expander cell");

        target.expand(&IndexPath::new(0));

        assert!(!expander.show_expander());
        assert!(!expander.is_expanded());
    }

    #[test]
    fn expand_all_expands_all_rows() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data_levels(&[5, 3, 3]);
            let target = create_target(data.models(), sorted, false);

            target.expand_all();

            assert_eq!(65, target.rows().count());
        }
    }

    #[test]
    fn collapse_all_collapses_all_rows() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data_levels(&[5, 3, 3]);
            let target = create_target(data.models(), sorted, false);

            // We need to expand before we can collapse.
            target.expand_all();
            assert_eq!(65, target.rows().count());

            // Now we can test collapsing.
            target.collapse_all();
            assert_eq!(5, target.rows().count());

            // Ensure that nested rows were collapsed, i.e. only the first level of rows is
            // visible after expanding now.
            target.expand(&IndexPath::new(0));
            assert_eq!(8, target.rows().count());
        }
    }
}

#[test]
fn adding_second_expander_column_throws() {
    let _app = start_app();
    let data = create_data(5, 5);
    let target = create_target(data.models(), false, false);

    assert_panics(|| {
        target.columns().add(
            HierarchicalExpanderColumn::new(id_column(), node_children, None, Some(is_expanded_accessor())) as ColumnRef<Node>,
        );
    });
}

#[test]
fn removing_expander_column_throws() {
    let _app = start_app();
    let data = create_data(5, 5);
    let target = create_target(data.models(), false, false);

    // `target.Columns.OfType<IExpanderColumn<Node>>().First()`.
    let expander = target
        .columns()
        .to_vec()
        .into_iter()
        .find(|column| column.as_expander_column_any())
        .expect("an expander column");

    assert_panics(|| {
        target.columns().remove(&expander);
    });
}

/// `HierarchicalTreeDataGridSourceTests.ExpansionBinding`: the expansion bound to `Node.IsExpanded`.
mod expansion_binding {
    use super::*;

    #[test]
    fn root_is_initially_expanded() {
        let _app = start_app();
        let data = create_data(5, 5);
        data.list().get(0).set_is_expanded(true);

        let target = create_target(data.models(), false, true);
        let realizer = CellRealizer::new(&target);
        realizer.realize_cells();

        assert_state(&target, &data, 10, false, &[IndexPath::new(0)]);
    }

    #[test]
    fn child_is_initially_expanded() {
        let _app = start_app();
        let data = create_data(5, 5);
        data.list().get(0).set_is_expanded(true);
        child_at(&data, &index_path![0, 1]).set_is_expanded(true);
        children_of(&child_at(&data, &index_path![0, 1])).list().add(Node::new());

        let target = create_target(data.models(), false, true);
        let realizer = CellRealizer::new(&target);
        realizer.realize_cells();

        assert_state(&target, &data, 11, false, &[IndexPath::new(0), index_path![0, 1]]);
    }

    #[test]
    fn handles_initial_expanded_row_with_no_children() {
        let _app = start_app();
        let data = create_data(5, 5);
        data.list().get(0).set_is_expanded(true);

        // This node has no children.
        child_at(&data, &index_path![0, 1]).set_is_expanded(true);

        let target = create_target(data.models(), false, true);
        let realizer = CellRealizer::new(&target);
        realizer.realize_cells();

        assert_state(&target, &data, 10, false, &[IndexPath::new(0)]);
    }

    #[test]
    fn root_can_be_expanded_via_model() {
        let _app = start_app();
        let data = create_data(5, 5);
        let target = create_target(data.models(), false, true);
        let realizer = CellRealizer::new(&target);

        realizer.realize_cells();
        assert_state(&target, &data, 5, false, &[]);

        data.list().get(0).set_is_expanded(true);

        assert_state(&target, &data, 10, false, &[IndexPath::new(0)]);
    }

    #[test]
    fn child_can_be_expanded_via_model() {
        let _app = start_app();
        let data = create_data(5, 5);
        children_of(&child_at(&data, &index_path![0, 1])).list().add(Node::new());

        let target = create_target(data.models(), false, true);
        let realizer = CellRealizer::new(&target);

        realizer.realize_cells();
        assert_state(&target, &data, 5, false, &[]);

        data.list().get(0).set_is_expanded(true);
        realizer.realize_row(&index_path![0, 1]);
        child_at(&data, &index_path![0, 1]).set_is_expanded(true);

        assert_state(&target, &data, 11, false, &[IndexPath::new(0), index_path![0, 1]]);
    }

    #[test]
    fn expanding_collapsing_root_row_writes_to_model() {
        let _app = start_app();
        let data = create_data(5, 5);
        let target = create_target(data.models(), false, true);
        let realizer = CellRealizer::new(&target);

        realizer.realize_cells();
        assert_state(&target, &data, 5, false, &[]);

        set_row_is_expanded(&target, 0, true);

        assert_state(&target, &data, 10, false, &[IndexPath::new(0)]);

        set_row_is_expanded(&target, 0, false);

        assert_state(&target, &data, 5, false, &[]);
    }

    #[test]
    fn expanding_collapsing_child_row_writes_to_model() {
        let _app = start_app();
        let data = create_data(5, 5);
        children_of(&child_at(&data, &index_path![0, 1])).list().add(Node::new());

        let target = create_target(data.models(), false, true);
        let realizer = CellRealizer::new(&target);

        realizer.realize_cells();
        assert_state(&target, &data, 5, false, &[]);

        set_row_is_expanded(&target, 0, true);
        set_row_is_expanded(&target, 2, true);

        assert_state(&target, &data, 11, false, &[IndexPath::new(0), index_path![0, 1]]);

        set_row_is_expanded(&target, 2, false);

        assert_state(&target, &data, 10, false, &[IndexPath::new(0)]);
    }

    /// The state of the rows, and the expansion of the models matches it.
    fn assert_state(
        target: &HierarchicalTreeDataGridSource<Node>,
        data: &ListDebug<Node>,
        expected_rows: usize,
        sorted: bool,
        expanded: &[IndexPath],
    ) {
        super::assert_state(target, data, expected_rows, sorted, expanded);
        assert_data_state(&IndexPath::UNSELECTED, &data.list().to_vec(), expanded);
    }

    fn assert_data_state(parent_index: &IndexPath, data: &[Rc<Node>], expanded: &[IndexPath]) {
        for (i, node) in data.iter().enumerate() {
            let node_index = parent_index.append(i as i32);
            assert_eq!(expanded.contains(&node_index), node.is_expanded());

            if let Some(children) = node.children() {
                assert_data_state(&node_index, &children.list().to_vec(), expanded);
            }
        }
    }

    /// `((IExpander)target.Rows[row_index]).IsExpanded = value`.
    fn set_row_is_expanded(target: &HierarchicalTreeDataGridSource<Node>, row_index: usize, value: bool) {
        let row = target.rows().get(row_index);
        row.as_expander().expect("an expander row").set_is_expanded(value);
    }

    /// `target.Rows.RealizeCell(...)` of the reference's `RealizeCells` and `RealizeRow`: the cells
    /// realized by the rows of the presentation of the source. The presentation and the cells are
    /// kept alive, as the reference's realized cells stay subscribed to their models.
    struct CellRealizer {
        presentation: Rc<TreeDataGridSourcePresentation>,
        cells: RefCell<Vec<CellRef>>,
    }

    impl CellRealizer {
        fn new(target: &Rc<HierarchicalTreeDataGridSource<Node>>) -> Self {
            Self { presentation: presentation_of::<Node, _>(target), cells: RefCell::new(Vec::new()) }
        }

        /// `RealizeCells`: every cell of every row; the count of the rows is read at each row, as
        /// realizing a cell may expand a row.
        fn realize_cells(&self) {
            let columns = self.presentation.columns();
            let rows = self.presentation.rows();
            for c in 0..columns.count() {
                let column = columns.get(c);
                let mut r = 0;
                while r < rows.count() {
                    let cell = rows.realize_cell(&column, c as i32, r as i32);
                    self.cells.borrow_mut().push(cell);
                    r += 1;
                }
            }
        }

        /// `RealizeRow`: the cells of the row of the model at `model_index`.
        fn realize_row(&self, model_index: &IndexPath) {
            let columns = self.presentation.columns();
            let rows = self.presentation.rows();
            let row_index = rows.model_index_to_row_index(model_index);

            assert_ne!(-1, row_index);

            for c in 0..columns.count() {
                let column = columns.get(c);
                let cell = rows.realize_cell(&column, c as i32, row_index);
                self.cells.borrow_mut().push(cell);
            }
        }
    }
}

/// `HierarchicalTreeDataGridSourceTests.ShowExpander`.
mod show_expander {
    use super::*;

    #[test]
    fn initially_hides_expander_with_no_children() {
        let _app = start_app();
        let data = create_data(1, 0);
        let target = create_target(data.models(), false, false);
        let (_presentation, cell) = realize_cell(&target, 0, 0);
        let expander = expander_cell(&cell);

        assert!(!expander.show_expander());
    }

    #[test]
    fn initially_shows_expander_with_children() {
        let _app = start_app();
        let data = create_data(1, 1);
        let target = create_target(data.models(), false, false);
        let (_presentation, cell) = realize_cell(&target, 0, 0);
        let expander = expander_cell(&cell);

        assert!(expander.show_expander());
    }

    #[test]
    fn shows_expander_when_first_child_added() {
        let _app = start_app();
        let data = create_data(1, 0);
        let target = create_target(data.models(), false, false);
        let (_presentation, cell) = realize_cell(&target, 0, 0);
        let expander = expander_cell(&cell);
        let raised = counter();

        {
            let raised = raised.clone();
            expander.property_changed().add(Rc::new(move |name: &str| {
                assert_eq!("ShowExpander", name);
                bump(&raised);
            }));
        }

        children_of(&data.list().get(0)).list().add(Node::new());

        assert!(expander.show_expander());
        assert_eq!(1, raised.get());
    }

    #[test]
    fn hides_expander_when_last_child_removed() {
        let _app = start_app();
        let data = create_data(1, 1);
        let target = create_target(data.models(), false, false);
        let (_presentation, cell) = realize_cell(&target, 0, 0);
        let expander = expander_cell(&cell);
        let raised = counter();

        {
            let raised = raised.clone();
            expander.property_changed().add(Rc::new(move |name: &str| {
                assert_eq!("ShowExpander", name);
                bump(&raised);
            }));
        }

        children_of(&data.list().get(0)).list().remove_at(0);

        assert!(!expander.show_expander());
        assert_eq!(1, raised.get());
    }

    #[test]
    fn cell_synchronizes_row_show_expander() {
        let _app = start_app();
        let data = create_data(1, 1);
        let target = create_target(data.models(), false, false);
        let row = hierarchical_row(&target.rows().get(0));
        let (_presentation, cell) = realize_cell(&target, 0, 0);
        let expander = expander_cell(&cell);

        assert!(expander.show_expander());
        assert!(row.show_expander());

        children_of(&data.list().get(0)).list().remove_at(0);

        assert!(!expander.show_expander());
        assert!(!row.show_expander());
    }
}

/// `HierarchicalTreeDataGridSourceTests.Selection`.
mod selection {
    use super::*;

    #[test]
    fn reassigning_source_updates_selection_model_source() {
        let _app = start_app();
        let data1 = create_data(5, 5);
        let data2 = create_data(5, 5);
        let target = create_target(data1.models(), false, false);

        // Ensure selection model is created.
        assert!(same_items(&selection_source(&target), &data1));

        target.set_items(data2.models());

        assert!(same_items(&selection_source(&target), &data2));
    }

    /// `((ITreeDataGridSelection?)target.RowSelection)!.Source`.
    fn selection_source(target: &HierarchicalTreeDataGridSource<Node>) -> Models<Node> {
        let selection = target.row_selection().expect("a row selection");
        let source = ITreeDataGridSelection::source(&*selection).expect("a selection source");
        downcast_models::<Node>(&source).expect("a source of nodes")
    }

    /// `Assert.Same(data, source)`.
    fn same_items(source: &Models<Node>, data: &Rc<ListDebug<Node>>) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(source), Rc::as_ptr(data))
    }
}

/// `HierarchicalTreeDataGridSourceTests.Items`.
mod items {
    use super::*;

    #[test]
    fn can_reassign_items() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);
            let raised = counter();

            assert_eq!(5, target.rows().count());

            {
                let raised = raised.clone();
                target.rows().add_collection_changed(Rc::new(move |e: &RowsChangedArgs<'_>| {
                    assert_eq!(NotifyCollectionChangedAction::Reset, e.action);
                    bump(&raised);
                }));
            }

            target.set_items(create_data(10, 5).models());

            assert_eq!(10, target.rows().count());
            assert_eq!(1, raised.get());
        }
    }

    #[test]
    fn can_reassign_items_with_expanded_node() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);
            let raised = counter();

            target.expand(&IndexPath::new(0));
            assert_eq!(10, target.rows().count());

            {
                let raised = raised.clone();
                target.rows().add_collection_changed(Rc::new(move |e: &RowsChangedArgs<'_>| {
                    assert_eq!(NotifyCollectionChangedAction::Reset, e.action);
                    bump(&raised);
                }));
            }

            target.set_items(create_data(12, 5).models());

            assert_eq!(12, target.rows().count());
            assert_eq!(1, raised.get());
        }
    }

    #[test]
    fn reassigning_items_with_expanded_root_node_unsubscribes_from_collection_changed() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);
            let to_remove = data.list().get(1);

            target.expand(&IndexPath::new(1));
            assert_eq!(1, to_remove.children().expect("children").collection_changed_subscriber_count());

            target.set_items(create_data(12, 5).models());

            assert_eq!(0, to_remove.children().expect("children").collection_changed_subscriber_count());
        }
    }

    #[test]
    fn reassigning_items_with_expanded_child_node_unsubscribes_from_collection_changed() {
        let _app = start_app();
        for sorted in [false, true] {
            let data = create_data(5, 5);
            let target = create_target(data.models(), sorted, false);
            let to_remove = child_at(&data, &index_path![1, 1]);

            to_remove.set_children(Some(ListDebug::from_items([Node::new()])));

            target.expand(&index_path![1, 1]);
            assert_eq!(1, to_remove.children().expect("children").collection_changed_subscriber_count());

            target.set_items(create_data(12, 5).models());

            assert_eq!(0, to_remove.children().expect("children").collection_changed_subscriber_count());
        }
    }

    #[test]
    fn selects_correct_item_after_items_reassigned() {
        let _app = start_app();
        let data = create_data(5, 5);
        let target = create_target(data.models(), false, false);
        let raised = counter();

        target.row_selection().expect("a row selection").base().select(&index_path![1, 0]);

        let new_data = create_data(10, 5);
        child_at(&new_data, &index_path![1, 0]).set_caption(Some("New Selection".to_string()));
        target.set_items(new_data.models());

        {
            let raised = raised.clone();
            target.row_selection().expect("a row selection").add_selection_changed(Rc::new(
                move |e: &TreeSelectionModelSelectionChangedEventArgsOf<Node>| {
                    let selected_indexes = e.selected_indexes();
                    assert_eq!(1, selected_indexes.count());
                    assert_eq!(index_path![1, 0], selected_indexes.get(0));
                    let selected_items = e.selected_items();
                    assert_eq!(1, selected_items.count());
                    assert_eq!(Some("New Selection".to_string()), selected_items.get(0).expect("a selected item").caption());
                    bump(&raised);
                },
            ));
        }

        target.row_selection().expect("a row selection").base().select(&index_path![1, 0]);

        assert_eq!(1, raised.get());
    }
}

#[test]
fn supports_filtering_children() {
    let _app = start_app();
    let data = create_data(2, 2);
    // The core's hierarchical source has no `Filter`: the reference's source shows the roots that
    // pass the filter and filters the children through a `FilteredExpanderColumn` around its
    // expander column. The port builds the same: the expander column of `CreateTarget` wrapped in
    // a `FilteredExpanderColumn` that reads the current filter, and the roots that pass the filter
    // as the items of the source.
    let filter: Rc<RefCell<Option<ModelFilter<Node>>>> = Rc::new(RefCell::new(None));
    let target = create_filterable_target(&data, &filter);

    apply_filter(&target, &data, &filter, Some(Rc::new(|x: &Rc<Node>| x.id() % 2 == 0)));

    assert_eq!(1, target.rows().count());

    target.expand(&IndexPath::new(0));

    assert_eq!(2, target.rows().count());
    assert_eq!(0, hierarchical_row(&target.rows().get(0)).model().id());
    assert_eq!(2, hierarchical_row(&target.rows().get(1)).model().id());
}

#[test]
fn row_events_use_tree_data_grid_row_model_event_args() {
    let _app = start_app();
    let data = create_data(1, 1);
    let target = create_target(data.models(), false, false);
    let expanded: Rc<RefCell<Option<TreeDataGridRowModelEventArgs>>> = Rc::new(RefCell::new(None));
    let collapsed: Rc<RefCell<Option<TreeDataGridRowModelEventArgs>>> = Rc::new(RefCell::new(None));

    // The core's source raises its row events with the row; the reference's arguments are the
    // row's model and model index (`TreeDataGridRowModelEventArgs`), made from it here.
    target.add_row_expanded(record_row_event(&expanded));
    target.add_row_collapsed(record_row_event(&collapsed));

    target.expand(&IndexPath::new(0));
    target.collapse(&IndexPath::new(0));

    let expanded = expanded.borrow().clone().expect("a RowExpanded event");
    let collapsed = collapsed.borrow().clone().expect("a RowCollapsed event");
    assert!(same_model(&data.list().get(0), expanded.row().model().as_ref()));
    assert_eq!(IndexPath::new(0), *expanded.row().model_index_path());
    assert!(same_model(&data.list().get(0), collapsed.row().model().as_ref()));
    assert_eq!(IndexPath::new(0), *collapsed.row().model_index_path());
}

/// A handler of a row event that records its arguments as `TreeDataGridRowModelEventArgs`.
fn record_row_event(slot: &Rc<RefCell<Option<TreeDataGridRowModelEventArgs>>>) -> Rc<RowEventHandler<Node>> {
    let slot = slot.clone();
    Rc::new(move |e: &RowEventArgs<Rc<HierarchicalRow<Node>>>| {
        let model = e.row.model().clone() as BoxedValue;
        let row = TreeDataGridRowModel::new(Some(model), e.row.model_index_path());
        *slot.borrow_mut() = Some(TreeDataGridRowModelEventArgs::new(row));
    })
}

/// The source of `Supports_Filtering_Children`: the columns of `CreateTarget`, the expander
/// filtered by `filter`.
fn create_filterable_target(
    data: &Rc<ListDebug<Node>>,
    filter: &Rc<RefCell<Option<ModelFilter<Node>>>>,
) -> Rc<HierarchicalTreeDataGridSource<Node>> {
    let expander = HierarchicalExpanderColumn::new(id_column(), node_children, None, None);
    let current = filter.clone();
    let filtered = FilteredExpanderColumn::new(expander as Rc<dyn IViewExpanderColumn<Node>>, move || current.borrow().clone());
    HierarchicalTreeDataGridSource::new(data.models())
        .with_column(filtered as ColumnRef<Node>)
        .with_column(caption_column())
}

/// `target.Filter(predicate)`: the children are filtered by the expander column, the roots are the
/// models of `data` that pass the filter.
fn apply_filter(
    target: &HierarchicalTreeDataGridSource<Node>,
    data: &Rc<ListDebug<Node>>,
    filter: &Rc<RefCell<Option<ModelFilter<Node>>>>,
    predicate: Option<ModelFilter<Node>>,
) {
    *filter.borrow_mut() = predicate.clone();
    let items: Models<Node> = match predicate {
        Some(predicate) => {
            let roots: Vec<Rc<Node>> = data.list().to_vec().into_iter().filter(|x| predicate(x)).collect();
            Rc::new(roots)
        }
        None => data.models(),
    };
    target.set_items(items);
}

/// `count` roots with `child_count` children each (`CreateData(int count = 5, int childCount = 5)`).
fn create_data(count: i32, child_count: i32) -> Rc<ListDebug<Node>> {
    let mut id = 0;
    let result = ListDebug::new();

    for i in 0..count {
        let node = Node::create(id, Some(&format!("Node {i}")), Some(ListDebug::new()));
        id += 1;

        result.list().add(node.clone());

        for j in 0..child_count {
            children_of(&node).list().add(Node::create(id, Some(&format!("Node {i}-{j}")), Some(ListDebug::new())));
            id += 1;
        }
    }

    result
}

/// A tree with `counts[level]` nodes under each node of a level (`CreateData(params int[] counts)`).
fn create_data_levels(counts: &[i32]) -> Rc<ListDebug<Node>> {
    fn create(counts: &[i32], index: usize, result: &ListDebug<Node>, id: &mut i32) {
        let count = counts[index];

        for i in 0..count {
            let node = Node::create(*id, Some(&format!("Node {i}")), Some(ListDebug::new()));
            *id += 1;

            if index < counts.len() - 1 {
                create(counts, index + 1, &children_of(&node), id);
            }

            result.list().add(node);
        }
    }

    let mut id = 0;
    let result = ListDebug::new();
    create(counts, 0, &result, &mut id);
    result
}

/// The source of the tests: an expander column around an `ID` text column, its expansion bound
/// to `Node.IsExpanded` with `bind_expanded`, and a `Caption` text column; sorted by descending id
/// with `sorted`.
fn create_target(roots: Models<Node>, sorted: bool, bind_expanded: bool) -> Rc<HierarchicalTreeDataGridSource<Node>> {
    let result = HierarchicalTreeDataGridSource::new(roots)
        .with_column(HierarchicalExpanderColumn::new(
            id_column(),
            node_children,
            None,
            bind_expanded.then(is_expanded_accessor),
        ) as ColumnRef<Node>)
        .with_column(caption_column());

    if sorted {
        result.sort(Some(by_descending_id()));
    }

    result
}

/// `new TextColumn<Node, int>("ID", x => x.Id)`.
fn id_column() -> Rc<dyn IViewColumn<Node>> {
    TextColumn::new(header("ID"), Accessor::member("Id", |x: &Rc<Node>| x.id()), None, None)
}

/// `new TextColumn<Node, string?>("Caption", x => x.Caption)`.
fn caption_column() -> ColumnRef<Node> {
    TextColumn::new(header("Caption"), Accessor::member("Caption", |x: &Rc<Node>| x.caption()), None, None)
        as ColumnRef<Node>
}

/// `x => x.IsExpanded`, with the setter the reference synthesizes from it.
fn is_expanded_accessor() -> Accessor<Node, bool> {
    Accessor::member("IsExpanded", |x: &Rc<Node>| x.is_expanded()).with_setter(|x: &Rc<Node>, value: bool| x.set_is_expanded(value))
}

/// `x => x.Children`.
fn node_children(x: &Rc<Node>) -> Option<Models<Node>> {
    x.children().map(|children| children.models())
}

/// `(x, y) => y.Id - x.Id`.
fn by_descending_id() -> Comparison<Node> {
    Rc::new(|x: &Rc<Node>, y: &Rc<Node>| y.id().cmp(&x.id()))
}

/// The rows of `target` are the models of `data`, each level sorted by descending id with
/// `sorted`, the models at `expanded` expanded.
fn assert_state(
    target: &HierarchicalTreeDataGridSource<Node>,
    data: &ListDebug<Node>,
    expected_rows: usize,
    sorted: bool,
    expanded: &[IndexPath],
) {
    assert_eq!(2, target.columns().count());
    let rows = target.rows();
    assert_eq!(expected_rows, rows.count());

    let data = data.list().to_vec();
    let mut row_index = 0usize;
    assert_level(&*rows, &data, &IndexPath::UNSELECTED, &data, sorted, expanded, &mut row_index);
}

fn assert_level(
    rows: &dyn IRows,
    data: &[Rc<Node>],
    parent: &IndexPath,
    level_data: &[Rc<Node>],
    sorted: bool,
    expanded: &[IndexPath],
    row_index: &mut usize,
) {
    let mut sorted_data = level_data.to_vec();

    if sorted {
        sorted_data.sort_by(|x, y| y.id().cmp(&x.id()));
    }

    for item in &sorted_data {
        let position = level_data.iter().position(|x| Rc::ptr_eq(x, item)).expect("a model of the level") as i32;
        let model_index = parent.append(position);
        let model = get_model(data, &model_index);
        let row = hierarchical_row(&rows.get(*row_index));
        let should_be_expanded = expanded.contains(&model_index);

        assert_eq!(model_index, row.model_index_path());
        assert!(
            row.is_expanded() == should_be_expanded,
            "Expected index {model_index} IsExpanded == {should_be_expanded}"
        );

        *row_index += 1;

        if row.is_expanded() {
            let children = model.children().expect("the children of an expanded model");
            assert_level(rows, data, &model_index, &children.list().to_vec(), sorted, expanded, row_index);
        }
    }
}

/// The model at `path` of `data`.
///
/// # Panics
/// For the empty path (`NotSupportedException`).
fn get_model(data: &[Rc<Node>], path: &IndexPath) -> Rc<Node> {
    let depth = path.count();

    if depth == 0 {
        panic!("Specified method is not supported.");
    }

    let mut node: Option<Rc<Node>> = None;

    for i in 0..depth {
        let j = path.get(i) as usize;
        node = Some(match node {
            None => data[j].clone(),
            Some(parent) => children_of(&parent).list().get(j),
        });
    }

    node.expect("a model")
}

/// The model at `path` of `data` (`data[0].Children![1]`).
fn child_at(data: &ListDebug<Node>, path: &IndexPath) -> Rc<Node> {
    get_model(&data.list().to_vec(), path)
}

/// `node.Children!`.
fn children_of(node: &Rc<Node>) -> Rc<ListDebug<Node>> {
    node.children().expect("children")
}

/// `Assert.IsType<HierarchicalRow<Node>>(row)`.
fn hierarchical_row(row: &RowRef) -> Rc<HierarchicalRow<Node>> {
    as_hierarchical_row::<Node>(row).expect("a hierarchical row")
}

/// `target.Rows.RealizeCell(target.Columns[column_index], column_index, row_index)`: the cell
/// realized by the rows of the presentation of the source, returned with the presentation, which
/// the test keeps alive while it uses the cell.
fn realize_cell(
    target: &Rc<HierarchicalTreeDataGridSource<Node>>,
    column_index: usize,
    row_index: usize,
) -> (Rc<TreeDataGridSourcePresentation>, CellRef) {
    let presentation = presentation_of::<Node, _>(target);
    let column = presentation.columns().get(column_index);
    let cell = presentation.rows().realize_cell(&column, column_index as i32, row_index as i32);
    (presentation, cell)
}

/// `(ExpanderCell<Node>)cell`.
fn expander_cell(cell: &CellRef) -> &ExpanderCell<Node> {
    cell.as_any().downcast_ref::<ExpanderCell<Node>>().expect("an expander cell")
}

/// `target.Rows.CollectionChanged += (s, e) => ++raised`.
fn count_rows_changes(rows: &dyn IRows) -> Rc<Cell<i32>> {
    let raised = counter();
    let counted = raised.clone();
    rows.add_collection_changed(Rc::new(move |_e: &RowsChangedArgs<'_>| bump(&counted)));
    raised
}

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

fn counter() -> Rc<Cell<i32>> {
    Rc::new(Cell::new(0))
}

fn bump(counter: &Cell<i32>) {
    counter.set(counter.get() + 1);
}

/// `Node : NotifyingBase` of the tests.
struct Node {
    notifier: TestNotifier,
    id: Cell<i32>,
    caption: RefCell<Option<String>>,
    children: RefCell<Option<Rc<ListDebug<Node>>>>,
    is_expanded: Cell<bool>,
}

impl Node {
    /// `new Node()`.
    fn new() -> Rc<Self> {
        Model::new_model(Self {
            notifier: TestNotifier::new(),
            id: Cell::new(0),
            caption: RefCell::new(None),
            children: RefCell::new(None),
            is_expanded: Cell::new(false),
        })
    }

    /// `new Node { Id = id, Caption = caption, Children = children }`.
    fn create(id: i32, caption: Option<&str>, children: Option<Rc<ListDebug<Node>>>) -> Rc<Self> {
        let node = Self::new();
        node.set_id(id);
        node.set_caption(caption.map(str::to_string));
        node.set_children(children);
        node
    }

    fn id(&self) -> i32 {
        self.id.get()
    }

    fn set_id(&self, value: i32) {
        self.notifier.raise_and_set_cell_if_changed(&self.id, value, "Id");
    }

    fn caption(&self) -> Option<String> {
        self.caption.borrow().clone()
    }

    fn set_caption(&self, value: Option<String>) {
        self.notifier.raise_and_set_if_changed(&self.caption, value, "Caption");
    }

    fn children(&self) -> Option<Rc<ListDebug<Node>>> {
        self.children.borrow().clone()
    }

    /// Sets the children, compared by identity (`RaiseAndSetIfChanged` of a reference).
    fn set_children(&self, value: Option<Rc<ListDebug<Node>>>) {
        let same = match (&*self.children.borrow(), &value) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if !same {
            *self.children.borrow_mut() = value;
            self.notifier.raise("Children");
        }
    }

    fn is_expanded(&self) -> bool {
        self.is_expanded.get()
    }

    fn set_is_expanded(&self, value: bool) {
        self.notifier.raise_and_set_cell_if_changed(&self.is_expanded, value, "IsExpanded");
    }
}

impl INotifyPropertyChanged for Node {
    fn property_changed(&self) -> &Event<str> {
        self.notifier.event()
    }
}

// `Children` is read by the child selector, never through a binding, so only the values the
// columns and the expansion binding read are described.
ferro_model!(Node, |b| b
    .notify_property_changed()
    .property::<Value<i32>>("Id", |o| o.id(), |o, v| o.set_id(v))
    .property::<Maybe<String>>("Caption", |o| o.caption(), |o, v| o.set_caption(v))
    .property::<Value<bool>>("IsExpanded", |o| o.is_expanded(), |o, v| o.set_is_expanded(v)));

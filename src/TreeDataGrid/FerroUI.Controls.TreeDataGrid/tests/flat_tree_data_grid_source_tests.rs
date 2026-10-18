//! `FlatTreeDataGridSourceTests.cs`: the flat source with the control's text columns: its rows
//! follow the changes of the items, unsorted and sorted, its filter, its sort and its selection.
//!
//! The reference tests its classic `FlatTreeDataGridSource<Row>` with the classic `TextColumn`s;
//! the port has one data layer, so the source is the core's `FlatTreeDataGridSource` holding the
//! control's view columns (`crate::models::TextColumn`), and the reference's notifying list of rows
//! is a notifying `FerroList`.

use super::support::{model_of, observable, start_app, TestNotifier};
use crate::models::TextColumn;
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction};
use ferroui_base::data::core::{Maybe, Value};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::{ferro_model, BoxedValue};
use ferroui_treedatagrid_core::models::{
    ColumnRef, Comparison, IColumn as CoreColumn, IRow, IRows, ListSortDirection, RowsChangedArgs,
};
use ferroui_treedatagrid_core::selection::{downcast_models, ITreeDataGridSelection};
use ferroui_treedatagrid_core::{Accessor, FlatTreeDataGridSource, ITreeDataGridSource, Models};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn selection_can_be_explicitly_disabled() {
    let target = FlatTreeDataGridSource::<Row>::new(Rc::new(Vec::<Rc<Row>>::new()) as Models<Row>);

    target.set_selection(None);

    assert!(target.selection().is_none());
}

#[test]
fn creates_initial_rows() {
    let _app = start_app();
    let data = create_data(10);
    let target = create_target(data.clone() as Models<Row>);

    assert_rows(&*target.rows(), &data.to_vec());
}

#[test]
fn supports_adding_row() {
    let _app = start_app();
    let data = create_data(10);
    let target = create_target(data.clone() as Models<Row>);

    assert_eq!(10, target.rows().count());

    let raised = counter();
    {
        let raised = raised.clone();
        subscribe_rows(&*target.rows(), move |e: &RowsChangedArgs<'_>| {
            assert_eq!(NotifyCollectionChangedAction::Add, e.action);
            assert_eq!(10, e.new_starting_index);
            bump(&raised);
        });
    }

    data.add(Row::new(10, "New Row 10"));

    assert_eq!(11, target.rows().count());
    assert_eq!(1, raised.get());

    assert_rows(&*target.rows(), &data.to_vec());
}

#[test]
fn supports_removing_row() {
    let _app = start_app();
    let data = create_data(10);
    let target = create_target(data.clone() as Models<Row>);

    assert_eq!(10, target.rows().count());

    let raised = counter();
    {
        let raised = raised.clone();
        subscribe_rows(&*target.rows(), move |e: &RowsChangedArgs<'_>| {
            assert_eq!(NotifyCollectionChangedAction::Remove, e.action);
            assert_eq!(5, e.old_starting_index);
            bump(&raised);
        });
    }

    data.remove_at(5);

    assert_eq!(1, raised.get());
    assert_rows(&*target.rows(), &data.to_vec());
}

#[test]
fn supports_replacing_row() {
    let _app = start_app();
    let data = create_data(10);
    let target = create_target(data.clone() as Models<Row>);

    assert_eq!(10, target.rows().count());

    let raised = counter();
    {
        let raised = raised.clone();
        subscribe_rows(&*target.rows(), move |e: &RowsChangedArgs<'_>| {
            assert_eq!(NotifyCollectionChangedAction::Replace, e.action);
            assert_eq!(5, e.new_starting_index);
            assert_eq!(5, e.old_starting_index);
            bump(&raised);
        });
    }

    data.set(5, Row::new(10, "New Row 10"));

    assert_eq!(1, raised.get());
    assert_rows(&*target.rows(), &data.to_vec());
}

#[test]
fn supports_moving_row() {
    let _app = start_app();
    let data = create_data(10);
    let target = create_target(data.clone() as Models<Row>);

    assert_eq!(10, target.rows().count());

    let raised = counter();
    {
        let raised = raised.clone();
        subscribe_rows(&*target.rows(), move |e: &RowsChangedArgs<'_>| {
            assert_eq!(NotifyCollectionChangedAction::Move, e.action);
            assert_eq!(8, e.new_starting_index);
            assert_eq!(5, e.old_starting_index);
            bump(&raised);
        });
    }

    data.move_item(5, 8);

    assert_eq!(1, raised.get());
    assert_rows(&*target.rows(), &data.to_vec());
}

#[test]
fn supports_clearing_rows() {
    let _app = start_app();
    let data = create_data(10);
    let target = create_target(data.clone() as Models<Row>);

    assert_eq!(10, target.rows().count());

    let raised = counter();
    {
        let raised = raised.clone();
        subscribe_rows(&*target.rows(), move |e: &RowsChangedArgs<'_>| {
            assert_eq!(NotifyCollectionChangedAction::Reset, e.action);
            bump(&raised);
        });
    }

    data.clear();

    assert_eq!(1, raised.get());
    assert_rows(&*target.rows(), &data.to_vec());
}

#[test]
fn can_reassign_items() {
    let _app = start_app();
    let data = create_data(10);
    let target = create_target(data.clone() as Models<Row>);
    let raised = counter();

    assert_rows(&*target.rows(), &data.to_vec());

    {
        let raised = raised.clone();
        subscribe_rows(&*target.rows(), move |e: &RowsChangedArgs<'_>| {
            assert_eq!(NotifyCollectionChangedAction::Reset, e.action);
            bump(&raised);
        });
    }

    let data = create_data(20);
    target.set_items(data.clone() as Models<Row>);

    assert_eq!(1, raised.get());
    assert_rows(&*target.rows(), &data.to_vec());
}

#[test]
fn raises_rows_reset_when_reassigning_items_but_rows_not_yet_read() {
    let _app = start_app();
    let data = create_data(10);
    let target = create_target(data as Models<Row>);
    let raised = counter();

    {
        let raised = raised.clone();
        subscribe_rows(&*target.rows(), move |e: &RowsChangedArgs<'_>| {
            if e.action == NotifyCollectionChangedAction::Reset {
                bump(&raised);
            }
        });
    }

    target.set_items(create_data(10) as Models<Row>);

    assert_eq!(1, raised.get());
}

#[test]
fn supports_filter_and_refresh() {
    let _app = start_app();
    let data = create_data(10);
    let target = create_target(data.clone() as Models<Row>);
    let max_id = Rc::new(Cell::new(4));

    {
        let max_id = max_id.clone();
        target.set_filter(Some(Rc::new(move |x: &Rc<Row>| x.id() <= max_id.get())));
    }

    assert_rows(&*target.rows(), &data.to_vec()[..5]);

    max_id.set(2);
    target.refresh_filter();

    assert_rows(&*target.rows(), &data.to_vec()[..3]);

    target.set_filter(None);

    assert_rows(&*target.rows(), &data.to_vec());
}

#[test]
fn supports_clearing_sort() {
    let _app = start_app();
    let data = create_data(10);
    let target = create_target(data.clone() as Models<Row>);

    let column = target.columns().get(0);
    let untyped: &dyn CoreColumn = &*column;
    assert!(ITreeDataGridSource::sort_by(&*target, Some(untyped), ListSortDirection::Descending));
    assert_eq!(9, model_of::<Row>(target.rows().get(0).model()).id());

    ITreeDataGridSource::clear_sort(&*target);

    assert_eq!(None, target.columns().get(0).sort_direction());
    assert_rows(&*target.rows(), &data.to_vec());
}

/// `FlatTreeDataGridSourceTests.Sorted`: the rows sorted by descending id.
mod sorted {
    use super::*;

    #[test]
    fn sorts_initial_cells() {
        let _app = start_app();
        let data = create_data(10);
        let target = create_target(data.clone() as Models<Row>);

        assert_eq!(10, target.rows().count());

        assert_rows(&*target.rows(), &data.to_vec());
    }

    #[test]
    fn supports_adding_row() {
        let _app = start_app();
        let data = create_data(10);
        let target = create_target(data.clone() as Models<Row>);

        assert_rows(&*target.rows(), &data.to_vec());

        let raised = counter();
        {
            let raised = raised.clone();
            subscribe_rows(&*target.rows(), move |e: &RowsChangedArgs<'_>| {
                assert_eq!(NotifyCollectionChangedAction::Add, e.action);
                assert_eq!(0, e.new_starting_index);
                assert_eq!(1, e.new_items.len());
                assert_eq!(10, model_index_of(&e.new_items[0]));
                bump(&raised);
            });
        }

        data.add(Row::new(10, "New Row 10"));

        assert_eq!(11, target.rows().count());
        assert_eq!(1, raised.get());

        assert_rows(&*target.rows(), &data.to_vec());
    }

    #[test]
    fn supports_removing_row() {
        let _app = start_app();
        let data = create_data(10);
        let target = create_target(data.clone() as Models<Row>);

        assert_rows(&*target.rows(), &data.to_vec());

        let raised = counter();
        {
            let raised = raised.clone();
            subscribe_rows(&*target.rows(), move |e: &RowsChangedArgs<'_>| {
                assert_eq!(NotifyCollectionChangedAction::Remove, e.action);
                assert_eq!(4, e.old_starting_index);
                assert_eq!(1, e.old_items.len());
                assert_eq!(5, model_index_of(&e.old_items[0]));
                bump(&raised);
            });
        }

        data.remove_at(5);

        assert_eq!(1, raised.get());
        assert_rows(&*target.rows(), &data.to_vec());
    }

    #[test]
    fn supports_replacing_row() {
        let _app = start_app();
        let data = create_data(10);
        let target = create_target(data.clone() as Models<Row>);

        assert_rows(&*target.rows(), &data.to_vec());

        let raised = counter();
        {
            let raised = raised.clone();
            subscribe_rows(&*target.rows(), move |e: &RowsChangedArgs<'_>| {
                match e.action {
                    NotifyCollectionChangedAction::Remove => assert_eq!(4, e.old_starting_index),
                    NotifyCollectionChangedAction::Add => assert_eq!(0, e.new_starting_index),
                    _ => panic!("Unexpected collection change"),
                }
                bump(&raised);
            });
        }

        data.set(5, Row::new(10, "New Row 10"));

        assert_eq!(2, raised.get());
        assert_rows(&*target.rows(), &data.to_vec());
    }

    #[test]
    fn supports_moving_row() {
        let _app = start_app();
        let data = create_data(10);
        let target = create_target(data.clone() as Models<Row>);

        assert_rows(&*target.rows(), &data.to_vec());

        let raised = counter();
        {
            let raised = raised.clone();
            subscribe_rows(&*target.rows(), move |e: &RowsChangedArgs<'_>| {
                match e.action {
                    NotifyCollectionChangedAction::Remove => assert_eq!(4, e.old_starting_index),
                    NotifyCollectionChangedAction::Add => assert_eq!(4, e.new_starting_index),
                    _ => panic!("Unexpected collection change"),
                }
                bump(&raised);
            });
        }

        data.move_item(5, 8);

        assert_eq!(2, raised.get());
        assert_rows(&*target.rows(), &data.to_vec());
    }

    #[test]
    fn supports_clearing_rows() {
        let _app = start_app();
        let data = create_data(10);
        let target = create_target(data.clone() as Models<Row>);

        assert_rows(&*target.rows(), &data.to_vec());

        let raised = counter();
        {
            let raised = raised.clone();
            subscribe_rows(&*target.rows(), move |e: &RowsChangedArgs<'_>| {
                assert_eq!(NotifyCollectionChangedAction::Reset, e.action);
                bump(&raised);
            });
        }

        data.clear();

        assert_eq!(1, raised.get());
        assert_rows(&*target.rows(), &data.to_vec());
    }

    #[test]
    fn can_reassign_items() {
        let _app = start_app();
        let data = create_data(10);
        let target = create_target(data.clone() as Models<Row>);
        let raised = counter();

        assert_rows(&*target.rows(), &data.to_vec());

        {
            let raised = raised.clone();
            subscribe_rows(&*target.rows(), move |e: &RowsChangedArgs<'_>| {
                assert_eq!(NotifyCollectionChangedAction::Reset, e.action);
                bump(&raised);
            });
        }

        let data = create_data(20);
        target.set_items(data.clone() as Models<Row>);

        assert_eq!(1, raised.get());
        assert_rows(&*target.rows(), &data.to_vec());
    }

    #[test]
    fn raises_rows_reset_when_reassigning_items_but_rows_not_yet_read() {
        let _app = start_app();
        let data = create_data(10);
        let target = create_target(data as Models<Row>);
        let raised = counter();

        {
            let raised = raised.clone();
            subscribe_rows(&*target.rows(), move |e: &RowsChangedArgs<'_>| {
                if e.action == NotifyCollectionChangedAction::Reset {
                    bump(&raised);
                }
            });
        }

        target.set_items(create_data(10) as Models<Row>);

        assert_eq!(1, raised.get());
    }

    /// The target of the parent tests with its rows sorted by descending id
    /// (`((AnonymousSortableRows<Row>)result.Rows).Sort(...)`): the rows are sorted, not the
    /// source, as in the reference.
    fn create_target(rows: Models<Row>) -> Rc<FlatTreeDataGridSource<Row>> {
        let result = super::create_target(rows);
        let comparison: Comparison<Row> = Rc::new(|x: &Rc<Row>, y: &Rc<Row>| y.id().cmp(&x.id()));
        result.typed_rows().sort(Some(comparison));
        result
    }

    /// The rows are the models by descending id, each with the index of its model in `data`.
    fn assert_rows(rows: &dyn IRows, data: &[Rc<Row>]) {
        assert_eq!(data.len(), rows.count());

        let mut sorted_data = data.to_vec();
        sorted_data.sort_by(|x, y| y.id().cmp(&x.id()));

        for (i, expected) in sorted_data.iter().enumerate() {
            let row = rows.get(i);
            let model = model_of::<Row>(row.model());
            let model_index = model_index_of(&row);
            assert!(Rc::ptr_eq(expected, &model));
            let index_in_data = data.iter().position(|x| Rc::ptr_eq(x, &model)).map_or(-1, |i| i as i32);
            assert_eq!(index_in_data, model_index);
        }
    }
}

/// `FlatTreeDataGridSourceTests.Selection`.
mod selection {
    use super::*;

    #[test]
    fn reassigning_source_updates_selection_model_source() {
        let _app = start_app();
        let data1 = create_data(10);
        let data2 = create_data(5);
        let target = create_target(data1.clone() as Models<Row>);

        // Ensure selection model is created.
        assert!(same_items(&selection_source(&target), &data1));

        target.set_items(data2.clone() as Models<Row>);

        assert!(same_items(&selection_source(&target), &data2));
    }

    /// `((ITreeDataGridSelection?)target.RowSelection)!.Source`.
    fn selection_source(target: &FlatTreeDataGridSource<Row>) -> Models<Row> {
        let selection = target.row_selection().expect("a row selection");
        let source = ITreeDataGridSelection::source(&*selection).expect("a selection source");
        downcast_models::<Row>(&source).expect("a source of rows")
    }

    /// `Assert.Same(data, source)`.
    fn same_items(source: &Models<Row>, data: &Rc<FerroList<Rc<Row>>>) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(source), Rc::as_ptr(data))
    }
}

/// The source of the tests: an `ID` and a `Caption` text column.
fn create_target(rows: Models<Row>) -> Rc<FlatTreeDataGridSource<Row>> {
    FlatTreeDataGridSource::new(rows)
        .with_column(TextColumn::new(header("ID"), Accessor::member("Id", |x: &Rc<Row>| x.id()), None, None) as ColumnRef<Row>)
        .with_column(
            TextColumn::new(header("Caption"), Accessor::member("Caption", |x: &Rc<Row>| x.caption()), None, None)
                as ColumnRef<Row>,
        )
}

/// `count` rows with the ids `0..count` (the reference's notifying list of the rows).
fn create_data(count: i32) -> Rc<FerroList<Rc<Row>>> {
    observable((0..count).map(|x| Row::new(x, &format!("Row {x}"))))
}

/// The rows are the models of `data` in order, each with its index.
fn assert_rows(rows: &dyn IRows, data: &[Rc<Row>]) {
    assert_eq!(data.len(), rows.count());

    for (i, expected) in data.iter().enumerate() {
        // The rows of a flat source are one reused row: each is read before the next is fetched.
        let row = rows.get(i);
        let model = model_of::<Row>(row.model());
        assert!(Rc::ptr_eq(&model, expected));
        assert_eq!(i as i32, model_index_of(&row));
    }
}

/// `((IModelIndexableRow)row).ModelIndex`.
fn model_index_of(row: &Rc<dyn IRow>) -> i32 {
    row.as_model_indexable().expect("a model indexable row").model_index()
}

/// Subscribes to the changes of the rows (`target.Rows.CollectionChanged += ...`).
fn subscribe_rows(rows: &dyn IRows, handler: impl for<'a> Fn(&RowsChangedArgs<'a>) + 'static) -> u64 {
    rows.add_collection_changed(Rc::new(handler))
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

/// `Row : NotifyingBase` of the tests.
struct Row {
    notifier: TestNotifier,
    id: Cell<i32>,
    caption: RefCell<Option<String>>,
}

impl Row {
    /// `new Row { Id = id, Caption = caption }`.
    fn new(id: i32, caption: &str) -> Rc<Self> {
        let row = Model::new_model(Self { notifier: TestNotifier::new(), id: Cell::new(0), caption: RefCell::new(None) });
        row.set_id(id);
        row.set_caption(Some(caption.to_string()));
        row
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
}

impl INotifyPropertyChanged for Row {
    fn property_changed(&self) -> &Event<str> {
        self.notifier.event()
    }
}

ferro_model!(Row, |b| b
    .notify_property_changed()
    .property::<Value<i32>>("Id", |o| o.id(), |o, v| o.set_id(v))
    .property::<Maybe<String>>("Caption", |o| o.caption(), |o, v| o.set_caption(v)));

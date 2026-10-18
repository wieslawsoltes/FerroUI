//! `CellRebindLifecycleTests.cs`.
//!
//! The reference's exceptions thrown by the lifecycle callbacks are panics, and its derived
//! column, cell model, factory and cell are the support module's layout test column with
//! overrides, a text cell wrapper, a factory behaviour and a class derived from the layout test
//! cell control.

use super::support::{
    assert_panics, column_list, rows_of_array, start_app, test_window, LayoutTestCell, LayoutTestCellControl,
    LayoutTestColumn, LayoutTestColumnState,
};
use ferroui_treedatagrid_core::models::RowRef;
use crate::models::{
    BeginEditGestures, CellRef, ColumnRef, ICell, IColumns, IEditableObject, ITextCell, ITreeDataGridRows, TextCell,
};
use crate::primitives::{
    TreeDataGridCellImpl, TreeDataGridCellImplExt, TreeDataGridCellsPresenter, TreeDataGridElementFactory,
    TreeDataGridElementFactoryImpl,
};
use crate::selection::ITreeDataGridSelectionInteraction;
use crate::{TreeDataGrid, TreeDataGridCellEventArgs};
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, Ref, Size, StyledElementImpl, VisualImpl};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::templates::FuncControlTemplate;
use ferroui_controls::{Control, ControlImpl, Window};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// A model of the rows (`object`).
struct Item;

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// The record of the fixture: the lifecycle events and the callback that fails.
struct Recorder {
    events: RefCell<Vec<String>>,
    failure: RefCell<Option<String>>,
    record_lifecycle: bool,
}

impl Recorder {
    fn record(&self, name: &str) {
        self.events.borrow_mut().push(name.to_string());
        let fails = self.failure.borrow().as_deref() == Some(name);
        if fails {
            panic!("{name}");
        }
    }

    fn events(&self) -> Vec<String> {
        self.events.borrow().clone()
    }
}

/// `RecordingModel`: a text cell of `"value"` whose disposal is recorded.
struct RecordingModel {
    inner: Rc<TextCell<String>>,
    recorder: Rc<Recorder>,
}

impl ICell for RecordingModel {
    fn can_edit(&self) -> bool {
        ICell::can_edit(&*self.inner)
    }

    fn edit_gestures(&self) -> BeginEditGestures {
        ICell::edit_gestures(&*self.inner)
    }

    fn value(&self) -> Option<BoxedValue> {
        ICell::value(&*self.inner)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_text_cell(&self) -> Option<&dyn ITextCell> {
        ICell::as_text_cell(&*self.inner)
    }

    fn as_notify_property_changed(&self) -> Option<&dyn INotifyPropertyChanged> {
        ICell::as_notify_property_changed(&*self.inner)
    }

    fn as_editable(&self) -> Option<&dyn IEditableObject> {
        ICell::as_editable(&*self.inner)
    }

    fn dispose(&self) {
        self.recorder.record("dispose");
        ICell::dispose(&*self.inner);
    }
}

/// `HookFactory`: creates [`HookCell`]s.
struct HookFactory {
    recorder: Rc<Recorder>,
}

impl TreeDataGridElementFactoryImpl for HookFactory {
    fn create_element(&self, _factory: &TreeDataGridElementFactory, _data: &dyn Any) -> Ref<Control> {
        HookCell::new(self.recorder.clone()).upcast()
    }

    fn get_data_recycle_key(&self, _factory: &TreeDataGridElementFactory, _data: &dyn Any) -> String {
        HookCell::TYPE.full_name()
    }
}

/// `HookCell`: a layout test cell control that records its rebind hooks and lifecycle.
#[repr(C)]
struct HookCell {
    base: LayoutTestCellControl,
    recorder: Rc<Recorder>,
}

ferro_class!(HookCell: LayoutTestCellControl);
ferroui_base::ferro_impl_classes!(
    HookCell: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl TreeDataGridCellImpl for HookCell {
    fn begin_rebind(this: &Self) {
        this.recorder.record("begin");
    }

    fn end_rebind(this: &Self, realized: bool) {
        this.recorder.record(&format!("end:{}", if realized { "True" } else { "False" }));
    }

    fn unrealize(this: &Self) {
        this.recorder.record("unrealize");
        Self::parent_unrealize(this);
    }

    fn realize(
        this: &Self,
        factory: &Rc<TreeDataGridElementFactory>,
        selection: Option<&dyn ITreeDataGridSelectionInteraction>,
        model: &CellRef,
        column_index: i32,
        row_index: i32,
    ) {
        this.recorder.record("realize");
        Self::parent_realize(this, factory, selection, model, column_index, row_index);
    }
}

impl HookCell {
    fn new(recorder: Rc<Recorder>) -> Ref<Self> {
        instantiate(Self { base: LayoutTestCellControl::construct(), recorder })
    }
}

struct Fixture {
    recorder: Rc<Recorder>,
    presenter: Ref<TreeDataGridCellsPresenter>,
    window: Ref<Window>,
    _subscriptions: Vec<Rc<dyn IDisposable>>,
}

impl Fixture {
    fn new(reuse: bool, null_value: bool, record_lifecycle: bool) -> Self {
        let recorder = Rc::new(Recorder {
            events: RefCell::new(Vec::new()),
            failure: RefCell::new(None),
            record_lifecycle,
        });

        let create_recorder = recorder.clone();
        let reuse_recorder = recorder.clone();
        let column = LayoutTestColumn::<Item>::with_overrides(
            "Value",
            None,
            None,
            Some(Box::new(move |state: &LayoutTestColumnState, row: &RowRef| -> CellRef {
                if !create_recorder.record_lifecycle {
                    return if null_value { LayoutTestCell::new(None) as CellRef } else { state.default_cell(row) };
                }
                create_recorder.record("create");
                Rc::new(RecordingModel {
                    inner: TextCell::from_value(Some("value".to_string())),
                    recorder: create_recorder.clone(),
                }) as CellRef
            })),
            Some(Box::new(move |_: &CellRef, _: &RowRef| {
                reuse_recorder.record("reuse");
                reuse
            })),
        );
        let columns = column_list([column as ColumnRef]);

        let presenter = TreeDataGridCellsPresenter::new();
        presenter.set_element_factory(Some(TreeDataGridElementFactory::with_impl(HookFactory { recorder: recorder.clone() })));
        presenter.set_items(Some(columns as Rc<dyn IColumns>));
        presenter.set_rows(Some(rows_of_array(vec![Rc::new(Item), Rc::new(Item)]) as Rc<dyn ITreeDataGridRows>));
        presenter.realize(0);

        let mut subscriptions = Vec::new();
        let mut root: Ref<Control> = presenter.clone().upcast();

        if record_lifecycle {
            let grid = TreeDataGrid::new();
            let template_presenter = presenter.clone();
            grid.set_template(Some(FuncControlTemplate::for_type::<TreeDataGrid>(move |_, _| {
                template_presenter.clone().upcast()
            })));

            let clearing_recorder = recorder.clone();
            subscriptions.push(grid.cell_clearing(move |e: &TreeDataGridCellEventArgs| {
                clearing_recorder.record(&format!("clearing:{}", e.row_index()));
            }));
            let prepared_recorder = recorder.clone();
            subscriptions.push(grid.cell_prepared(move |e: &TreeDataGridCellEventArgs| {
                prepared_recorder.record(&format!("prepared:{}", e.row_index()));
            }));
            let index_recorder = recorder.clone();
            subscriptions.push(presenter.child_index_changed(move |_| index_recorder.record("index")));
            root = grid.upcast();
        }

        let window = test_window(Some(root), Some(Size::new(100.0, 100.0)));
        window.update_layout();
        recorder.events.borrow_mut().clear();

        Self { recorder, presenter, window, _subscriptions: subscriptions }
    }

    fn cell(&self) -> Ref<HookCell> {
        self.presenter.realized_elements()[0]
            .as_ref()
            .and_then(|element| element.cast::<HookCell>())
            .expect("a hook cell")
    }

    fn set_failure(&self, failure: Option<&str>) {
        *self.recorder.failure.borrow_mut() = failure.map(str::to_string);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.set_failure(None);
        self.window.set_content(None);
        self.window.close();
    }
}

fn same_cell(a: &Option<CellRef>, b: &Option<CellRef>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

fn hooks_bracket_replacement_or_reuse_without_changing_lifecycle_order(reuse: bool, null_value: bool) {
    let _app = start_app();
    let fixture = Fixture::new(reuse, null_value, false);
    let original = fixture.cell().model();
    fixture.presenter.unrealize();
    // Row-level deferred unrealization is not a cell rebind yet.
    assert!(fixture.recorder.events().is_empty());
    fixture.presenter.realize(1);
    assert_eq!(vec!["begin", "unrealize", "reuse", "realize", "end:True"], fixture.recorder.events());
    assert_eq!(reuse, same_cell(&original, &fixture.cell().model()));
    assert_eq!(1, fixture.cell().row_index());
    assert_eq!(0, fixture.cell().column_index());
    if null_value {
        assert!(fixture.cell().model().expect("a model").value().is_none());
    }
}

#[test]
fn hooks_bracket_replacement_without_changing_lifecycle_order() {
    hooks_bracket_replacement_or_reuse_without_changing_lifecycle_order(false, false);
}

#[test]
fn hooks_bracket_reuse_without_changing_lifecycle_order() {
    hooks_bracket_replacement_or_reuse_without_changing_lifecycle_order(true, false);
}

#[test]
fn hooks_bracket_replacement_of_a_null_value_without_changing_lifecycle_order() {
    hooks_bracket_replacement_or_reuse_without_changing_lifecycle_order(false, true);
}

#[test]
fn hooks_bracket_reuse_of_a_null_value_without_changing_lifecycle_order() {
    hooks_bracket_replacement_or_reuse_without_changing_lifecycle_order(true, true);
}

fn failed_callbacks_always_end_the_rebind_with_failure(failure: &str) {
    let _app = start_app();
    let fixture = Fixture::new(true, false, false);
    fixture.set_failure(Some(failure));
    fixture.presenter.unrealize();
    assert_panics(|| fixture.presenter.realize(1));
    let expected = ["begin", "unrealize", "reuse", "realize"];
    let position = expected.iter().position(|name| *name == failure).expect("a callback");
    let mut expected: Vec<String> = expected[..=position].iter().map(|name| name.to_string()).collect();
    expected.push("end:False".to_string());
    assert_eq!(expected, fixture.recorder.events());
    fixture.set_failure(None);
}

#[test]
fn failed_begin_always_ends_the_rebind_with_failure() {
    failed_callbacks_always_end_the_rebind_with_failure("begin");
}

#[test]
fn failed_unrealize_always_ends_the_rebind_with_failure() {
    failed_callbacks_always_end_the_rebind_with_failure("unrealize");
}

#[test]
fn failed_reuse_always_ends_the_rebind_with_failure() {
    failed_callbacks_always_end_the_rebind_with_failure("reuse");
}

#[test]
fn failed_realize_always_ends_the_rebind_with_failure() {
    failed_callbacks_always_end_the_rebind_with_failure("realize");
}

#[test]
fn finalizing_an_idle_row_does_not_call_rebind_hooks() {
    let _app = start_app();
    let fixture = Fixture::new(false, false, false);
    fixture.presenter.unrealize();
    let cell = fixture.cell();
    fixture.presenter.finalize_unrealize();
    assert_eq!(vec!["unrealize"], fixture.recorder.events());
    assert!(cell.model().is_none());
}

fn prepared_clearing_indexes_and_disposal_keep_their_order(reuse: bool) {
    let _app = start_app();
    let fixture = Fixture::new(reuse, false, true);
    fixture.presenter.unrealize();
    fixture.presenter.realize(1);
    let mut expected = vec!["begin", "unrealize", "clearing:0", "index", "reuse"];
    if !reuse {
        expected.extend(["dispose", "create"]);
    }
    expected.extend(["realize", "prepared:1", "index", "end:True"]);
    assert_eq!(expected, fixture.recorder.events());
}

#[test]
fn prepared_clearing_indexes_and_disposal_keep_their_order_on_replacement() {
    prepared_clearing_indexes_and_disposal_keep_their_order(false);
}

#[test]
fn prepared_clearing_indexes_and_disposal_keep_their_order_on_reuse() {
    prepared_clearing_indexes_and_disposal_keep_their_order(true);
}

fn ordinary_unrealization_preserves_existing_model_disposal_order(row_removed: bool) {
    let _app = start_app();
    let fixture = Fixture::new(false, false, true);
    if row_removed {
        fixture.presenter.unrealize_on_row_removed();
    } else {
        fixture.presenter.unrealize();
        fixture.presenter.finalize_unrealize();
    }
    let expected = if row_removed {
        vec!["dispose", "unrealize", "clearing:0", "index"]
    } else {
        vec!["unrealize", "clearing:0", "dispose", "index"]
    };
    assert_eq!(expected, fixture.recorder.events());
}

#[test]
fn ordinary_unrealization_preserves_existing_model_disposal_order_when_recycled() {
    ordinary_unrealization_preserves_existing_model_disposal_order(false);
}

#[test]
fn ordinary_unrealization_preserves_existing_model_disposal_order_when_the_row_is_removed() {
    ordinary_unrealization_preserves_existing_model_disposal_order(true);
}

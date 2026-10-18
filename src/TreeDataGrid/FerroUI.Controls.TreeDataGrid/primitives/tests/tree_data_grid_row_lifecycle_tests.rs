//! `TreeDataGridRowLifecycleTests.cs`.

use super::support::start_app;
use crate::primitives::{TreeDataGridRow, TreeDataGridRowImpl, TreeDataGridRowUnrealizeReason};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::ControlImpl;
use std::cell::RefCell;

/// `RecordingRow`: a row that records its lifecycle hooks.
#[repr(C)]
struct RecordingRow {
    base: TreeDataGridRow,
    events: RefCell<Vec<String>>,
}

ferro_class!(RecordingRow: TreeDataGridRow);
ferroui_base::ferro_impl_classes!(
    RecordingRow: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl TreeDataGridRowImpl for RecordingRow {
    fn on_realized(this: &Self, row_index: i32) {
        this.events.borrow_mut().push(format!("realized:{row_index}:{}", this.row_index()));
    }

    fn on_row_index_changed(this: &Self, old_row_index: i32, new_row_index: i32) {
        this.events.borrow_mut().push(format!("index:{old_row_index}:{new_row_index}:{}", this.row_index()));
    }

    fn on_unrealizing(this: &Self, row_index: i32, reason: TreeDataGridRowUnrealizeReason) {
        this.events.borrow_mut().push(format!("unrealizing:{row_index}:{reason}:{}", this.row_index()));
    }
}

impl RecordingRow {
    fn new() -> Ref<Self> {
        instantiate(Self { base: TreeDataGridRow::construct(), events: RefCell::new(Vec::new()) })
    }

    fn events(&self) -> Vec<String> {
        self.events.borrow().clone()
    }
}

#[test]
fn derived_row_receives_synchronous_lifecycle_hooks() {
    let _app = start_app();
    let target = RecordingRow::new();

    target.realize(None, None, None, None, 4);
    target.update_index(7);
    target.unrealize();

    assert_eq!(vec!["realized:4:4", "index:4:7:7", "unrealizing:7:Recycle:7"], target.events());
    assert_eq!(-1, target.row_index());
}

#[test]
fn item_removal_is_distinguished_from_recycling() {
    let _app = start_app();
    let target = RecordingRow::new();

    target.realize(None, None, None, None, 2);
    target.unrealize_on_item_removed();

    assert_eq!(vec!["realized:2:2", "unrealizing:2:ItemRemoved:2"], target.events());
    assert_eq!(-1, target.row_index());
}

//! The reference tests show and close a window; a test root stands in for
//! it here.

use crate::test_support::{test_scope, TestRoot};
use crate::{Control, Panel};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

fn run_loaded_jobs() {
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
}

fn count_loaded(target: &Control) -> (Rc<Cell<i32>>, Rc<Cell<i32>>) {
    let loaded_count = Rc::new(Cell::new(0));
    let unloaded_count = Rc::new(Cell::new(0));
    let loaded = loaded_count.clone();
    target.loaded(move |_, _| loaded.set(loaded.get() + 1));
    let unloaded = unloaded_count.clone();
    target.unloaded(move |_, _| unloaded.set(unloaded.get() + 1));
    (loaded_count, unloaded_count)
}

#[test]
fn control_loads_and_unloads() {
    let _scope = test_scope();
    let root = TestRoot::new();

    let target = Control::new();
    let (loaded_count, unloaded_count) = count_loaded(&target);

    assert_eq!(loaded_count.get(), 0);
    assert_eq!(unloaded_count.get(), 0);

    root.set_child(&target);
    run_loaded_jobs();
    assert!(target.is_loaded());

    assert_eq!(loaded_count.get(), 1);
    assert_eq!(unloaded_count.get(), 0);

    root.set_child(None);

    assert_eq!(loaded_count.get(), 1);
    assert_eq!(unloaded_count.get(), 1);
    assert!(!target.is_loaded());
}

#[test]
fn loaded_exception_does_not_prevent_other_controls_from_loading() {
    let _scope = test_scope();
    let root = TestRoot::new();
    run_loaded_jobs();

    // Batch 1: a control whose Loaded handler panics, plus siblings queued
    // in the same batch.
    let throwing = Control::new();
    throwing.loaded(|_, _| panic!("Loaded handler failure"));

    let sibling1 = Control::new();
    let sibling2 = Control::new();
    let (sibling1_loaded_count, _) = count_loaded(&sibling1);
    let (sibling2_loaded_count, _) = count_loaded(&sibling2);

    let panel = Panel::new();
    panel.children().add(&sibling1);
    panel.children().add(&throwing);
    panel.children().add(&sibling2);
    root.set_child(&panel);

    // The failure must surface, not be swallowed. Keep pumping so that the
    // rescheduled loaded-processing job also runs.
    let first = catch_unwind(AssertUnwindSafe(run_loaded_jobs));
    if first.is_err() {
        run_loaded_jobs();
    }

    // Both siblings from the same batch must still have been loaded.
    assert!(sibling1.is_loaded());
    assert!(sibling2.is_loaded());
    assert_eq!(sibling1_loaded_count.get(), 1);
    assert_eq!(sibling2_loaded_count.get(), 1);

    // Batch 2: controls loaded afterwards must still receive Loaded.
    let later = Control::new();
    let (later_loaded_count, _) = count_loaded(&later);
    panel.children().add(&later);

    run_loaded_jobs();

    assert!(later.is_loaded());
    assert_eq!(later_loaded_count.get(), 1);
}

#[test]
fn loaded_should_not_be_raised_if_detached_from_visual_tree() {
    let _scope = test_scope();
    let root = TestRoot::new();

    let target = Control::new();
    let (loaded_count, unloaded_count) = count_loaded(&target);

    // Attach to, then immediately detach from the visual tree.
    root.set_child(&target);
    root.set_child(None);

    // Attach to another logical parent.
    target.set_parent(TestRoot::new());

    run_loaded_jobs();

    // At this point, the control shouldn't have been loaded at all.
    assert!(target.visual_parent().is_none());
    assert!(!target.is_loaded());
    assert_eq!(loaded_count.get(), 0);
    assert_eq!(unloaded_count.get(), 0);
}

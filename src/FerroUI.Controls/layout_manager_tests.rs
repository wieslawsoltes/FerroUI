//! Port of `LayoutManagerTests.cs` (base unit tests). The tests use
//! decorators, panels and borders, so they live with the controls.

use crate::layout_test_control::{LayoutTestControl, LayoutTestRoot};
use crate::test_support::test_scope;
use crate::{Border, Decorator, StackPanel};
use ferroui_base::layout::Layoutable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{Ref, Size};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn measures_and_arranges_invalidate_measured_control() {
    let _scope = test_scope();
    let control = LayoutTestControl::new();
    let root = LayoutTestRoot::new();
    root.set_child(&control);

    root.layout_manager().execute_initial_layout_pass();
    control.set_measured(false);
    control.set_arranged(false);

    control.invalidate_measure();
    root.layout_manager().execute_layout_pass();

    assert!(control.measured());
    assert!(control.arranged());
}

#[test]
fn doesnt_measure_and_arrange_invalidate_measured_control_when_top_level_is_not_visible() {
    let _scope = test_scope();
    let control = LayoutTestControl::new();
    let root = LayoutTestRoot::new();
    root.set_child(&control);
    root.set_is_visible(false);

    root.layout_manager().execute_initial_layout_pass();
    control.set_measured(false);
    control.set_arranged(false);

    control.invalidate_measure();
    root.layout_manager().execute_layout_pass();

    assert!(!control.measured());
    assert!(!control.arranged());
}

#[test]
fn doesnt_measure_and_arrange_invalidate_measured_control_when_ancestor_is_not_visible() {
    let _scope = test_scope();
    let control = LayoutTestControl::new();
    let parent = Decorator::new();
    parent.set_child(&control);
    let root = LayoutTestRoot::new();
    root.set_child(&parent);

    root.layout_manager().execute_initial_layout_pass();
    control.set_measured(false);
    control.set_arranged(false);

    parent.set_is_visible(false);
    control.invalidate_measure();
    root.layout_manager().execute_layout_pass();

    assert!(!control.measured());
    assert!(!control.arranged());
}

#[test]
fn lays_out_descendents_that_were_invalidated_while_ancestor_was_not_visible() {
    // Issue #11076
    let _scope = test_scope();
    let control = LayoutTestControl::new();
    let parent = Decorator::new();
    parent.set_child(&control);
    let grandparent = Decorator::new();
    grandparent.set_child(&parent);
    let root = LayoutTestRoot::new();
    root.set_child(&grandparent);

    root.layout_manager().execute_initial_layout_pass();

    grandparent.set_is_visible(false);
    control.invalidate_measure();
    root.layout_manager().execute_initial_layout_pass();

    grandparent.set_is_visible(true);

    root.layout_manager().execute_layout_pass();

    assert!(control.is_measure_valid());
    assert!(control.is_arrange_valid());
}

#[test]
fn arranges_invalidate_arranged_control() {
    let _scope = test_scope();
    let control = LayoutTestControl::new();
    let root = LayoutTestRoot::new();
    root.set_child(&control);

    root.layout_manager().execute_initial_layout_pass();
    control.set_measured(false);
    control.set_arranged(false);

    control.invalidate_arrange();
    root.layout_manager().execute_layout_pass();

    assert!(!control.measured());
    assert!(control.arranged());
}

#[test]
fn measures_parent_of_newly_added_control() {
    let _scope = test_scope();
    let control = LayoutTestControl::new();
    let root = LayoutTestRoot::new();

    root.layout_manager().execute_initial_layout_pass();
    root.set_child(&control);
    root.set_measured(false);
    root.set_arranged(false);

    root.layout_manager().execute_layout_pass();

    assert!(root.measured());
    assert!(root.arranged());
    assert!(control.measured());
    assert!(control.arranged());
}

/// The measure override of the ordering tests: records the control and
/// measures it as 10x10.
fn recording_measure_override(order: &Rc<RefCell<Vec<Ref<Layoutable>>>>) -> Rc<dyn Fn(&Layoutable, Size) -> Size> {
    let order = order.clone();
    Rc::new(move |control: &Layoutable, _size: Size| {
        order.borrow_mut().push(control.to_ref());
        Size::new(10.0, 10.0)
    })
}

#[test]
fn measures_in_correct_order() {
    let _scope = test_scope();
    let control2 = LayoutTestControl::new();
    let control1 = LayoutTestControl::new();
    control1.set_child(&control2);
    let root = LayoutTestRoot::new();
    root.set_child(&control1);

    let order = Rc::new(RefCell::new(Vec::<Ref<Layoutable>>::new()));

    root.set_do_measure_override(Some(recording_measure_override(&order)));
    control1.set_do_measure_override(Some(recording_measure_override(&order)));
    control2.set_do_measure_override(Some(recording_measure_override(&order)));
    root.layout_manager().execute_initial_layout_pass();

    control2.invalidate_measure();
    control1.invalidate_measure();
    root.invalidate_measure();

    order.borrow_mut().clear();
    root.layout_manager().execute_layout_pass();

    let expected: Vec<Ref<Layoutable>> = vec![root.clone().upcast(), control1.clone().upcast(), control2.clone().upcast()];
    assert_eq!(expected, *order.borrow());
}

#[test]
fn measures_root_and_grandparent_in_correct_order() {
    let _scope = test_scope();
    let control2 = LayoutTestControl::new();
    let control1 = LayoutTestControl::new();
    control1.set_child(&control2);
    let root = LayoutTestRoot::new();
    root.set_child(&control1);

    let order = Rc::new(RefCell::new(Vec::<Ref<Layoutable>>::new()));

    root.set_do_measure_override(Some(recording_measure_override(&order)));
    control1.set_do_measure_override(Some(recording_measure_override(&order)));
    control2.set_do_measure_override(Some(recording_measure_override(&order)));
    root.layout_manager().execute_initial_layout_pass();

    control2.invalidate_measure();
    root.invalidate_measure();

    order.borrow_mut().clear();
    root.layout_manager().execute_layout_pass();

    let expected: Vec<Ref<Layoutable>> = vec![root.clone().upcast(), control2.clone().upcast()];
    assert_eq!(expected, *order.borrow());
}

#[test]
fn doesnt_measure_non_invalidated_root() {
    let _scope = test_scope();
    let control = LayoutTestControl::new();
    let root = LayoutTestRoot::new();
    root.set_child(&control);

    root.layout_manager().execute_initial_layout_pass();
    root.set_measured(false);
    root.set_arranged(false);
    control.set_measured(false);
    control.set_arranged(false);

    control.invalidate_measure();
    root.layout_manager().execute_layout_pass();

    assert!(!root.measured());
    assert!(!root.arranged());
    assert!(control.measured());
    assert!(control.arranged());
}

#[test]
fn doesnt_measure_removed_control() {
    let _scope = test_scope();
    let control = LayoutTestControl::new();
    let root = LayoutTestRoot::new();
    root.set_child(&control);

    root.layout_manager().execute_initial_layout_pass();
    control.set_measured(false);
    control.set_arranged(false);

    control.invalidate_measure();
    root.set_child(None);
    root.layout_manager().execute_layout_pass();

    assert!(!control.measured());
    assert!(!control.arranged());
}

#[test]
fn measures_root_with_infinity() {
    let _scope = test_scope();
    let root = LayoutTestRoot::new();
    let available_size = Rc::new(Cell::new(Size::default()));

    root.set_do_measure_override(Some({
        let available_size = available_size.clone();
        Rc::new(move |_: &Layoutable, s: Size| {
            available_size.set(s);
            Size::new(100.0, 100.0)
        })
    }));

    root.layout_manager().execute_initial_layout_pass();

    assert_eq!(Size::INFINITY, available_size.get());
}

#[test]
fn arranges_root_with_desired_size() {
    let _scope = test_scope();
    let root = LayoutTestRoot::new();
    root.set_width(100.0);
    root.set_height(100.0);

    let arrange_size = Rc::new(Cell::new(Size::default()));

    root.set_do_arrange_override(Some({
        let arrange_size = arrange_size.clone();
        Rc::new(move |_: &Layoutable, s: Size| {
            arrange_size.set(s);
            s
        })
    }));

    root.layout_manager().execute_initial_layout_pass();
    assert_eq!(Size::new(100.0, 100.0), arrange_size.get());

    root.set_width(120.0);

    root.layout_manager().execute_layout_pass();
    assert_eq!(Size::new(120.0, 100.0), arrange_size.get());
}

#[test]
fn invalidating_child_remeasures_parent() {
    let _scope = test_scope();
    let border = Border::new();
    let panel = StackPanel::new();
    panel.children().add(border.clone());

    let root = LayoutTestRoot::new();
    root.set_child(&panel);

    root.layout_manager().execute_initial_layout_pass();
    assert_eq!(Size::new(0.0, 0.0), root.desired_size());

    border.set_width(100.0);
    border.set_height(100.0);

    root.layout_manager().execute_layout_pass();
    assert_eq!(Size::new(100.0, 100.0), panel.desired_size());
}

#[test]
fn layout_manager_should_prevent_infinite_loop_on_measure() {
    let _scope = test_scope();
    let control = LayoutTestControl::new();
    let root = LayoutTestRoot::new();
    root.set_child(&control);

    root.layout_manager().execute_initial_layout_pass();
    control.set_measured(false);

    let cnt = Rc::new(Cell::new(0));
    let maxcnt = 100;
    control.set_do_measure_override(Some({
        let cnt = cnt.clone();
        let control = control.downgrade();
        Rc::new(move |_: &Layoutable, _: Size| {
            //emulate a problem in the logic of a control that triggers
            //invalidate measure during measure
            //it can lead to an infinite loop in layoutmanager
            cnt.set(cnt.get() + 1);
            if cnt.get() < maxcnt {
                control.upgrade().unwrap().invalidate_measure();
            }

            Size::new(100.0, 100.0)
        })
    }));

    control.invalidate_measure();

    root.layout_manager().execute_layout_pass();

    assert!(cnt.get() < 100);
}

#[test]
fn layout_manager_should_prevent_infinite_loop_on_arrange() {
    let _scope = test_scope();
    let control = LayoutTestControl::new();
    let root = LayoutTestRoot::new();
    root.set_child(&control);

    root.layout_manager().execute_initial_layout_pass();
    control.set_arranged(false);

    let cnt = Rc::new(Cell::new(0));
    let maxcnt = 100;
    control.set_do_arrange_override(Some({
        let cnt = cnt.clone();
        let control = control.downgrade();
        Rc::new(move |_: &Layoutable, _: Size| {
            //emulate a problem in the logic of a control that triggers
            //invalidate measure during arrange
            //it can lead to infinity loop in layoutmanager
            cnt.set(cnt.get() + 1);
            if cnt.get() < maxcnt {
                control.upgrade().unwrap().invalidate_arrange();
            }

            Size::new(100.0, 100.0)
        })
    }));

    control.invalidate_arrange();

    root.layout_manager().execute_layout_pass();

    assert!(cnt.get() < 100);
}

#[test]
fn layout_manager_should_properly_arrange_visuals_even_when_there_are_issues_with_previous_arranged() {
    let _scope = test_scope();
    let non_arrageable_targets: Vec<_> = (1..=10).map(|_| LayoutTestControl::new()).collect();
    let targets: Vec<_> = (1..=10).map(|_| LayoutTestControl::new()).collect();

    let panel = StackPanel::new();
    let root = LayoutTestRoot::new();
    root.set_child(&panel);

    for c in non_arrageable_targets.iter().chain(targets.iter()) {
        panel.children().add(c.clone());
    }

    root.layout_manager().execute_initial_layout_pass();

    for c in panel.children().to_vec().into_iter().filter_map(|c| c.cast::<LayoutTestControl>()) {
        c.set_measured(false);
        c.set_arranged(false);
        c.invalidate_measure();
    }

    for c in &non_arrageable_targets {
        let weak = c.downgrade();
        c.set_do_arrange_override(Some(Rc::new(move |_: &Layoutable, _: Size| {
            //emulate a problem in the logic of a control that triggers
            //invalidate measure during arrange
            weak.upgrade().unwrap().invalidate_measure();
            Size::new(100.0, 100.0)
        })));
    }

    root.layout_manager().execute_layout_pass();

    //although nonArrageableTargets has rubbish logic and can't be measured/arranged properly
    //layoutmanager should process properly other visuals
    for c in &targets {
        assert!(c.arranged());
    }
}

#[test]
fn layout_manager_should_recover_from_infinite_loop_on_measure() {
    // Test for issue #3041.
    let _scope = test_scope();
    let control = LayoutTestControl::new();
    let root = LayoutTestRoot::new();
    root.set_child(&control);

    root.layout_manager().execute_initial_layout_pass();
    control.set_measured(false);

    control.set_do_measure_override(Some({
        let control = control.downgrade();
        Rc::new(move |_: &Layoutable, _: Size| {
            control.upgrade().unwrap().invalidate_measure();
            Size::new(100.0, 100.0)
        })
    }));

    control.invalidate_measure();
    root.layout_manager().execute_layout_pass();

    // This is the important part: running a second layout pass in which we exceed the maximum
    // retries causes LayoutQueue<T>.Info.Count to exceed _maxEnqueueCountPerLoop.
    root.layout_manager().execute_layout_pass();

    control.set_measured(false);
    control.set_do_measure_override(None);

    root.layout_manager().execute_layout_pass();

    assert!(control.measured());
    assert!(control.is_measure_valid());
}

#[test]
fn calling_execute_layout_pass_from_execute_initial_layout_pass_does_not_break_measure() {
    // Test for issue #3550.
    let _scope = test_scope();
    let control = LayoutTestControl::new();
    let root = LayoutTestRoot::new();
    root.set_child(&control);
    let count = Rc::new(Cell::new(0));

    root.layout_manager().execute_initial_layout_pass();
    control.set_measured(false);

    control.set_do_measure_override(Some({
        let count = count.clone();
        let control = control.downgrade();
        let root = root.downgrade();
        Rc::new(move |_: &Layoutable, _: Size| {
            let first = count.get() == 0;
            count.set(count.get() + 1);
            if first {
                control.upgrade().unwrap().invalidate_measure();
                root.upgrade().unwrap().layout_manager().execute_layout_pass();
                Size::new(100.0, 100.0)
            } else {
                Size::new(200.0, 200.0)
            }
        })
    }));

    root.invalidate_measure();
    control.invalidate_measure();
    root.layout_manager().execute_initial_layout_pass();

    assert_eq!(Size::new(200.0, 200.0), control.bounds().size());
    assert_eq!(Size::new(200.0, 200.0), control.desired_size());
}

#[test]
fn layout_manager_execute_layout_pass_should_clear_queued_layout_passes() {
    let _scope = test_scope();
    let control = LayoutTestControl::new();
    let root = LayoutTestRoot::new();
    root.set_child(&control);

    let layout_count = Rc::new(Cell::new(0));
    let _subscription = root.layout_updated({
        let layout_count = layout_count.clone();
        move || layout_count.set(layout_count.get() + 1)
    });

    root.layout_manager().invalidate_arrange(&control);
    root.layout_manager().execute_initial_layout_pass();

    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::RENDER));

    assert_eq!(1, layout_count.get());
}

#[test]
fn child_can_invalidate_parent_measure_during_arrange() {
    // Issue #11015.
    //
    // - Child invalidates parent measure in arrange pass
    // - Parent is added to measure & arrange queues
    // - Arrange pass dequeues parent
    // - Measure is not valid so parent is not arranged
    // - Parent is measured
    // - Parent has been dequeued from arrange queue so no arrange is performed
    let _scope = test_scope();
    let child = LayoutTestControl::new();
    let parent = LayoutTestControl::new();
    parent.set_child(&child);
    let root = LayoutTestRoot::new();
    root.set_child(&parent);

    root.layout_manager().execute_initial_layout_pass();

    child.set_do_arrange_override(Some({
        let parent = parent.downgrade();
        Rc::new(move |_: &Layoutable, s: Size| {
            parent.upgrade().unwrap().invalidate_measure();
            s
        })
    }));

    child.invalidate_measure();
    parent.invalidate_measure();

    root.layout_manager().execute_layout_pass();

    assert!(child.is_measure_valid());
    assert!(child.is_arrange_valid());
    assert!(parent.is_measure_valid());
    assert!(parent.is_arrange_valid());
}

#[test]
fn grandparent_can_invalidate_root_measure_during_arrange() {
    // Issue #11161.
    let _scope = test_scope();
    let child = LayoutTestControl::new();
    let parent = LayoutTestControl::new();
    parent.set_child(&child);
    let grandparent = LayoutTestControl::new();
    grandparent.set_child(&parent);
    let root = LayoutTestRoot::new();
    root.set_child(&grandparent);

    root.layout_manager().execute_initial_layout_pass();

    grandparent.set_do_arrange_override(Some({
        let root = root.downgrade();
        Rc::new(move |_: &Layoutable, s: Size| {
            root.upgrade().unwrap().invalidate_measure();
            s
        })
    }));
    grandparent.set_call_base_arrange(true);

    child.invalidate_measure();
    grandparent.invalidate_measure();

    root.layout_manager().execute_layout_pass();

    assert!(child.is_measure_valid());
    assert!(child.is_arrange_valid());
    assert!(parent.is_measure_valid());
    assert!(parent.is_arrange_valid());
    assert!(grandparent.is_measure_valid());
    assert!(grandparent.is_arrange_valid());
    assert!(root.is_measure_valid());
    assert!(root.is_arrange_valid());
}

#[test]
fn great_grandparent_can_invalidate_grandparent_measure_during_arrange() {
    // Issue #7706 (second part: scrollbar gets stuck)
    let _scope = test_scope();
    let child = LayoutTestControl::new();
    let parent = LayoutTestControl::new();
    parent.set_child(&child);
    let grandparent = LayoutTestControl::new();
    grandparent.set_child(&parent);
    let great_grandparent = LayoutTestControl::new();
    great_grandparent.set_child(&grandparent);
    let root = LayoutTestRoot::new();
    root.set_child(&great_grandparent);

    root.layout_manager().execute_initial_layout_pass();

    great_grandparent.set_do_arrange_override(Some({
        let grandparent = grandparent.downgrade();
        Rc::new(move |_: &Layoutable, s: Size| {
            grandparent.upgrade().unwrap().invalidate_measure();
            s
        })
    }));

    child.invalidate_arrange();
    great_grandparent.invalidate_arrange();

    root.layout_manager().execute_layout_pass();

    assert!(child.is_measure_valid());
    assert!(child.is_arrange_valid());
    assert!(parent.is_measure_valid());
    assert!(parent.is_arrange_valid());
    assert!(great_grandparent.is_measure_valid());
    assert!(great_grandparent.is_arrange_valid());
    assert!(root.is_measure_valid());
    assert!(root.is_arrange_valid());
}

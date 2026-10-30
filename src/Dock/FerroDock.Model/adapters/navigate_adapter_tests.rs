use super::NavigateAdapter;
use crate::core::factory_parameter_tests::RecordingFactory;
use crate::core::*;
use crate::test_support::{TestDock, TestNode, TestRoot, TestSplit, TestWindow};
use crate::BoxedValue;
use ferrodock_model::collections::ListHandle;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactiveui::ICommand;
use ferroui_system::IDisposable;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn node(id: &str) -> Rc<dyn IDockable> {
    let node = TestNode::new(None);
    node.set_id(id.to_owned());
    node
}

fn parameter(node: Rc<dyn IDockable>) -> Option<BoxedValue> {
    Some(Rc::new(node))
}

fn assert_active(dock: &dyn IDock, expected: &Rc<dyn IDockable>) {
    assert!(Rc::ptr_eq(&dock.active_dockable().unwrap(), expected));
}

struct Command {
    enabled: Cell<bool>,
    calls: Cell<usize>,
    execute: Box<dyn Fn()>,
}
impl Command {
    fn new(execute: impl Fn() + 'static) -> Rc<Self> {
        Rc::new(Self {
            enabled: Cell::new(true),
            calls: Cell::new(0),
            execute: Box::new(execute),
        })
    }
}
impl ICommand for Command {
    fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
        assert!(parameter.is_none());
        self.enabled.get()
    }
    fn execute(&self, parameter: Option<&dyn Any>) {
        assert!(parameter.is_none());
        self.calls.set(self.calls.get() + 1);
        (self.execute)();
    }
    fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

#[test]
fn snapshot_history_goes_both_directions_and_new_branches_clear_forward_history() {
    let dock = TestDock::new(None);
    let adapter = NavigateAdapter::new(dock.clone());
    let a = node("a");
    let b = node("b");
    let c = node("c");
    let d = node("d");
    adapter.go_back();
    adapter.go_forward();
    assert!(!adapter.can_go_back() && !adapter.can_go_forward());
    adapter.navigate(parameter(a.clone()), true);
    assert!(!adapter.can_go_back());
    adapter.navigate(parameter(b.clone()), true);
    adapter.navigate(parameter(c.clone()), true);
    adapter.go_back();
    assert_active(dock.as_ref(), &b);
    adapter.go_back();
    assert_active(dock.as_ref(), &a);
    assert!(!adapter.can_go_back() && adapter.can_go_forward());
    adapter.go_forward();
    assert_active(dock.as_ref(), &b);
    assert!(adapter.can_go_back() && adapter.can_go_forward());
    adapter.navigate(parameter(d.clone()), true);
    assert!(!adapter.can_go_forward());
    adapter.go_back();
    assert_active(dock.as_ref(), &b);
    adapter.go_forward();
    assert_active(dock.as_ref(), &d);
    assert!(!adapter.can_go_forward());
}

#[test]
fn same_target_and_unsnapshotted_navigation_leave_history_unchanged() {
    let dock = TestDock::new(None);
    let adapter = NavigateAdapter::new(dock.clone());
    let a = node("a");
    let b = node("b");
    let c = node("c");
    adapter.navigate(parameter(a.clone()), true);
    adapter.navigate(parameter(a.clone()), true);
    assert!(!adapter.can_go_back());
    adapter.navigate(parameter(b.clone()), false);
    assert!(!adapter.can_go_back());
    adapter.navigate(parameter(c), true);
    adapter.go_back();
    assert_active(dock.as_ref(), &b);
    adapter.navigate(parameter(a.clone()), false);
    assert!(adapter.can_go_forward());
    adapter.go_forward();
    adapter.go_back();
    assert_active(dock.as_ref(), &a);
}

#[test]
fn reset_clears_active_and_both_histories_and_releases_their_models() {
    let dock = TestDock::new(None);
    let adapter = NavigateAdapter::new(dock.clone());
    let a = node("a");
    let weak = Rc::downgrade(&a);
    adapter.navigate(parameter(a), true);
    adapter.navigate(parameter(node("b")), true);
    adapter.navigate(parameter(node("c")), true);
    adapter.go_back();
    assert!(adapter.can_go_back() && adapter.can_go_forward());
    assert!(weak.upgrade().is_some());
    adapter.navigate(None, false);
    assert!(dock.active_dockable().is_none());
    assert!(!adapter.can_go_back() && !adapter.can_go_forward());
    assert!(weak.upgrade().is_none());
}

#[test]
fn embedded_adapter_does_not_keep_its_containing_dock_alive() {
    let dock = TestDock::new(None);
    let weak = Rc::downgrade(&dock);
    let dock: Rc<dyn IDock> = dock;
    let _adapter = NavigateAdapter::from_weak(Rc::downgrade(&dock));
    drop(dock);
    assert!(weak.upgrade().is_none());
}

#[test]
fn standalone_adapter_retains_its_source_dock_until_released() {
    let dock = TestDock::new(None);
    let weak = Rc::downgrade(&dock);
    let adapter = NavigateAdapter::new(dock);
    assert!(weak.upgrade().is_some());
    adapter.navigate(None, true);
    drop(adapter);
    assert!(weak.upgrade().is_none());
}

#[test]
fn direct_visible_match_precedes_a_nested_duplicate() {
    let dock = TestDock::new(None);
    let nested = TestDock::new(None);
    nested
        .visible_dockables()
        .unwrap()
        .add(Some(node("duplicate")));
    let direct = node("duplicate");
    dock.visible_dockables().unwrap().add(Some(nested));
    dock.visible_dockables().unwrap().add(Some(direct.clone()));
    let adapter = NavigateAdapter::new(dock.clone());
    adapter.navigate(Some(Rc::new("duplicate".to_owned())), true);
    assert_active(dock.as_ref(), &direct);
}

#[test]
fn flattened_lookup_checks_all_descendants_before_their_siblings() {
    let dock = TestDock::new(None);
    let outer = TestDock::new(None);
    let earlier = node("match");
    let later = TestDock::new(None);
    let descendant = node("match");
    later
        .visible_dockables()
        .unwrap()
        .add(Some(descendant.clone()));
    outer.visible_dockables().unwrap().add(Some(earlier));
    outer.visible_dockables().unwrap().add(Some(later));
    dock.visible_dockables().unwrap().add(Some(outer));
    let adapter = NavigateAdapter::new(dock.clone());
    adapter.navigate(Some(Rc::new("match".to_owned())), true);
    assert_active(dock.as_ref(), &descendant);
}

#[test]
fn split_view_pane_and_content_are_navigable_at_root_and_nested_levels() {
    let split = Rc::new(TestSplit::default());
    split.set_visible_dockables(None);
    let pane = TestDock::new(None);
    let leaf = node("pane-leaf");
    pane.visible_dockables().unwrap().add(Some(leaf.clone()));
    *split.split.pane_dockable.borrow_mut() = Some(pane.clone());
    *split.split.content_dockable.borrow_mut() = Some(pane);
    let adapter = NavigateAdapter::new(split.clone());
    adapter.navigate(Some(Rc::new("pane-leaf".to_owned())), true);
    assert_active(split.as_ref(), &leaf);

    let outer = TestDock::new(None);
    let content = node("content");
    *split.split.content_dockable.borrow_mut() = Some(content.clone());
    outer.visible_dockables().unwrap().add(Some(split));
    let adapter = NavigateAdapter::new(outer.clone());
    adapter.navigate(Some(Rc::new("content".to_owned())), true);
    assert_active(outer.as_ref(), &content);
}

#[test]
fn unknown_id_is_a_noop_with_empty_or_populated_children() {
    let dock = TestDock::new(None);
    let adapter = NavigateAdapter::new(dock.clone());
    let active = node("active");
    dock.set_active_dockable(Some(active.clone()));
    for visible in [
        None,
        Some(ListHandle::new()),
        Some(ListHandle::from_items(vec![Some(node("other"))])),
    ] {
        dock.set_visible_dockables(visible);
        adapter.navigate(Some(Rc::new("missing".to_owned())), true);
        assert_active(dock.as_ref(), &active);
        assert!(!adapter.can_go_back());
    }
}

#[test]
#[should_panic(expected = "Invalid root object type")]
fn invalid_root_type_is_rejected() {
    let dock = TestDock::new(None);
    NavigateAdapter::new(dock.clone()).navigate(Some(Rc::new(42_i32)), true);
}

#[test]
fn activation_is_delegated_to_the_factory_when_one_is_present() {
    let factory = Rc::new(RecordingFactory::default());
    let dock = TestDock::new(None);
    dock.set_factory(Some(factory.clone()));
    let old = node("old");
    let next = node("next");
    dock.set_active_dockable(Some(old.clone()));
    let adapter = NavigateAdapter::new(dock.clone());
    adapter.navigate(parameter(next.clone()), true);
    assert!(Rc::ptr_eq(
        &factory.active_dockable_requests.borrow()[0],
        &next
    ));
    assert_active(dock.as_ref(), &old); // Recording factory deliberately performs no activation.
    assert!(adapter.can_go_back());
}

#[test]
fn navigating_between_or_to_the_same_root_exits_then_shows_its_windows() {
    let dock = TestDock::new(None);
    let root = TestRoot::new(None);
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    let exit = Command::new(move || log.borrow_mut().push("exit"));
    let log = events.clone();
    let show = Command::new(move || log.borrow_mut().push("show"));
    *root.root.exit_windows.borrow_mut() = exit.clone();
    *root.root.show_windows.borrow_mut() = show.clone();
    dock.set_active_dockable(Some(root.clone()));
    let adapter = NavigateAdapter::new(dock.clone());
    adapter.navigate(parameter(root.clone()), true);
    assert_eq!(*events.borrow(), ["exit", "show"]);
    assert!(!adapter.can_go_back());
    exit.enabled.set(false);
    show.enabled.set(false);
    adapter.navigate(parameter(root), true);
    assert_eq!(*events.borrow(), ["exit", "show"]);
    adapter.navigate(None, true);
    assert!(dock.active_dockable().is_none());
    assert_eq!(exit.calls.get(), 1);
}

#[test]
fn resetting_an_active_root_runs_exit_before_clearing_active() {
    let dock = TestDock::new(None);
    let root = TestRoot::new(None);
    let weak = Rc::downgrade(&dock);
    let exit = Command::new(move || assert!(weak.upgrade().unwrap().active_dockable().is_some()));
    *root.root.exit_windows.borrow_mut() = exit.clone();
    dock.set_active_dockable(Some(root));
    let adapter = NavigateAdapter::new(dock.clone());
    adapter.navigate(None, false);
    assert_eq!(exit.calls.get(), 1);
    assert!(dock.active_dockable().is_none());
}

#[test]
fn close_forwards_the_active_dockable_only_for_nonroot_docks() {
    let factory = Rc::new(RecordingFactory::default());
    let active = node("active");
    let dock = TestDock::new(None);
    let adapter = NavigateAdapter::new(dock.clone());
    adapter.close(); // Neither factory nor active.
    dock.set_factory(Some(factory.clone()));
    adapter.close(); // No active.
    dock.set_active_dockable(Some(active.clone()));
    adapter.close();
    assert_eq!(factory.calls.borrow().len(), 1);
    let calls = factory.calls.borrow();
    assert_eq!(calls[0].0, "close_dockable");
    assert!(Rc::ptr_eq(&calls[0].1, &active));
    drop(calls);
    let root = TestRoot::new(None);
    root.set_factory(Some(factory.clone()));
    root.set_active_dockable(Some(active));
    let exit = Command::new(|| {});
    *root.root.exit_windows.borrow_mut() = exit.clone();
    NavigateAdapter::new(root.clone()).close();
    assert_eq!(exit.calls.get(), 1);
    exit.enabled.set(false);
    NavigateAdapter::new(root.clone()).close();
    assert_eq!(exit.calls.get(), 1);
    assert_eq!(factory.calls.borrow().len(), 1);
}

#[test]
fn navigating_or_resetting_preserves_documents_in_the_previous_document_dock() {
    // Source regression scenarios; full concrete Factory integration is pending.
    let root = TestRoot::new(None);
    let documents = TestDock::new(None);
    let document = node("document");
    let tools = TestDock::new(None);
    documents
        .visible_dockables()
        .unwrap()
        .add(Some(document.clone()));
    documents.set_active_dockable(Some(document.clone()));
    root.visible_dockables()
        .unwrap()
        .add(Some(documents.clone()));
    root.visible_dockables().unwrap().add(Some(tools.clone()));
    let adapter = NavigateAdapter::new(root.clone());
    for target in [parameter(tools), None] {
        root.set_active_dockable(Some(documents.clone()));
        adapter.navigate(target, false);
        assert!(documents
            .visible_dockables()
            .unwrap()
            .contains(&Some((document).clone())));
        assert_active(documents.as_ref(), &document);
    }
}

#[test]
fn show_windows_presents_each_modal_flag_before_the_active_root_command() {
    let root = TestRoot::new(None);
    let first = TestWindow::new();
    let second = TestWindow::new();
    second.window.is_modal.set(true);
    *root.root.windows.borrow_mut() = Some(ListHandle::from_items(vec![
        Some(first.clone() as Rc<dyn IDockWindow>),
        Some(second.clone() as Rc<dyn IDockWindow>),
    ]));
    let active = TestRoot::new(None);
    let a = first.clone();
    let b = second.clone();
    let show = Command::new(move || {
        assert_eq!(*a.window.calls.borrow(), ["Present"]);
        assert_eq!(*b.window.calls.borrow(), ["PresentModal"]);
    });
    *active.root.show_windows.borrow_mut() = show.clone();
    root.set_active_dockable(Some(active));
    NavigateAdapter::new(root.clone()).show_windows();
    assert_eq!(show.calls.get(), 1);
}

#[test]
fn exit_windows_snapshots_the_list_before_save_exit_and_nested_exit() {
    let root = TestRoot::new(None);
    let first = TestWindow::new();
    let second = TestWindow::new();
    let windows: ListHandle<Option<Rc<dyn IDockWindow>>> = ListHandle::from_items(vec![
        Some(first.clone() as Rc<dyn IDockWindow>),
        Some(second.clone() as Rc<dyn IDockWindow>),
    ]);
    *root.root.windows.borrow_mut() = Some(windows.clone());
    *first.window.on_exit.borrow_mut() = Some(Rc::new(move || windows.clear()));
    let active = TestRoot::new(None);
    let a = first.clone();
    let b = second.clone();
    let exit = Command::new(move || {
        assert_eq!(*a.window.calls.borrow(), ["Save", "Exit"]);
        assert_eq!(*b.window.calls.borrow(), ["Save", "Exit"]);
    });
    *active.root.exit_windows.borrow_mut() = exit.clone();
    root.set_active_dockable(Some(active));
    NavigateAdapter::new(root.clone()).exit_windows();
    assert_eq!(exit.calls.get(), 1);
    assert!(root.root.windows.borrow().as_ref().unwrap().is_empty());
}

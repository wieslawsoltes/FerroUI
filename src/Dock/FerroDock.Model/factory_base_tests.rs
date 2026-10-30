use super::*;
use crate::collections::IList;
use crate::controls::*;
use crate::core::factory_parameter_tests::RecordingFactory;
use crate::test_support::{
    TestControl as Control, TestDock, TestHost, TestNode, TestRoot, TestSplit, TestWindow,
};
use crate::BoxedValue;
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::collections::specialized::{CollectionChange, INotifyCollectionChanged};
use ferroui_system::events::Event;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn node(id: &str) -> Rc<dyn IDockable> {
    let node = TestNode::new(None);
    node.set_id(id.into());
    node
}
fn dock(id: &str) -> Rc<TestDock> {
    let dock = TestDock::new(None);
    dock.set_id(id.into());
    dock
}
fn ids(query: &DockableQuery) -> Vec<String> {
    query
        .iter()
        .map(|item| item.as_ref().expect("nonnull query fixture").id())
        .collect()
}

#[test]
fn locator_state_defaults_and_shared_registries_are_preserved() {
    let factory = RecordingFactory::with_base();
    let base = factory.base.get().unwrap();
    assert!(!base.hide_tools_on_close() && !base.hide_documents_on_close());
    assert!(
        base.default_context_locator().is_none()
            && base.default_host_window_locator().is_none()
            && base.context_locator().is_none()
            && base.host_window_locator().is_none()
            && base.dockable_locator().is_none()
    );
    factory.set_hide_tools_on_close(true);
    factory.set_hide_documents_on_close(true);
    assert!(factory.hide_tools_on_close() && factory.hide_documents_on_close());
    let contexts = Rc::new(RefCell::new(Dictionary::new()));
    factory.set_context_locator(Some(contexts.clone()));
    assert!(Rc::ptr_eq(&factory.context_locator().unwrap(), &contexts));
    let hosts = Rc::new(RefCell::new(Dictionary::new()));
    factory.set_host_window_locator(Some(hosts.clone()));
    assert!(Rc::ptr_eq(&factory.host_window_locator().unwrap(), &hosts));
    let dockables = Rc::new(RefCell::new(Dictionary::new()));
    factory.set_dockable_locator(Some(dockables.clone()));
    assert!(Rc::ptr_eq(&factory.dockable_locator().unwrap(), &dockables));
}

#[test]
fn empty_ids_skip_default_locators_and_registered_null_results_do_not_fall_back() {
    let factory = RecordingFactory::with_base();
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    factory.set_default_context_locator(Some(Rc::new(move || {
        count.set(count.get() + 1);
        Some(Rc::new("default".to_owned()))
    })));
    let count = calls.clone();
    factory.set_default_host_window_locator(Some(Rc::new(move || {
        count.set(count.get() + 1);
        Some(Rc::new(TestHost::default()))
    })));
    assert!(factory.get_context(String::new()).is_none());
    assert!(factory.get_host_window(String::new()).is_none());
    assert_eq!(calls.get(), 0);
    let contexts = Rc::new(RefCell::new(Dictionary::new()));
    contexts
        .borrow_mut()
        .add("null".into(), Some(Rc::new(|| None) as ContextFactory));
    factory.set_context_locator(Some(contexts));
    let hosts = Rc::new(RefCell::new(Dictionary::new()));
    hosts
        .borrow_mut()
        .add("null".into(), Some(Rc::new(|| None) as HostWindowFactory));
    factory.set_host_window_locator(Some(hosts));
    assert!(factory.get_context("null".into()).is_none());
    assert!(factory.get_host_window("null".into()).is_none());
    assert_eq!(calls.get(), 0);
    assert_eq!(
        factory
            .get_context("missing".into())
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "default"
    );
    assert!(factory.get_host_window("missing".into()).is_some());
    assert_eq!(calls.get(), 2);
}

#[test]
fn locator_callbacks_see_shared_edits_and_can_replace_or_clear_their_registry() {
    let factory = RecordingFactory::with_base();
    let contexts = Rc::new(RefCell::new(Dictionary::new()));
    factory.set_context_locator(Some(contexts.clone()));
    let weak = Rc::downgrade(&factory);
    let value: BoxedValue = Rc::new(42_i32);
    let result = value.clone();
    contexts.borrow_mut().add(
        "found".into(),
        Some(Rc::new(move || {
            weak.upgrade().unwrap().set_context_locator(None);
            Some(result.clone())
        }) as ContextFactory),
    );
    assert!(Rc::ptr_eq(
        &factory.get_context("found".into()).unwrap(),
        &value
    ));
    assert!(factory.context_locator().is_none());
    let host = Rc::new(TestHost::default());
    let result = host.clone();
    let hosts = Rc::new(RefCell::new(Dictionary::new()));
    factory.set_host_window_locator(Some(hosts.clone()));
    let registry = Rc::downgrade(&hosts);
    hosts.borrow_mut().add(
        "host".into(),
        Some(Rc::new(move || {
            registry.upgrade().unwrap().borrow_mut().clear();
            Some(result.clone() as Rc<dyn IHostWindow>)
        }) as HostWindowFactory),
    );
    assert!(Rc::ptr_eq(
        &factory.get_host_window("host".into()).unwrap(),
        &(host as Rc<dyn IHostWindow>)
    ));
    assert!(hosts.borrow().is_empty());
}

#[test]
fn find_root_rechecks_owner_after_predicate_and_preserves_virtual_recursion() {
    let factory = RecordingFactory::with_base();
    let first = TestRoot::new(None);
    let second = TestRoot::new(None);
    let leaf = node("leaf");
    leaf.set_owner(Some(first.clone()));
    let target = second.clone();
    let changing = leaf.clone();
    let first_root: Rc<dyn IRootDock> = first.clone();
    let predicate: RootPredicate = Rc::new(move |root| {
        if Rc::ptr_eq(&root, &first_root) {
            changing.set_owner(Some(target.clone()));
            false
        } else {
            true
        }
    });
    assert!(Rc::ptr_eq(
        &factory.find_root(Some(leaf), Some(predicate)).unwrap(),
        &(second as Rc<dyn IRootDock>)
    ));
    assert_eq!(factory.find_root_calls.get(), 2);
    assert!(factory.find_root(Some(node("orphan")), None).is_none());
    assert!(Rc::ptr_eq(
        &factory.find_root(Some(first.clone()), None).unwrap(),
        &(first as Rc<dyn IRootDock>)
    ));
}

#[test]
fn find_dockable_searches_visible_subtrees_before_later_siblings() {
    let factory = RecordingFactory::with_base();
    let parent = dock("parent");
    let branch = dock("branch");
    let child = node("wanted");
    let later = node("wanted");
    branch.visible_dockables().unwrap().add(Some(child.clone()));
    parent.visible_dockables().unwrap().add(Some(branch));
    parent.visible_dockables().unwrap().add(Some(later));
    let result = factory
        .find_dockable(
            Some(parent),
            Some(Rc::new(|item| {
                item.as_ref().expect("nonnull query fixture").id() == "wanted"
            })),
        )
        .unwrap();
    assert!(Rc::ptr_eq(&result, &child));
}

#[test]
fn find_dockable_includes_split_view_extras_once_and_floating_layouts() {
    let factory = RecordingFactory::with_base();
    let root = TestRoot::new(None);
    let split = Rc::new(TestSplit::default());
    let pane = node("pane");
    let content = node("content");
    split.set_pane_dockable(Some(pane.clone()));
    split.set_content_dockable(Some(content.clone()));
    split.visible_dockables().unwrap().add(Some(pane.clone()));
    root.visible_dockables().unwrap().add(Some(split.clone()));
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    assert!(factory
        .find_dockable(
            Some(root.clone()),
            Some(Rc::new(move |item| {
                if item.as_ref().expect("nonnull query fixture").id() == "pane" {
                    count.set(count.get() + 1);
                }
                false
            }))
        )
        .is_none());
    assert_eq!(calls.get(), 1);
    split.set_content_dockable(Some(pane));
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    factory.find_dockable(
        Some(root.clone()),
        Some(Rc::new(move |item| {
            if item.as_ref().expect("nonnull query fixture").id() == "pane" {
                count.set(count.get() + 1);
            }
            false
        })),
    );
    assert_eq!(calls.get(), 1);
    split.set_content_dockable(Some(content.clone()));
    assert!(Rc::ptr_eq(
        &factory
            .find_dockable(
                Some(root.clone()),
                Some(Rc::new(|item| item
                    .as_ref()
                    .expect("nonnull query fixture")
                    .id()
                    == "content"))
            )
            .unwrap(),
        &content
    ));
    let empty = TestWindow::new();
    let window = TestWindow::new();
    let floating = TestRoot::new(None);
    let leaf = node("floating");
    floating
        .visible_dockables()
        .unwrap()
        .add(Some(leaf.clone()));
    window.set_layout(Some(floating));
    root.set_windows(Some(ListHandle::from_plain(vec![
        Some(empty as Rc<dyn IDockWindow>),
        Some(window),
    ])));
    assert!(Rc::ptr_eq(
        &factory
            .find_dockable(
                Some(root),
                Some(Rc::new(|item| item
                    .as_ref()
                    .expect("nonnull query fixture")
                    .id()
                    == "floating"))
            )
            .unwrap(),
        &leaf
    ));
}

#[test]
fn find_query_is_lazy_repeatable_and_retains_source_duplicate_container_results() {
    let factory = RecordingFactory::with_base();
    let parent = dock("parent");
    let branch = dock("branch");
    branch.visible_dockables().unwrap().add(Some(node("leaf")));
    parent.visible_dockables().unwrap().add(Some(branch));
    let calls = Rc::new(RefCell::new(Vec::new()));
    let log = calls.clone();
    let query = factory.find_in_dock(
        Some(parent.clone()),
        Some(Rc::new(move |item| {
            log.borrow_mut()
                .push(item.as_ref().expect("nonnull query fixture").id());
            true
        })),
    );
    let mut first = query.iter();
    assert!(calls.borrow().is_empty());
    assert_eq!(
        first
            .next()
            .unwrap()
            .as_ref()
            .expect("nonnull query fixture")
            .id(),
        "parent"
    );
    assert_eq!(*calls.borrow(), ["parent"]);
    parent.visible_dockables().unwrap().add(Some(node("later")));
    assert_eq!(
        first
            .map(|item| item.as_ref().expect("nonnull query fixture").id())
            .collect::<Vec<_>>(),
        ["branch", "branch", "leaf", "later"]
    );
    assert_eq!(ids(&query), ["parent", "branch", "branch", "leaf", "later"]);
    assert_eq!(
        ids(&query.clone()),
        ["parent", "branch", "branch", "leaf", "later"]
    );
}

#[test]
fn find_query_excludes_split_view_extras_and_windows_as_the_source_overload_does() {
    let factory = RecordingFactory::with_base();
    let root = TestRoot::new(None);
    root.set_id("root".into());
    let split = Rc::new(TestSplit::default());
    split.set_id("split".into());
    split.set_pane_dockable(Some(node("pane")));
    split.set_content_dockable(Some(node("content")));
    root.visible_dockables().unwrap().add(Some(split));
    let window = TestWindow::new();
    let floating = TestRoot::new(None);
    floating.set_id("floating".into());
    window.set_layout(Some(floating));
    root.set_windows(Some(ListHandle::from_plain(vec![Some(
        window as Rc<dyn IDockWindow>,
    )])));
    assert_eq!(
        ids(&factory.find_in_dock(Some(root), Some(Rc::new(|_| true)))),
        ["root", "split", "split"]
    );
}

#[test]
fn query_enumeration_keeps_plain_list_mutation_checks_and_can_be_restarted() {
    let factory = RecordingFactory::with_base();
    let parent = dock("parent");
    let children = ListHandle::from_plain(vec![Some(node("one")), Some(node("two"))]);
    parent.set_visible_dockables(Some(children.clone()));
    let query = factory.find_in_dock(
        Some(parent),
        Some(Rc::new(|item| {
            item.as_ref().expect("nonnull query fixture").id() != "parent"
        })),
    );
    let mut iterator = query.iter();
    assert_eq!(
        iterator
            .next()
            .unwrap()
            .as_ref()
            .expect("nonnull query fixture")
            .id(),
        "one"
    );
    children.add(Some(node("three")));
    assert!(catch_unwind(AssertUnwindSafe(|| iterator.next())).is_err());
    assert_eq!(ids(&query), ["one", "two", "three"]);
}

#[test]
fn query_and_enumerator_retain_the_source_factory_until_both_are_released() {
    let factory = RecordingFactory::with_base();
    let weak = Rc::downgrade(&factory);
    let query = factory.find_in_dock(Some(dock("parent")), Some(Rc::new(|_| true)));
    let iterator = query.iter();
    drop(factory);
    assert!(weak.upgrade().is_some());
    drop(query);
    assert!(weak.upgrade().is_some());
    drop(iterator);
    assert!(weak.upgrade().is_none());
}

#[test]
fn all_controls_query_defers_layout_reads_and_sees_live_control_changes_between_enumerations() {
    let factory = RecordingFactory::with_base();
    let absent = Control::new(None);
    let control = Control::new(Some(dock("old")));
    factory.controls.add(absent.clone());
    factory.controls.add(control.clone());
    let query = factory.find(Some(Rc::new(|_| true)));
    assert_eq!(control.reads.get(), 0);
    control.set_layout(Some(dock("new")));
    assert_eq!(ids(&query), ["new"]);
    assert_eq!(absent.reads.get(), 1);
    factory.controls.add(Control::new(Some(dock("later"))));
    assert_eq!(ids(&query), ["new", "later"]);
}

struct NotifyingList {
    items: ListHandle<Option<Rc<dyn IDockable>>>,
    event: Event<CollectionChange>,
}
impl INotifyCollectionChanged for NotifyingList {
    fn collection_changed(&self) -> &Event<CollectionChange> {
        &self.event
    }
}
impl IList<Option<Rc<dyn IDockable>>> for NotifyingList {
    fn count(&self) -> usize {
        self.items.count()
    }
    fn is_read_only(&self) -> bool {
        true
    }
    fn get(&self, index: usize) -> Option<Rc<dyn IDockable>> {
        self.items.get(index)
    }
    fn set(&self, _: usize, _: Option<Rc<dyn IDockable>>) {
        panic!("read-only");
    }
    fn insert(&self, _: usize, _: Option<Rc<dyn IDockable>>) {
        panic!("read-only");
    }
    fn remove_at(&self, _: usize) {
        panic!("read-only");
    }
    fn clear(&self) {
        panic!("read-only");
    }
    fn index_of(&self, item: &Option<Rc<dyn IDockable>>) -> Option<usize> {
        self.items.index_of(item)
    }
    fn as_notify_collection_changed(&self) -> Option<&dyn INotifyCollectionChanged> {
        Some(self)
    }
}

#[test]
fn normalization_preserves_observable_and_custom_notifying_list_identity_without_calling_creator() {
    let factory = RecordingFactory::with_base();
    factory.create_plain_lists.set(true);
    let base = factory.base.get().unwrap();
    let list = ListHandle::from_items(vec![Some(node("one"))]);
    assert!(base
        .ensure_observable_dockables(Some(list.clone()))
        .ptr_eq(&list));
    let custom = ListHandle::from_list(Rc::new(NotifyingList {
        items: ListHandle::from_plain(vec![Some(node("two"))]),
        event: Event::new(),
    }));
    let normalized = base.ensure_observable_dockables(Some(custom.clone()));
    assert!(normalized.ptr_eq(&custom) && normalized.is_read_only());
    assert_eq!(factory.create_list_calls.get(), 0);
}

#[test]
fn normalization_copies_plain_lists_and_falls_back_when_generic_creator_is_not_observable() {
    for plain_creator in [false, true] {
        let factory = RecordingFactory::with_base();
        factory.create_plain_lists.set(plain_creator);
        let base = factory.base.get().unwrap();
        let child = node("one");
        let plain = ListHandle::from_plain(vec![Some(child.clone())]);
        let normalized = base.ensure_observable_dockables(Some(plain.clone()));
        assert!(!normalized.ptr_eq(&plain));
        assert!(normalized.as_observable_collection().is_some());
        assert!(Rc::ptr_eq(
            (normalized.get(0))
                .as_ref()
                .expect("nonnull identity fixture"),
            &child
        ));
        plain.add(Some(node("later")));
        assert_eq!(normalized.count(), 1);
        assert_eq!(factory.create_list_calls.get(), 1);
        let empty = base.ensure_observable_dockables(None);
        assert!(empty.is_empty() && empty.as_notify_collection_changed().is_some());
        assert_eq!(factory.create_list_calls.get(), 2);
    }
}

#[test]
fn predicate_failures_finish_the_iterator_while_a_new_enumeration_can_succeed() {
    let factory = RecordingFactory::with_base();
    let parent = dock("parent");
    parent.visible_dockables().unwrap().add(Some(node("child")));
    factory.controls.add(Control::new(Some(parent.clone())));
    for all_controls in [false, true] {
        let fail = Rc::new(Cell::new(true));
        let flag = fail.clone();
        let predicate: DockablePredicate = Rc::new(move |_| {
            assert!(!flag.get(), "predicate failure");
            true
        });
        let query = if all_controls {
            factory.find(Some(predicate))
        } else {
            factory.find_in_dock(Some(parent.clone()), Some(predicate))
        };
        let mut iterator = query.iter();
        assert!(catch_unwind(AssertUnwindSafe(|| iterator.next())).is_err());
        fail.set(false);
        assert!(iterator.next().is_none());
        assert_eq!(ids(&query), ["parent", "child"]);
    }
}

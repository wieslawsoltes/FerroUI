//! Nullable reference arguments retain the original factory virtual slots and call order.
use crate::controls::{DocumentDock, ToolDock};
use crate::{impl_factory, Factory, FactoryImpl};
use ferrodock_model::{core::*, FactoryBase, FactoryBaseImpl};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

type Trace = Rc<RefCell<Vec<(&'static str, &'static str)>>>;
type Action = Rc<dyn Fn()>;
struct NullFactory {
    component: Factory,
    label: &'static str,
    trace: Trace,
    on_add: RefCell<Option<Action>>,
    on_active: RefCell<Option<Action>>,
    last_dock: RefCell<Option<Rc<dyn IDock>>>,
    abort_init: Cell<bool>,
}
impl NullFactory {
    fn new(label: &'static str, trace: Trace) -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            label,
            trace,
            on_add: Default::default(),
            on_active: Default::default(),
            last_dock: Default::default(),
            abort_init: Cell::new(false),
        })
    }
    fn record(&self, name: &'static str, item: Option<Rc<dyn IDockable>>) {
        assert!(item.is_none());
        self.trace.borrow_mut().push((self.label, name));
    }
}
impl FactoryBaseImpl for NullFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for NullFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn add_dockable(this: &Self, dock: Option<Rc<dyn IDock>>, item: Option<Rc<dyn IDockable>>) {
        this.record("add", item);
        this.last_dock.replace(dock);
        let action = this.on_add.borrow().clone();
        if let Some(action) = action {
            action();
        }
    }
    fn set_active_dockable(this: &Self, item: Option<Rc<dyn IDockable>>) {
        this.record("active", item);
        let action = this.on_active.borrow().clone();
        if let Some(action) = action {
            action();
        }
    }
    fn set_focused_dockable(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        item: Option<Rc<dyn IDockable>>,
    ) {
        this.record("focus", item);
        this.last_dock.replace(dock);
    }
    fn init_dockable(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        owner: Option<Rc<dyn IDockable>>,
    ) {
        this.record("init", item);
        this.last_dock
            .replace(owner.and_then(|item| item.as_dock()));
        assert!(!this.abort_init.get(), "init sentinel");
    }
}
impl_factory!(NullFactory);

#[test]
fn original_document_and_tool_bodies_forward_null_to_each_factory_override_in_order() {
    let trace = Trace::default();
    let factory = NullFactory::new("f", trace.clone());
    let document = DocumentDock::new();
    document.set_factory(Some(factory.clone()));
    document.set_document_factory(Some(Rc::new(|| None)));
    document.create_document().unwrap().execute(None);
    document.add_document_nullable(None);
    document.add_tool_nullable(None);
    let tool = ToolDock::new();
    tool.set_factory(Some(factory.clone()));
    tool.add_tool_nullable(None);
    let expected: Rc<dyn IDock> = tool.clone();
    assert!(Rc::ptr_eq(
        factory.last_dock.borrow().as_ref().unwrap(),
        &expected
    ));
    assert_eq!(
        &*trace.borrow(),
        &[("f", "add"), ("f", "active"), ("f", "focus")].repeat(4)
    );
    assert!(document.visible_dockables().is_none() && tool.visible_dockables().is_none());
}

#[test]
fn null_factory_calls_reread_replaced_or_removed_factory_after_each_virtual_call() {
    for remove_after in ["add", "active", "never"] {
        let trace = Trace::default();
        let first = NullFactory::new("first", trace.clone());
        let second = NullFactory::new("second", trace.clone());
        let dock = DocumentDock::new();
        dock.set_factory(Some(first.clone()));
        let weak = Rc::downgrade(&dock);
        let replacement = second.clone();
        first.on_add.replace(Some(Rc::new(move || {
            weak.upgrade()
                .unwrap()
                .set_factory(if remove_after == "add" {
                    None
                } else {
                    Some(replacement.clone())
                });
        })));
        if remove_after == "active" {
            let weak = Rc::downgrade(&dock);
            second.on_active.replace(Some(Rc::new(move || {
                weak.upgrade().unwrap().set_factory(None)
            })));
        }
        dock.add_document_nullable(None);
        let expected = match remove_after {
            "add" => vec![("first", "add")],
            "active" => vec![("first", "add"), ("second", "active")],
            _ => vec![("first", "add"), ("second", "active"), ("second", "focus")],
        };
        assert_eq!(&*trace.borrow(), &expected);
    }
}

#[test]
fn original_init_virtual_accepts_null_and_base_add_dispatches_it_before_graph_mutation() {
    let trace = Trace::default();
    let factory = NullFactory::new("f", trace.clone());
    let dock = DocumentDock::new();
    factory.init_dockable(None, Some(dock.clone()));
    factory.abort_init.set(true);
    let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        factory
            .component
            .base()
            .add_dockable(Some(dock.clone()), None);
    }))
    .unwrap_err();
    assert_eq!(error.downcast_ref::<&str>(), Some(&"init sentinel"));
    assert_eq!(&*trace.borrow(), &[("f", "init"), ("f", "init")]);
    assert!(dock.visible_dockables().is_none());
    let expected: Rc<dyn IDock> = dock;
    assert!(Rc::ptr_eq(
        factory.last_dock.borrow().as_ref().unwrap(),
        &expected
    ));
}

#[test]
fn default_init_and_activation_fail_at_source_dereference_before_mutating_graph() {
    let factory = Factory::new();
    for activate in [false, true] {
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if activate {
                factory.set_active_dockable(None);
            } else {
                factory.init_dockable(None, None);
            }
        }))
        .is_err());
        assert!(factory.current_dockable().is_none());
    }
}

#[test]
fn base_add_preserves_null_list_element_empty_state_and_original_added_event() {
    let factory = NullFactory::new("f", Trace::default());
    for (dock, opened) in [
        (DocumentDock::new() as Rc<dyn IDock>, 1),
        (crate::controls::RootDock::new() as Rc<dyn IDock>, 0),
    ] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let trace = events.clone();
        let subscription = factory.dockable_added(Rc::new(move |args| {
            trace.borrow_mut().push(args.dockable().is_none());
        }));
        factory
            .component
            .base()
            .add_dockable(Some(dock.clone()), None);
        let list = dock.visible_dockables().unwrap();
        assert_eq!(list.count(), 1);
        assert!(list.get(0).is_none() && list.contains(&None));
        assert_eq!(list.index_of(&None), Some(0));
        assert!(dock.is_empty());
        assert_eq!(dock.opened_dockables_count(), opened);
        assert_eq!(&*events.borrow(), &[true]);
        subscription.dispose();
    }
}

#[test]
fn null_list_entry_is_a_lazy_query_result_and_does_not_end_the_iterator() {
    let factory = Factory::new();
    let dock = DocumentDock::new();
    let child: Rc<dyn IDockable> = crate::controls::Document::new();
    let list = factory.create_list(vec![None, Some(child.clone())]);
    dock.set_visible_dockables(Some(list));
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    let query = factory.find_in_dock(
        Some(dock.clone()),
        Some(Rc::new(move |_| {
            seen.set(seen.get() + 1);
            true
        })),
    );
    assert_eq!(calls.get(), 0);
    let mut cursor = query.iter();
    let root: Rc<dyn IDockable> = dock;
    assert!(Rc::ptr_eq(&cursor.next().unwrap().unwrap(), &root));
    assert!(matches!(cursor.next(), Some(None)));
    assert!(Rc::ptr_eq(&cursor.next().unwrap().unwrap(), &child));
    assert!(cursor.next().is_none());
    assert_eq!(calls.get(), 3);
    assert_eq!(query.iter().count(), 3);
}

#[test]
fn nullable_plain_list_normalization_keeps_values_and_notifying_list_identity() {
    use ferrodock_model::collections::ListHandle;
    let factory = Factory::new();
    let child: Rc<dyn IDockable> = crate::controls::Document::new();
    let plain = ListHandle::from_plain(vec![None, Some(child.clone())]);
    let normalized = factory
        .base()
        .ensure_observable_dockables(Some(plain.clone()));
    assert!(!normalized.ptr_eq(&plain));
    assert!(normalized.as_notify_collection_changed().is_some());
    assert!(normalized.get(0).is_none());
    assert!(Rc::ptr_eq(&normalized.get(1).unwrap(), &child));
    let same = factory
        .base()
        .ensure_observable_dockables(Some(normalized.clone()));
    assert!(same.ptr_eq(&normalized));
}

#[test]
fn splitter_cleanup_keeps_null_content_and_uses_the_original_first_null_fallback() {
    let factory = Factory::new();
    let dock = crate::controls::RootDock::new();
    let splitter: Rc<dyn IDockable> = crate::controls::ProportionalDockSplitter::new();
    let child: Rc<dyn IDockable> = crate::controls::Document::new();
    let list = factory.create_list(vec![Some(splitter.clone()), None, Some(child.clone())]);
    dock.set_visible_dockables(Some(list.clone()));
    dock.set_active_dockable(Some(splitter));
    factory
        .base()
        .cleanup_orphaned_splitters(Some(dock.clone()));
    assert_eq!(list.count(), 2);
    assert!(list.get(0).is_none());
    assert!(Rc::ptr_eq(&list.get(1).unwrap(), &child));
    assert!(dock.active_dockable().is_none());
}

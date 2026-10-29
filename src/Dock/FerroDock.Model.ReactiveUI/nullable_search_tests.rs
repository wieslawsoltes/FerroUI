//! Original nullable roots/predicates, deferred queries and live window layout reads.
use crate::test_support::{TestControl, TestWindow};
use crate::{controls::*, impl_factory, Factory, FactoryImpl};
use ferrodock_model::{
    collections::ListHandle, controls::IRootDock, core::*, FactoryBase, FactoryBaseImpl,
};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
fn failure(action: impl FnOnce()) -> String {
    let e = catch_unwind(AssertUnwindSafe(action)).expect_err("source failure");
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| (*s).into()))
        .unwrap()
}
struct SearchCall {
    dock: Option<Rc<dyn IDock>>,
    predicate: Option<DockablePredicate>,
}
struct SearchFactory {
    component: Factory,
    result: Rc<dyn IDockable>,
    calls: RefCell<Vec<SearchCall>>,
}
impl SearchFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            result: Document::new(),
            calls: Default::default(),
        })
    }
}
impl FactoryBaseImpl for SearchFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for SearchFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn find_dockable(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        predicate: Option<DockablePredicate>,
    ) -> Option<Rc<dyn IDockable>> {
        this.calls.borrow_mut().push(SearchCall { dock, predicate });
        Some(this.result.clone())
    }
}
impl_factory!(SearchFactory);
fn sequence(
    root: &Rc<RootDock>,
    values: Vec<Option<Rc<dyn IRootDock>>>,
) -> (Rc<TestWindow>, Rc<Cell<usize>>) {
    let window = TestWindow::new();
    let reads = Rc::new(Cell::new(0));
    let count = reads.clone();
    let values = RefCell::new(VecDeque::from(values));
    window.window.layout_getter.replace(Some(Rc::new(move || {
        count.set(count.get() + 1);
        values.borrow_mut().pop_front().expect("source layout read")
    })));
    root.set_windows(Some(ListHandle::from_items(vec![Some(
        window.clone() as Rc<dyn IDockWindow>
    )])));
    (window, reads)
}
#[test]
fn immediate_null_root_invokes_predicate_before_return_or_visible_access_failure() {
    let f = Factory::new();
    for matches in [false, true] {
        let count = Rc::new(Cell::new(0));
        let calls = count.clone();
        let predicate: DockablePredicate = Rc::new(move |item| {
            assert!(item.is_none());
            calls.set(calls.get() + 1);
            matches
        });
        if matches {
            assert!(f.find_dockable(None, Some(predicate)).is_none());
        } else {
            assert!(failure(|| {
                f.find_dockable(None, Some(predicate));
            })
            .contains("dock.VisibleDockables"));
        }
        assert_eq!(count.get(), 1);
    }
    for dock in [None, Some(RootDock::new() as Rc<dyn IDock>)] {
        assert!(failure(|| {
            f.find_dockable(dock, None);
        })
        .contains("predicate invocation"));
    }
}
#[test]
fn null_root_lazy_query_yields_actual_null_then_faults_and_repeats_without_early_calls() {
    let f = Factory::new();
    let count = Rc::new(Cell::new(0));
    let calls = count.clone();
    let query = f.find_in_dock(
        None,
        Some(Rc::new(move |item| {
            assert!(item.is_none());
            calls.set(calls.get() + 1);
            true
        })),
    );
    assert_eq!(count.get(), 0);
    let mut first = query.iter();
    assert_eq!(count.get(), 0);
    assert!(matches!(first.next(), Some(None)));
    assert_eq!(count.get(), 1);
    assert!(failure(|| {
        first.next();
    })
    .contains("dock.VisibleDockables"));
    assert!(first.next().is_none());
    assert_eq!(count.get(), 1);
    let mut second = query.iter();
    assert!(matches!(second.next(), Some(None)));
    assert_eq!(count.get(), 2);
}
#[test]
fn null_predicate_is_deferred_and_all_controls_search_skips_absent_layouts_before_calling_it() {
    let f = Factory::new();
    for dock in [None, Some(RootDock::new() as Rc<dyn IDock>)] {
        let query = f.find_in_dock(dock, None);
        let mut cursor = query.iter();
        assert!(failure(|| {
            cursor.next();
        })
        .contains("predicate invocation"));
        assert!(cursor.next().is_none());
    }
    let query = f.find(None);
    assert!(query.iter().next().is_none());
    f.dock_controls().add(TestControl::new(None));
    assert!(query.iter().next().is_none());
    f.dock_controls()
        .add(TestControl::new(Some(RootDock::new())));
    let mut cursor = query.iter();
    assert!(failure(|| {
        cursor.next();
    })
    .contains("predicate invocation"));
    assert!(cursor.next().is_none());
}
#[test]
fn throwing_predicate_precedes_null_root_access_and_keeps_original_panic_message() {
    let f = Factory::new();
    let count = Rc::new(Cell::new(0));
    let calls = count.clone();
    let predicate: DockablePredicate = Rc::new(move |item| {
        assert!(item.is_none());
        calls.set(calls.get() + 1);
        panic!("predicate aborted")
    });
    assert_eq!(
        failure(|| {
            f.find_dockable(None, Some(predicate.clone()));
        }),
        "predicate aborted"
    );
    let query = f.find_in_dock(None, Some(predicate));
    assert_eq!(count.get(), 1);
    let mut cursor = query.iter();
    assert_eq!(
        failure(|| {
            cursor.next();
        }),
        "predicate aborted"
    );
    assert_eq!(count.get(), 2);
    assert!(cursor.next().is_none());
}
#[test]
fn actual_virtual_override_receives_null_callable_and_retains_original_some_callable_identity() {
    let f = SearchFactory::new();
    assert!(Rc::ptr_eq(&f.find_dockable(None, None).unwrap(), &f.result));
    let predicate: DockablePredicate = Rc::new(|_| panic!("override must not evaluate predicate"));
    assert!(Rc::ptr_eq(
        &f.find_dockable(None, Some(predicate.clone())).unwrap(),
        &f.result
    ));
    let calls = f.calls.borrow();
    assert_eq!(calls.len(), 2);
    assert!(calls[0].dock.is_none() && calls[0].predicate.is_none());
    assert!(calls[1].dock.is_none());
    assert!(Rc::ptr_eq(calls[1].predicate.as_ref().unwrap(), &predicate));
}
#[test]
fn live_layout_predicate_and_return_reads_preserve_null_without_searching_later_windows() {
    for null_return in [false, true] {
        let f = Factory::new();
        let root = RootDock::new();
        let a = RootDock::new();
        let b = RootDock::new();
        let (window, reads) = sequence(
            &root,
            vec![
                Some(a),
                None,
                if null_return { None } else { Some(b.clone()) },
            ],
        );
        let last = TestWindow::new();
        let visited = Rc::new(Cell::new(0));
        let count = visited.clone();
        last.window.layout_getter.replace(Some(Rc::new(move || {
            count.set(count.get() + 1);
            None
        })));
        root.windows().unwrap().add(Some(last));
        let values = Rc::new(RefCell::new(Vec::new()));
        let observed = values.clone();
        let result = f.find_dockable(
            Some(root.clone()),
            Some(Rc::new(move |item| {
                let null = item.is_none();
                observed.borrow_mut().push(item);
                null
            })),
        );
        assert_eq!(reads.get(), 3);
        assert_eq!(visited.get(), 0);
        assert_eq!(values.borrow().len(), 2);
        assert!(values.borrow()[1].is_none());
        if null_return {
            assert!(result.is_none());
        } else {
            assert!(Rc::ptr_eq(&result.unwrap(), &(b as Rc<dyn IDockable>)));
        }
        drop(window);
    }
}
#[test]
fn recursive_virtual_search_receives_live_null_layout_with_original_predicate() {
    let f = SearchFactory::new();
    let root = RootDock::new();
    let a = RootDock::new();
    let (_window, reads) = sequence(&root, vec![Some(a.clone()), Some(a), None]);
    let count = Rc::new(Cell::new(0));
    let observed = count.clone();
    let predicate: DockablePredicate = Rc::new(move |_| {
        observed.set(observed.get() + 1);
        false
    });
    let result = f
        .component
        .base()
        .find_dockable(Some(root), Some(predicate.clone()))
        .unwrap();
    assert!(Rc::ptr_eq(&result, &f.result));
    assert_eq!(reads.get(), 3);
    assert_eq!(count.get(), 2);
    let calls = f.calls.borrow();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].dock.is_none());
    assert!(Rc::ptr_eq(calls[0].predicate.as_ref().unwrap(), &predicate));
}
#[test]
fn recursive_default_search_invokes_null_predicate_argument_before_its_own_null_root_failure() {
    let f = Factory::new();
    let root = RootDock::new();
    let a = RootDock::new();
    let (_window, reads) = sequence(&root, vec![Some(a.clone()), Some(a), None]);
    let nulls = Rc::new(RefCell::new(Vec::new()));
    let values = nulls.clone();
    let error = failure(|| {
        f.find_dockable(
            Some(root),
            Some(Rc::new(move |item| {
                values.borrow_mut().push(item.is_none());
                false
            })),
        );
    });
    assert!(error.contains("dock.VisibleDockables"));
    assert_eq!(&*nulls.borrow(), &[false, false, true]);
    assert_eq!(reads.get(), 3);
}
#[test]
fn fixed_lazy_find_uses_original_query_engine_without_virtual_single_search_dispatch() {
    let f = SearchFactory::new();
    let query = f.find_in_dock(None, Some(Rc::new(|_| true)));
    let mut cursor = query.iter();
    assert!(matches!(cursor.next(), Some(None)));
    assert!(failure(|| {
        cursor.next();
    })
    .contains("dock.VisibleDockables"));
    let root = RootDock::new();
    f.dock_controls().add(TestControl::new(Some(root.clone())));
    let query = f.find(Some(Rc::new(|_| true)));
    let mut cursor = query.iter();
    assert!(Rc::ptr_eq(
        &cursor.next().unwrap().unwrap(),
        &(root as Rc<dyn IDockable>)
    ));
    assert!(cursor.next().is_none());
    assert!(f.calls.borrow().is_empty());
}

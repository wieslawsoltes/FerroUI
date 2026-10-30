//! Native nullable search roots/callables preserve source virtual and iterator contracts.
use crate::markup_extension_tests::run;
use ferrodock_model::{core::*, FactoryBase, FactoryBaseImpl};
use ferrodock_model_ferro::{
    DockablePredicateReference, DockableQueryEnumeratorReference, DockableQueryReference,
    FactoryBaseReference, IFactoryMetadata,
};
use ferrodock_model_reactiveui::{
    controls::{Document, RootDock},
    impl_factory, Factory, FactoryImpl,
};
use ferroui_base::{
    data::core::ValueType,
    metadata::{from_markup_value, MarkupType, MarkupTyped},
    BoxedValue,
};
use std::{
    cell::{Cell, RefCell},
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
fn views<T: IFactory + FactoryBaseImpl + 'static>(
    f: Rc<T>,
) -> [(BoxedValue, &'static MarkupType); 2] {
    [
        (
            Rc::new(f.clone() as Rc<dyn IFactory>) as BoxedValue,
            <dyn IFactoryMetadata as MarkupTyped>::MARKUP,
        ),
        (
            Rc::new(FactoryBaseReference::new(f)) as BoxedValue,
            FactoryBaseReference::MARKUP,
        ),
    ]
}
#[test]
fn native_direct_search_preserves_null_root_predicate_order_and_actual_missing_callable() {
    run(|| {
        for (receiver, metadata) in views(Factory::new()) {
            let method = metadata.find_methods("FindDockable").next().unwrap();
            assert_eq!(
                (method.parameters[0])(),
                ValueType::of::<Option<Rc<dyn IDock>>>()
            );
            assert_eq!(
                (method.parameters[1])(),
                ValueType::of::<Option<DockablePredicateReference>>()
            );
            for matched in [false, true] {
                let count = Rc::new(Cell::new(0));
                let calls = count.clone();
                let predicate = DockablePredicateReference::new(Rc::new(move |item| {
                    assert!(item.is_none());
                    calls.set(calls.get() + 1);
                    matched
                }));
                let args = [
                    Some(receiver.clone()),
                    None,
                    Some(Rc::new(predicate) as BoxedValue),
                ];
                if matched {
                    assert!((method.invoke)(&args).unwrap().is_none());
                } else {
                    assert!(failure(|| {
                        (method.invoke)(&args).unwrap();
                    })
                    .contains("dock.VisibleDockables"));
                }
                assert_eq!(count.get(), 1);
            }
            assert!(failure(|| {
                (method.invoke)(&[Some(receiver), None, None]).unwrap();
            })
            .contains("predicate invocation"));
        }
    });
}
#[test]
fn native_fixed_queries_defer_null_function_and_preserve_null_current_fault_repeat_and_dispose() {
    run(|| {
        for (receiver, metadata) in views(Factory::new()) {
            let method = metadata
                .find_methods("Find")
                .find(|m| m.parameters.len() == 2)
                .unwrap();
            let count = Rc::new(Cell::new(0));
            let calls = count.clone();
            let predicate = DockablePredicateReference::new(Rc::new(move |item| {
                assert!(item.is_none());
                calls.set(calls.get() + 1);
                true
            }));
            let value =
                (method.invoke)(&[Some(receiver.clone()), None, Some(Rc::new(predicate))]).unwrap();
            assert_eq!(count.get(), 0);
            let query = from_markup_value::<DockableQueryReference>(&value).unwrap();
            let cursor = query.get_enumerator();
            assert_eq!(count.get(), 0);
            assert!(cursor.current().is_none());
            assert!(cursor.move_next());
            assert_eq!(count.get(), 1);
            assert!(cursor.current().is_none());
            assert!(failure(|| {
                cursor.move_next();
            })
            .contains("dock.VisibleDockables"));
            assert!(!cursor.move_next());
            assert!(cursor.current().is_none());
            cursor.dispose();
            assert!(!cursor.move_next());
            assert!(cursor.reset().is_err());
            let again = query.get_enumerator();
            assert!(again.move_next());
            assert_eq!(count.get(), 2);
            again.dispose();
            let value = (method.invoke)(&[Some(receiver.clone()), None, None]).unwrap();
            let cursor = from_markup_value::<DockableQueryReference>(&value)
                .unwrap()
                .get_enumerator();
            assert!(failure(|| {
                cursor.move_next();
            })
            .contains("predicate invocation"));
            assert!(!cursor.move_next());
            let all = metadata
                .find_methods("Find")
                .find(|m| m.parameters.len() == 1)
                .unwrap();
            let value = (all.invoke)(&[Some(receiver), None]).unwrap();
            assert!(!from_markup_value::<DockableQueryReference>(&value)
                .unwrap()
                .get_enumerator()
                .move_next());
        }
    });
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
#[test]
fn both_native_views_preserve_actual_nullable_override_inputs_and_original_predicate_result_identity(
) {
    run(|| {
        let f = SearchFactory::new();
        let predicate: DockablePredicate =
            Rc::new(|_| panic!("source override does not invoke predicate"));
        for (receiver, metadata) in views(f.clone()) {
            let method = metadata.find_methods("FindDockable").next().unwrap();
            for nonnull in [false, true] {
                let arg = if nonnull {
                    Some(Rc::new(DockablePredicateReference::new(predicate.clone())) as BoxedValue)
                } else {
                    None
                };
                let value = (method.invoke)(&[Some(receiver.clone()), None, arg]).unwrap();
                assert!(Rc::ptr_eq(
                    &from_markup_value::<Rc<dyn IDockable>>(&value).unwrap(),
                    &f.result
                ));
                let calls = f.calls.borrow();
                let call = calls.last().unwrap();
                assert!(call.dock.is_none());
                if nonnull {
                    assert!(Rc::ptr_eq(call.predicate.as_ref().unwrap(), &predicate));
                } else {
                    assert!(call.predicate.is_none());
                }
            }
        }
        assert_eq!(f.calls.borrow().len(), 4);
    });
}
#[test]
fn native_cursor_reentrant_move_next_returns_false_with_original_previous_current() {
    run(|| {
        let f = Factory::new();
        let root = RootDock::new();
        let child: Rc<dyn IDockable> = f.create_document();
        root.set_visible_dockables(Some(
            f.create_list(vec![Some(child.clone() as Rc<dyn IDockable>)]),
        ));
        let holder: Rc<RefCell<Option<DockableQueryEnumeratorReference>>> = Default::default();
        let input = holder.clone();
        let values = Rc::new(RefCell::new(Vec::new()));
        let log = values.clone();
        let query = f.find_in_dock(
            Some(root.clone()),
            Some(Rc::new(move |_| {
                let cursor = input.borrow().as_ref().unwrap().clone();
                let current = cursor.current();
                let advanced = cursor.move_next();
                log.borrow_mut().push((advanced, current));
                true
            })),
        );
        let cursor = DockableQueryReference::new(query).get_enumerator();
        holder.replace(Some(cursor.clone()));
        assert!(cursor.move_next());
        assert!(Rc::ptr_eq(
            &cursor.current().unwrap(),
            &(root.clone() as Rc<dyn IDockable>)
        ));
        assert!(cursor.move_next());
        assert!(Rc::ptr_eq(&cursor.current().unwrap(), &child));
        assert!(!cursor.move_next());
        let values = values.borrow();
        assert_eq!(values.len(), 2);
        assert!(!values[0].0 && values[0].1.is_none());
        assert!(!values[1].0);
        assert!(Rc::ptr_eq(
            values[1].1.as_ref().unwrap(),
            &(root as Rc<dyn IDockable>)
        ));
        holder.take();
        cursor.dispose();
    });
}

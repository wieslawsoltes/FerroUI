//! Native factory views preserve original nullable focus/activation declarations and dispatch.
use crate::markup_extension_tests::run;
use ferrodock_model::{controls::IRootDock, core::*, FactoryBase, FactoryBaseImpl};
use ferrodock_model_ferro::{FactoryBaseReference, IFactoryMetadata};
use ferrodock_model_reactiveui::{impl_factory, Factory, FactoryImpl};
use ferroui_base::{data::core::ValueType, metadata::MarkupTyped, BoxedValue};
use std::{
    cell::RefCell,
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
struct NullFocusFactory {
    component: Factory,
    calls: RefCell<Vec<&'static str>>,
}
impl NullFocusFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            calls: Default::default(),
        })
    }
}
impl FactoryBaseImpl for NullFocusFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for NullFocusFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn init_active_dockable(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        owner: Option<Rc<dyn IDock>>,
    ) {
        assert!(item.is_none() && owner.is_none());
        this.calls.borrow_mut().push("init");
    }
    fn set_focused_dockable(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        item: Option<Rc<dyn IDockable>>,
    ) {
        assert!(dock.is_none() && item.is_none());
        this.calls.borrow_mut().push("focus");
    }
    fn find_root(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        predicate: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>> {
        assert!(item.is_none() && predicate.is_none());
        this.calls.borrow_mut().push("lookup");
        None
    }
}
impl_factory!(NullFocusFactory);
#[test]
fn both_native_factory_views_forward_null_to_actual_focus_init_and_fixed_activation_lookup() {
    run(|| {
        let f = NullFocusFactory::new();
        for (receiver, metadata) in [
            (
                Rc::new(f.clone() as Rc<dyn IFactory>) as BoxedValue,
                <dyn IFactoryMetadata as MarkupTyped>::MARKUP,
            ),
            (
                Rc::new(FactoryBaseReference::new(f.clone())) as BoxedValue,
                FactoryBaseReference::MARKUP,
            ),
        ] {
            for (name, types) in [
                (
                    "InitActiveDockable",
                    vec![
                        ValueType::of::<Option<Rc<dyn IDockable>>>(),
                        ValueType::of::<Option<Rc<dyn IDock>>>(),
                    ],
                ),
                (
                    "SetFocusedDockable",
                    vec![
                        ValueType::of::<Option<Rc<dyn IDock>>>(),
                        ValueType::of::<Option<Rc<dyn IDockable>>>(),
                    ],
                ),
                (
                    "ActivateWindow",
                    vec![ValueType::of::<Option<Rc<dyn IDockable>>>()],
                ),
            ] {
                let method = metadata
                    .find_methods(name)
                    .find(|m| {
                        m.parameters.len() == types.len()
                            && m.parameters
                                .iter()
                                .zip(&types)
                                .all(|(actual, expected)| actual() == *expected)
                    })
                    .unwrap();
                let mut args = vec![Some(receiver.clone())];
                args.resize(types.len() + 1, None);
                assert!((method.invoke)(&args).unwrap().is_none());
            }
        }
        assert_eq!(
            &*f.calls.borrow(),
            &["init", "focus", "lookup", "init", "focus", "lookup"]
        );
    });
}
#[test]
fn native_default_null_owner_raises_active_event_before_original_selection_failure() {
    run(|| {
        let f = Factory::new();
        let events = Rc::new(RefCell::new(Vec::new()));
        let received = events.clone();
        f.active_dockable_changed(Rc::new(move |e| received.borrow_mut().push(e.dockable())));
        for (receiver, metadata) in [
            (
                Rc::new(f.clone() as Rc<dyn IFactory>) as BoxedValue,
                <dyn IFactoryMetadata as MarkupTyped>::MARKUP,
            ),
            (
                Rc::new(FactoryBaseReference::new(f.clone())) as BoxedValue,
                FactoryBaseReference::MARKUP,
            ),
        ] {
            for (name, count, expected) in [
                ("InitActiveDockable", 2, "owner.VisibleDockables"),
                ("SetFocusedDockable", 2, "source dockable.Owner"),
                ("ActivateWindow", 1, "source dockable.Owner"),
            ] {
                let method = metadata
                    .find_methods(name)
                    .find(|m| m.parameters.len() == count)
                    .unwrap();
                let mut args = vec![Some(receiver.clone())];
                args.resize(count + 1, None);
                let error = catch_unwind(AssertUnwindSafe(|| (method.invoke)(&args)))
                    .expect_err("original access failure");
                let message = error
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| error.downcast_ref::<&str>().copied())
                    .unwrap();
                assert!(message.contains(expected), "{message}");
            }
        }
        assert_eq!(events.borrow().len(), 2);
        assert!(events.borrow().iter().all(Option::is_none));
    });
}

//! Native nullable movement and capability calls use the original engine and slots.
use crate::markup_extension_tests::run;
use ferrodock_model::{controls::IRootDock, core::*, FactoryBase, FactoryBaseImpl};
use ferrodock_model_ferro::{
    DockCapabilityResolverMetadata, FactoryBaseReference, IFactoryMetadata,
};
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock, RootDock},
    impl_factory, Factory, FactoryImpl,
};
use ferroui_base::{data::core::ValueType, metadata::MarkupTyped, BoxedValue};
use std::{
    cell::RefCell,
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
#[test]
fn both_native_factory_views_move_and_swap_null_in_the_original_list() {
    run(|| {
        let f = Factory::new();
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
            let dock = DocumentDock::new();
            let item: Rc<dyn IDockable> = Document::new();
            let list = f.create_list(vec![None, Some(item.clone())]);
            dock.set_visible_dockables(Some(list.clone()));
            for name in ["MoveDockable", "SwapDockable"] {
                let method = metadata
                    .find_methods(name)
                    .find(|m| m.parameters.len() == 3)
                    .unwrap();
                assert_eq!(
                    (method.parameters[0])(),
                    ValueType::of::<Option<Rc<dyn IDock>>>()
                );
                (method.invoke)(&[
                    Some(receiver.clone()),
                    Some(Rc::new(dock.clone() as Rc<dyn IDock>)),
                    None,
                    Some(Rc::new(item.clone())),
                ])
                .unwrap();
                assert!(list.ptr_eq(&dock.visible_dockables().unwrap()));
                let index = usize::from(name == "MoveDockable");
                assert!(list.get(index).is_none());
                assert!(Rc::ptr_eq(&list.get(1 - index).unwrap(), &item));
            }
            assert!(Rc::ptr_eq(&dock.active_dockable().unwrap(), &item));
        }
    });
}
struct MovementFactory {
    component: Factory,
    calls: RefCell<Vec<&'static str>>,
    root: Rc<dyn IRootDock>,
}
impl MovementFactory {
    fn new() -> Rc<Self> {
        let f = Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            calls: Default::default(),
            root: RootDock::new(),
        });
        f.root
            .set_left_pinned_dockables(Some(f.create_list(vec![None])));
        f
    }
}
impl FactoryBaseImpl for MovementFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for MovementFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn move_dockable(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        source: Option<Rc<dyn IDockable>>,
        target: Option<Rc<dyn IDockable>>,
    ) {
        assert!(dock.is_none() && source.is_none() && target.is_none());
        this.calls.borrow_mut().push("move3");
    }
    fn move_dockable_between_docks(
        this: &Self,
        source_dock: Option<Rc<dyn IDock>>,
        target_dock: Option<Rc<dyn IDock>>,
        source: Option<Rc<dyn IDockable>>,
        target: Option<Rc<dyn IDockable>>,
    ) {
        assert!(
            source_dock.is_none() && target_dock.is_none() && source.is_none() && target.is_none()
        );
        this.calls.borrow_mut().push("move4");
    }
    fn swap_dockable(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        source: Option<Rc<dyn IDockable>>,
        target: Option<Rc<dyn IDockable>>,
    ) {
        assert!(dock.is_none() && source.is_none() && target.is_none());
        this.calls.borrow_mut().push("swap3");
    }
    fn swap_dockable_between_docks(
        this: &Self,
        source_dock: Option<Rc<dyn IDock>>,
        target_dock: Option<Rc<dyn IDock>>,
        source: Option<Rc<dyn IDockable>>,
        target: Option<Rc<dyn IDockable>>,
    ) {
        assert!(
            source_dock.is_none() && target_dock.is_none() && source.is_none() && target.is_none()
        );
        this.calls.borrow_mut().push("swap4");
    }
    fn move_pinned_dockable(
        this: &Self,
        source: Option<Rc<dyn IDockable>>,
        target: Option<Rc<dyn IDockable>>,
    ) {
        assert!(source.is_none() && target.is_none());
        this.calls.borrow_mut().push("pinned");
    }
    fn pin_dockable(this: &Self, item: Option<Rc<dyn IDockable>>) {
        assert!(item.is_none());
        this.calls.borrow_mut().push("pin");
    }
    fn find_root(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        _: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>> {
        assert!(item.is_none());
        this.calls.borrow_mut().push("root");
        Some(this.root.clone())
    }
}
impl_factory!(MovementFactory);
#[test]
fn both_native_factory_views_forward_null_slots_and_keep_unpin_fixed_dispatch() {
    run(|| {
        let f = MovementFactory::new();
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
            for (name, count) in [
                ("MoveDockable", 3),
                ("MoveDockable", 4),
                ("SwapDockable", 3),
                ("SwapDockable", 4),
                ("MovePinnedDockable", 2),
                ("PinDockable", 1),
                ("UnpinDockable", 1),
            ] {
                let first = if name == "MoveDockable" || name == "SwapDockable" {
                    ValueType::of::<Option<Rc<dyn IDock>>>()
                } else {
                    ValueType::of::<Option<Rc<dyn IDockable>>>()
                };
                let method = metadata
                    .find_methods(name)
                    .find(|m| m.parameters.len() == count && (m.parameters[0])() == first)
                    .unwrap();
                let mut values = vec![Some(receiver.clone())];
                values.resize(count + 1, None);
                (method.invoke)(&values).unwrap();
            }
        }
        assert_eq!(
            &*f.calls.borrow(),
            &["move3", "move4", "swap3", "swap4", "pinned", "pin", "root", "pin"].repeat(2)
        );
        let pins = f.root.left_pinned_dockables().unwrap();
        assert_eq!(pins.count(), 1);
        assert!(pins.get(0).is_none());
    });
}
#[test]
fn native_capability_nullable_arguments_reach_original_default_accesses() {
    run(|| {
        let capability = Some(Rc::new(DockCapability::Pin) as BoxedValue);
        for (name, arguments, access) in [
            (
                "Evaluate",
                vec![None, capability.clone()],
                "ResolveOperationDock dockable.Owner",
            ),
            (
                "Evaluate",
                vec![None, capability.clone(), None],
                "capability dockable flag",
            ),
            (
                "IsEnabled",
                vec![None, capability.clone()],
                "capability dockable flag",
            ),
            (
                "IsEnabled",
                vec![None, capability, None],
                "capability dockable flag",
            ),
            (
                "ResolveOperationDock",
                vec![None],
                "ResolveOperationDock dockable.Owner",
            ),
            (
                "ResolveDropTargetDock",
                vec![None],
                "ResolveDropTargetDock target.Owner",
            ),
        ] {
            let method = DockCapabilityResolverMetadata::MARKUP
                .find_methods(name)
                .find(|m| m.parameters.len() == arguments.len())
                .unwrap();
            assert_eq!(
                (method.parameters[0])(),
                ValueType::of::<Option<Rc<dyn IDockable>>>()
            );
            let error = catch_unwind(AssertUnwindSafe(|| {
                (method.invoke)(&arguments).unwrap();
            }))
            .expect_err("original null failure");
            let message = error
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied())
                .unwrap();
            assert!(message.contains(access), "{name}: {message}");
        }
    });
}

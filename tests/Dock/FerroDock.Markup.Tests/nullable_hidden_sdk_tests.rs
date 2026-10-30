//! Native nullable hidden-item contracts and lazy ID helpers on actual controls.
use crate::markup_extension_tests::run;
use ferrodock_ferro::controls::DockControl;
use ferrodock_model::{controls::IRootDock, core::*, FactoryBase, FactoryBaseImpl};
use ferrodock_model_ferro::{FactoryBaseReference, IFactoryMetadata};
use ferrodock_model_reactiveui::{
    controls::{RootDock, Tool},
    impl_factory, Factory, FactoryImpl,
};
use ferroui_base::{data::core::ValueType, metadata::MarkupTyped, BoxedValue};
use std::{
    cell::{Cell, RefCell},
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
struct HiddenFactory {
    component: Factory,
    root: Rc<dyn IRootDock>,
    delegate: Cell<bool>,
    calls: RefCell<Vec<&'static str>>,
}
impl HiddenFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            root: RootDock::new(),
            delegate: Cell::new(false),
            calls: Default::default(),
        })
    }
}
impl FactoryBaseImpl for HiddenFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for HiddenFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn hide_dockable(this: &Self, item: Option<Rc<dyn IDockable>>) {
        assert!(item.is_none());
        this.calls.borrow_mut().push("hide");
    }
    fn restore_dockable(this: &Self, item: Option<Rc<dyn IDockable>>) {
        assert!(item.is_none());
        this.calls.borrow_mut().push("restore");
        if this.delegate.get() {
            this.component.base().restore_dockable(item);
        }
    }
    fn find_root(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        _: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>> {
        assert!(item.is_none());
        Some(this.root.clone())
    }
}
impl_factory!(HiddenFactory);
fn failure(action: impl FnOnce()) -> String {
    let e = catch_unwind(AssertUnwindSafe(action)).expect_err("source failure");
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| (*s).into()))
        .unwrap()
}
#[test]
fn both_native_hidden_views_pass_actual_null_to_original_virtual_overrides() {
    run(|| {
        let f = HiddenFactory::new();
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
            for name in ["HideDockable", "RestoreDockable"] {
                let method = metadata
                    .find_methods(name)
                    .find(|m| {
                        m.parameters.len() == 1
                            && (m.parameters[0])() == ValueType::of::<Option<Rc<dyn IDockable>>>()
                    })
                    .unwrap();
                (method.invoke)(&[Some(receiver.clone()), None]).unwrap();
            }
        }
        assert_eq!(&*f.calls.borrow(), &["hide", "restore"].repeat(2));
    });
}
#[test]
fn native_restore_null_preserves_shared_removal_and_event_before_original_owner_failure() {
    run(|| {
        let f = HiddenFactory::new();
        f.delegate.set(true);
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
            let list = f.create_list(vec![None]);
            f.root.set_hidden_dockables(Some(list.clone()));
            let observed = list.clone();
            let called = Rc::new(Cell::new(false));
            let flag = called.clone();
            let event = f.dockable_restored(Rc::new(move |args| {
                assert!(args.dockable().is_none());
                assert_eq!(observed.count(), 0);
                flag.set(true);
            }));
            let method = metadata
                .find_methods("RestoreDockable")
                .find(|m| {
                    m.parameters.len() == 1
                        && (m.parameters[0])() == ValueType::of::<Option<Rc<dyn IDockable>>>()
                })
                .unwrap();
            assert!(failure(|| {
                (method.invoke)(&[Some(receiver), None]).unwrap();
            })
            .contains("restore dockable.OriginalOwner"));
            assert!(called.get());
            assert_eq!(list.count(), 0);
            assert!(list.ptr_eq(&f.root.hidden_dockables().unwrap()));
            event.dispose();
        }
    });
}
#[test]
fn native_id_helpers_keep_first_null_failure_and_lazy_root_order_on_actual_controls() {
    run(|| {
        for base in [false, true] {
            let f = Factory::new();
            let root = RootDock::new();
            root.set_id("root".into());
            let wanted = Tool::new();
            wanted.set_id("wanted".into());
            let visible = f.create_list(vec![None, Some(wanted as Rc<dyn IDockable>)]);
            root.set_visible_dockables(Some(visible.clone()));
            let later = RootDock::new();
            let later_item = Tool::new();
            later_item.set_id("wanted".into());
            let later_hidden = f.create_list(vec![Some(later_item.clone() as Rc<dyn IDockable>)]);
            later.set_visible_dockables(Some(
                f.create_list(vec![Some(later_item as Rc<dyn IDockable>)]),
            ));
            later.set_hidden_dockables(Some(later_hidden.clone()));
            let first_control = DockControl::new();
            first_control.set_initialize_layout(false);
            first_control.set_initialize_factory(false);
            first_control.set_layout(Some(root.clone()));
            let next_control = DockControl::new();
            next_control.set_initialize_layout(false);
            next_control.set_initialize_factory(false);
            next_control.set_layout(Some(later));
            f.dock_controls()
                .add(DockControl::as_dock_control(first_control.clone()));
            f.dock_controls()
                .add(DockControl::as_dock_control(next_control.clone()));
            let (receiver, metadata) = if base {
                (
                    Rc::new(FactoryBaseReference::new(f.clone())) as BoxedValue,
                    FactoryBaseReference::MARKUP,
                )
            } else {
                (
                    Rc::new(f.clone() as Rc<dyn IFactory>) as BoxedValue,
                    <dyn IFactoryMetadata as MarkupTyped>::MARKUP,
                )
            };
            let hide = metadata
                .find_methods("HideDockable")
                .find(|m| {
                    m.parameters.len() == 1 && (m.parameters[0])() == ValueType::of::<String>()
                })
                .unwrap();
            assert!(failure(|| {
                (hide.invoke)(&[
                    Some(receiver.clone()),
                    Some(Rc::new(String::from("wanted"))),
                ])
                .unwrap();
            })
            .contains("predicate item.Id"));
            assert_eq!(visible.count(), 2);
            assert!(root.hidden_dockables().is_none());
            assert_eq!(later_hidden.count(), 1);
            let hidden_item = Tool::new();
            hidden_item.set_id("wanted".into());
            let hidden = f.create_list(vec![None, Some(hidden_item as Rc<dyn IDockable>)]);
            root.set_hidden_dockables(Some(hidden.clone()));
            let restore = metadata
                .find_methods("RestoreDockable")
                .find(|m| {
                    m.parameters.len() == 1 && (m.parameters[0])() == ValueType::of::<String>()
                })
                .unwrap();
            assert!(failure(|| {
                (restore.invoke)(&[Some(receiver), Some(Rc::new(String::from("wanted")))]).unwrap();
            })
            .contains("item.Id"));
            assert_eq!(hidden.count(), 2);
            assert!(hidden.ptr_eq(&root.hidden_dockables().unwrap()));
            assert_eq!(later_hidden.count(), 1);
            first_control.set_layout(None);
            next_control.set_layout(None);
        }
    });
}

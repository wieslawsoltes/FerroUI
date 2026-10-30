//! Native preview declarations and genuine nullable Init recovery after model reentry.
use crate::markup_extension_tests::run;
use ferrodock_model::{
    controls::{IRootDock, IToolDock},
    core::*,
    FactoryBase, FactoryBaseImpl,
};
use ferrodock_model_ferro::{FactoryBaseReference, IFactoryMetadata};
use ferrodock_model_reactiveui::{
    controls::{RootDock, Tool, ToolDock},
    impl_factory, Factory, FactoryImpl,
};
use ferroui_base::{data::core::ValueType, metadata::MarkupTyped, BoxedValue};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::{
    cell::{Cell, RefCell},
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
struct PreviewFactory {
    component: Factory,
    root: RefCell<Option<Rc<dyn IRootDock>>>,
    lookups: Cell<usize>,
    null_inits: Cell<usize>,
    replacement: RefCell<Option<Rc<dyn IToolDock>>>,
}
impl PreviewFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            root: Default::default(),
            lookups: Cell::new(0),
            null_inits: Cell::new(0),
            replacement: Default::default(),
        })
    }
}
impl FactoryBaseImpl for PreviewFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for PreviewFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn find_root(
        this: &Self,
        _: Option<Rc<dyn IDockable>>,
        _: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>> {
        this.lookups.set(this.lookups.get() + 1);
        this.root.borrow().clone()
    }
    fn init_dockable(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        owner: Option<Rc<dyn IDockable>>,
    ) {
        assert!(item.is_none());
        let root = this.root.borrow().clone().unwrap();
        assert!(Rc::ptr_eq(
            &owner.unwrap(),
            &(root.clone() as Rc<dyn IDockable>)
        ));
        this.null_inits.set(this.null_inits.get() + 1);
        root.set_pinned_dock(this.replacement.borrow().clone());
    }
}
impl_factory!(PreviewFactory);
#[test]
fn native_preview_views_distinguish_typed_null_lookup_from_object_null_noop() {
    run(|| {
        let f = PreviewFactory::new();
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
            for name in ["PreviewPinnedDockable", "TogglePreviewPinnedDockable"] {
                for parameter_type in [
                    ValueType::of::<Option<Rc<dyn IDockable>>>(),
                    ValueType::of::<Option<BoxedValue>>(),
                ] {
                    let method = metadata
                        .find_methods(name)
                        .find(|m| m.parameters.len() == 1 && (m.parameters[0])() == parameter_type)
                        .unwrap();
                    (method.invoke)(&[Some(receiver.clone()), None]).unwrap();
                }
            }
            let hide = metadata
                .find_methods("HidePreviewingDockables")
                .next()
                .unwrap();
            assert_eq!(
                (hide.parameters[0])(),
                ValueType::of::<Option<Rc<dyn IRootDock>>>()
            );
            let error = catch_unwind(AssertUnwindSafe(|| {
                (hide.invoke)(&[Some(receiver), None]).unwrap();
            }))
            .expect_err("source null root failure");
            let message = error
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied())
                .unwrap();
            assert!(message.contains("hide root.PinnedDock"));
        }
        assert_eq!(f.lookups.get(), 4);
        assert_eq!(f.null_inits.get(), 0);
    });
}
#[test]
fn native_preview_views_keep_committed_original_list_and_nullable_virtual_init_recovery() {
    run(|| {
        for base in [false, true] {
            let f = PreviewFactory::new();
            let root = RootDock::new();
            let original = ToolDock::new();
            let replacement = ToolDock::new();
            let owner = ToolDock::new();
            owner.set_owner(Some(root.clone()));
            let item = Tool::new();
            item.set_owner(Some(owner.clone()));
            root.set_pinned_dock(Some(original.clone()));
            f.root.replace(Some(root.clone()));
            f.replacement.replace(Some(replacement.clone()));
            let weak = Rc::downgrade(&root);
            original.property_changed().add(Rc::new(move |property| {
                if property == "VisibleDockables" {
                    weak.upgrade().unwrap().set_pinned_dock(None);
                }
            }));
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
            let method = metadata
                .find_methods("PreviewPinnedDockable")
                .find(|m| {
                    m.parameters.len() == 1
                        && (m.parameters[0])() == ValueType::of::<Option<Rc<dyn IDockable>>>()
                })
                .unwrap();
            (method.invoke)(&[
                Some(receiver),
                Some(Rc::new(item.clone() as Rc<dyn IDockable>)),
            ])
            .unwrap();
            assert_eq!(f.lookups.get(), 1);
            assert_eq!(f.null_inits.get(), 1);
            assert!(Rc::ptr_eq(
                &root.pinned_dock().unwrap(),
                &(replacement.clone() as Rc<dyn IToolDock>)
            ));
            assert!(Rc::ptr_eq(
                &replacement.active_dockable().unwrap(),
                &(item.clone() as Rc<dyn IDockable>)
            ));
            let list = original.visible_dockables().unwrap();
            assert_eq!(list.count(), 1);
            assert!(Rc::ptr_eq(
                &list.get(0).unwrap(),
                &(item.clone() as Rc<dyn IDockable>)
            ));
            assert!(original.active_dockable().is_none());
            assert!(Rc::ptr_eq(
                &item.original_owner().unwrap(),
                &(owner as Rc<dyn IDockable>)
            ));
        }
    });
}

//! Native floating/window overloads retain genuine null, options and returned windows.
use crate::markup_extension_tests::run;
use ferrodock_model::{controls::IRootDock, core::*, FactoryBase, FactoryBaseImpl};
use ferrodock_model_ferro::{FactoryBaseReference, IFactoryMetadata};
use ferrodock_model_reactiveui::{
    controls::RootDock, core::DockWindow, impl_factory, Factory, FactoryImpl,
};
use ferroui_base::{
    data::core::ValueType,
    metadata::{from_markup_value, MarkupTyped},
    BoxedValue,
};
use std::{
    cell::RefCell,
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
#[test]
fn native_default_window_null_is_noop_and_typed_float_null_preserves_original_access_failure() {
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
            for count in [1, 2] {
                let method = metadata
                    .find_methods("CreateWindowFrom")
                    .find(|m| m.parameters.len() == count)
                    .unwrap();
                assert_eq!(
                    (method.parameters[0])(),
                    ValueType::of::<Option<Rc<dyn IDockable>>>()
                );
                let mut args = vec![Some(receiver.clone())];
                args.resize(count + 1, None);
                assert!((method.invoke)(&args).unwrap().is_none());
            }
            for name in ["FloatDockable", "FloatAllDockables"] {
                let object = metadata
                    .find_methods(name)
                    .find(|m| {
                        m.parameters.len() == 1
                            && (m.parameters[0])() == ValueType::of::<Option<BoxedValue>>()
                    })
                    .unwrap();
                (object.invoke)(&[Some(receiver.clone()), None]).unwrap();
                let typed = metadata
                    .find_methods(name)
                    .find(|m| {
                        m.parameters.len() == 1
                            && (m.parameters[0])() == ValueType::of::<Option<Rc<dyn IDockable>>>()
                    })
                    .unwrap();
                assert!(failure(|| {
                    (typed.invoke)(&[Some(receiver.clone()), None]).unwrap();
                })
                .contains("ResolveOperationDock dockable.Owner"));
            }
        }
    });
}
struct DispatchFactory {
    component: Factory,
    window: Rc<dyn IDockWindow>,
    calls: RefCell<Vec<&'static str>>,
    options: RefCell<Vec<Rc<DockWindowOptions>>>,
}
impl DispatchFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            window: DockWindow::new(),
            calls: Default::default(),
            options: Default::default(),
        })
    }
    fn record(&self, name: &'static str, options: Option<Rc<DockWindowOptions>>) {
        self.calls.borrow_mut().push(name);
        if let Some(options) = options {
            self.options.borrow_mut().push(options);
        }
    }
}
impl FactoryBaseImpl for DispatchFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for DispatchFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn float_dockable(this: &Self, item: Option<Rc<dyn IDockable>>) {
        assert!(item.is_none());
        this.record("float", None);
    }
    fn float_dockable_with_options(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        assert!(item.is_none());
        this.record("float-options", options);
    }
    fn float_all_dockables(this: &Self, item: Option<Rc<dyn IDockable>>) {
        assert!(item.is_none());
        this.record("all", None);
    }
    fn float_all_dockables_with_options(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        assert!(item.is_none());
        this.record("all-options", options);
    }
    fn create_window_from(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
    ) -> Option<Rc<dyn IDockWindow>> {
        assert!(item.is_none());
        this.record("plain", None);
        Some(this.window.clone())
    }
    fn create_window_from_with_options(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) -> Option<Rc<dyn IDockWindow>> {
        assert!(item.is_none());
        this.record("create-options", options);
        Some(this.window.clone())
    }
    #[allow(clippy::too_many_arguments)]
    fn split_to_window(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        item: Option<Rc<dyn IDockable>>,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    ) {
        assert!(dock.is_none() && item.is_none());
        assert_eq!((x, y, w, h), (1.0, 2.0, 3.0, 4.0));
        this.record("split", None);
    }
    #[allow(clippy::too_many_arguments)]
    fn split_to_window_with_options(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        item: Option<Rc<dyn IDockable>>,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        assert!(dock.is_none() && item.is_none());
        assert_eq!((x, y, w, h), (1.0, 2.0, 3.0, 4.0));
        this.record("split-options", options);
    }
}
impl_factory!(DispatchFactory);
#[test]
fn both_native_views_forward_all_nullable_floating_slots_with_original_options_and_results() {
    run(|| {
        let f = DispatchFactory::new();
        let options = DockWindowOptions::new();
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
                ("FloatDockable", 1),
                ("FloatDockable", 2),
                ("FloatAllDockables", 1),
                ("FloatAllDockables", 2),
                ("CreateWindowFrom", 1),
                ("CreateWindowFrom", 2),
                ("SplitToWindow", 6),
                ("SplitToWindow", 7),
            ] {
                let first = if name == "SplitToWindow" {
                    ValueType::of::<Option<Rc<dyn IDock>>>()
                } else {
                    ValueType::of::<Option<Rc<dyn IDockable>>>()
                };
                let method = metadata
                    .find_methods(name)
                    .find(|m| m.parameters.len() == count && (m.parameters[0])() == first)
                    .unwrap();
                let mut args = vec![Some(receiver.clone()), None];
                if name == "SplitToWindow" {
                    args.push(None);
                    for value in [1.0f64, 2.0, 3.0, 4.0] {
                        args.push(Some(Rc::new(value)));
                    }
                }
                if count == 2 || count == 7 {
                    args.push(Some(Rc::new(options.clone())));
                }
                let result = (method.invoke)(&args).unwrap();
                if name == "CreateWindowFrom" {
                    assert!(Rc::ptr_eq(
                        &from_markup_value::<Rc<dyn IDockWindow>>(&result).unwrap(),
                        &f.window
                    ));
                } else {
                    assert!(result.is_none());
                }
            }
        }
        assert_eq!(
            &*f.calls.borrow(),
            &[
                "float",
                "float-options",
                "all",
                "all-options",
                "plain",
                "create-options",
                "split",
                "split-options"
            ]
            .repeat(2)
        );
        assert_eq!(f.options.borrow().len(), 8);
        for actual in f.options.borrow().iter() {
            assert!(Rc::ptr_eq(actual, &options));
        }
    });
}
struct OwnershipFactory {
    component: Factory,
    root: Rc<dyn IRootDock>,
    window: Rc<dyn IDockWindow>,
    calls: RefCell<Vec<&'static str>>,
}
impl OwnershipFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            root: RootDock::new(),
            window: DockWindow::new(),
            calls: Default::default(),
        })
    }
}
impl FactoryBaseImpl for OwnershipFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for OwnershipFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn find_root(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        _: Option<RootPredicate>,
    ) -> Option<Rc<dyn IRootDock>> {
        assert!(item.is_none());
        this.calls.borrow_mut().push("find");
        Some(this.root.clone())
    }
    fn create_window_from(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
    ) -> Option<Rc<dyn IDockWindow>> {
        assert!(item.is_none());
        this.calls.borrow_mut().push("plain");
        Some(this.window.clone())
    }
}
impl_factory!(OwnershipFactory);
#[test]
fn native_options_capture_original_parent_before_nullable_plain_override_and_apply_same_object() {
    run(|| {
        for base in [false, true] {
            let f = OwnershipFactory::new();
            let parent = DockWindow::new();
            f.root.set_window(Some(parent.clone()));
            let options = DockWindowOptions::new();
            options.set_owner_mode(DockWindowOwnerMode::DockableWindow);
            options.set_is_modal(true);
            options.set_show_in_taskbar(Some(false));
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
                .find_methods("CreateWindowFrom")
                .find(|m| m.parameters.len() == 2)
                .unwrap();
            let result =
                (method.invoke)(&[Some(receiver), None, Some(Rc::new(options.clone()))]).unwrap();
            let window = from_markup_value::<Rc<dyn IDockWindow>>(&result).unwrap();
            assert!(Rc::ptr_eq(&window, &f.window));
            assert!(Rc::ptr_eq(
                &options.parent_window().unwrap(),
                &(parent.clone() as Rc<dyn IDockWindow>)
            ));
            assert!(Rc::ptr_eq(
                &window.parent_window().unwrap(),
                &(parent as Rc<dyn IDockWindow>)
            ));
            assert_eq!(window.owner_mode(), DockWindowOwnerMode::DockableWindow);
            assert!(window.is_modal());
            assert_eq!(window.show_in_taskbar(), Some(false));
            assert_eq!(&*f.calls.borrow(), &["find", "plain"]);
        }
    });
}

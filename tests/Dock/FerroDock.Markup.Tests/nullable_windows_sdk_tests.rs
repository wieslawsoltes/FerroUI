//! Native lifecycle slots and compiled/runtime nullable Windows consumers use original shared models.
use crate::markup_extension_tests::run;
use ferrodock_model::{controls::IRootDock, core::*, FactoryBase, FactoryBaseImpl};
use ferrodock_model_ferro::{
    DockWindowListMetadata, FactoryBaseReference, IDockWindowHostFactoryMetadata, IFactoryMetadata,
};
use ferrodock_model_reactiveui::{
    controls::RootDock, core::DockWindow, impl_factory, Factory, FactoryImpl,
};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::{from_markup_value, MarkupTyped},
    threading::Dispatcher,
    BoxedValue, Ref,
};
use ferroui_controls::{ContentControl, ItemsControl, StackPanel, TextBlock};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn flush() {
    Dispatcher::ui_thread().run_jobs(None);
}
fn check_bindings(panel: &Ref<StackPanel>) {
    let f = Factory::new();
    let root = RootDock::new();
    let list = f.create_list::<Option<Rc<dyn IDockWindow>>>(vec![None]);
    root.set_windows(Some(list.clone()));
    let context: BoxedValue = Rc::new(root.clone() as Rc<dyn IRootDock>);
    panel.set_data_context(Some(context.clone()));
    flush();
    let count = panel.children().get(0).cast::<TextBlock>().unwrap();
    let content = panel.children().get(1).cast::<ContentControl>().unwrap();
    let items = panel.children().get(2).cast::<ItemsControl>().unwrap();
    assert_eq!(count.text().as_deref(), Some("1"));
    assert!(content.content().is_none());
    assert_eq!(items.items().count(), 1);
    let window: Rc<dyn IDockWindow> = DockWindow::new();
    list.set(0, Some(window.clone()));
    flush();
    assert!(content.content().is_none());
    panel.set_data_context(None);
    panel.set_data_context(Some(context.clone()));
    flush();
    assert!(Rc::ptr_eq(
        &from_markup_value::<Rc<dyn IDockWindow>>(&content.content()).unwrap(),
        &window
    ));
    list.set(0, None);
    list.add(None);
    flush();
    assert_eq!(count.text().as_deref(), Some("2"));
    assert_eq!(items.items().count(), 2);
    assert!(Rc::ptr_eq(
        &from_markup_value::<Rc<dyn IDockWindow>>(&content.content()).unwrap(),
        &window
    ));
    panel.set_data_context(None);
    panel.set_data_context(Some(context));
    flush();
    assert!(content.content().is_none());
    assert!(list.remove(&None));
    flush();
    assert_eq!(count.text().as_deref(), Some("1"));
    assert_eq!(items.items().count(), 1);
    assert!(root.windows().unwrap().ptr_eq(&list));
    panel.set_data_context(None);
}
#[test]
fn compiled_window_bindings_preserve_null_slots_counts_items_and_original_indexer_rebind() {
    run(|| {
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://ferrodock_markup_tests/CoreNullWindowsBindings.xaml",
        )
        .unwrap()
        .unwrap();
        check_bindings(value.downcast_ref::<Ref<StackPanel>>().unwrap());
    });
}
#[test]
fn runtime_window_bindings_preserve_original_nullable_collection_and_indexer_semantics() {
    run(|| {
        let mut config = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default();
        config.local_assembly = Some(&crate::locator_factory::ASSEMBLY);
        config.use_compiled_bindings_by_default = true;
        let source = include_str!("CoreNullWindowsBindings.xaml");
        let emitted = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &[("Windows.xaml", source)],
            Some("ferres://ferrodock_markup_tests/"),
            &config,
            &[],
        );
        assert_eq!(emitted.len(), 1);
        assert!(emitted[0].source.is_ok(), "{:?}", emitted[0].source);
        let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
            ferroui_markup_xaml::RuntimeXamlLoaderDocument::new(source),
            Some(config),
        )
        .unwrap();
        check_bindings(value.downcast_ref::<Ref<StackPanel>>().unwrap());
    });
}
#[test]
fn native_window_list_mutations_retain_real_null_and_window_references_in_original_list() {
    run(|| {
        let f = Factory::new();
        let root = f.create_root_dock();
        let list = f.create_list::<Option<Rc<dyn IDockWindow>>>(Vec::new());
        root.set_windows(Some(list.clone()));
        let value: BoxedValue = Rc::new(list.clone());
        let metadata = <dyn DockWindowListMetadata as MarkupTyped>::MARKUP;
        let add = metadata.find_methods("Add").next().unwrap();
        assert_eq!(
            (add.parameters[0])(),
            ValueType::of::<Option<Rc<dyn IDockWindow>>>()
        );
        (add.invoke)(&[Some(value.clone()), None]).unwrap();
        let window = f.create_dock_window();
        (add.invoke)(&[Some(value.clone()), Some(Rc::new(window.clone()))]).unwrap();
        assert!(list.get(0).is_none());
        assert!(Rc::ptr_eq(list.get(1).as_ref().unwrap(), &window));
        for name in ["Contains", "IndexOf"] {
            let method = metadata.find_methods(name).next().unwrap();
            let result = (method.invoke)(&[Some(value.clone()), None]).unwrap();
            if name == "Contains" {
                assert!(from_markup_value::<bool>(&result).unwrap());
            } else {
                assert_eq!(from_markup_value::<i32>(&result).unwrap(), 0);
            }
        }
        let remove = metadata.find_methods("Remove").next().unwrap();
        assert!(
            from_markup_value::<bool>(&(remove.invoke)(&[Some(value), None]).unwrap()).unwrap()
        );
        assert_eq!(list.count(), 1);
        assert!(Rc::ptr_eq(list.get(0).as_ref().unwrap(), &window));
        assert!(root.windows().unwrap().ptr_eq(&list));
    });
}
struct DispatchFactory {
    component: Factory,
    calls: RefCell<Vec<&'static str>>,
    host: Rc<dyn IHostWindow>,
    requests: Cell<usize>,
}
impl DispatchFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            calls: Default::default(),
            host: ferrodock_ferro::controls::ManagedHostWindow::new(),
            requests: Cell::new(0),
        })
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
    fn as_dock_window_host_factory(this: Rc<Self>) -> Option<Rc<dyn IDockWindowHostFactory>> {
        Some(this)
    }
    fn add_window(
        this: &Self,
        root: Option<Rc<dyn IRootDock>>,
        window: Option<Rc<dyn IDockWindow>>,
    ) {
        assert!(root.is_none() && window.is_none());
        this.calls.borrow_mut().push("add");
    }
    fn insert_window(
        this: &Self,
        root: Option<Rc<dyn IRootDock>>,
        window: Option<Rc<dyn IDockWindow>>,
        index: i32,
    ) {
        assert!(root.is_none() && window.is_none());
        assert_eq!(index, 0);
        this.calls.borrow_mut().push("insert");
    }
    fn remove_window(this: &Self, window: Option<Rc<dyn IDockWindow>>) {
        assert!(window.is_none());
        this.calls.borrow_mut().push("remove");
    }
    fn close_window(this: &Self, window: Option<Rc<dyn IDockWindow>>) {
        assert!(window.is_none());
        this.calls.borrow_mut().push("close");
    }
    fn init_dock_window(
        this: &Self,
        window: Option<Rc<dyn IDockWindow>>,
        owner: Option<Rc<dyn IDockable>>,
    ) {
        assert!(window.is_none() && owner.is_none());
        this.calls.borrow_mut().push("init");
    }
    fn init_dock_window_with_host(
        this: &Self,
        window: Option<Rc<dyn IDockWindow>>,
        owner: Option<Rc<dyn IDockable>>,
        host: Option<Rc<dyn IHostWindow>>,
    ) {
        assert!(window.is_none() && owner.is_none() && host.is_none());
        this.calls.borrow_mut().push("explicit");
    }
}
impl IDockWindowHostFactory for DispatchFactory {
    fn create_host_window(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
    ) -> Option<Rc<dyn IHostWindow>> {
        assert!(window.is_none());
        self.requests.set(self.requests.get() + 1);
        Some(self.host.clone())
    }
}
impl_factory!(DispatchFactory);
#[test]
fn both_native_factory_views_forward_actual_null_to_all_window_lifecycle_virtual_slots() {
    run(|| {
        let f = DispatchFactory::new();
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
                ("AddWindow", 2),
                ("InsertWindow", 3),
                ("RemoveWindow", 1),
                ("CloseWindow", 1),
                ("InitDockWindow", 2),
                ("InitDockWindow", 3),
            ] {
                let method = metadata
                    .find_methods(name)
                    .find(|m| m.parameters.len() == count)
                    .unwrap();
                let mut args = vec![Some(receiver.clone())];
                args.resize(count + 1, None);
                if name == "InsertWindow" {
                    args[3] = Some(Rc::new(0i32));
                }
                (method.invoke)(&args).unwrap();
            }
        }
        assert_eq!(
            &*f.calls.borrow(),
            &["add", "insert", "remove", "close", "init", "explicit"].repeat(2)
        );
    });
}
#[test]
fn optional_host_factory_native_cast_and_method_keep_null_input_and_original_host_result() {
    run(|| {
        let f = DispatchFactory::new();
        let factory_value: BoxedValue = Rc::new(f.clone() as Rc<dyn IFactory>);
        let projected_value = ValueTypes::try_cast(
            &factory_value,
            ValueType::of::<Rc<dyn IDockWindowHostFactory>>(),
        )
        .unwrap();
        let projected =
            from_markup_value::<Rc<dyn IDockWindowHostFactory>>(&Some(projected_value)).unwrap();
        let expected: Rc<dyn IDockWindowHostFactory> = f.clone();
        assert!(Rc::ptr_eq(&projected, &expected));
        let method = <dyn IDockWindowHostFactoryMetadata as MarkupTyped>::MARKUP
            .find_methods("CreateHostWindow")
            .next()
            .unwrap();
        assert_eq!(
            (method.parameters[0])(),
            ValueType::of::<Option<Rc<dyn IDockWindow>>>()
        );
        let value = (method.invoke)(&[Some(Rc::new(projected)), None]).unwrap();
        let host = from_markup_value::<Rc<dyn IHostWindow>>(&value).unwrap();
        assert!(Rc::ptr_eq(&host, &f.host));
        assert_eq!(f.requests.get(), 1);
    });
}

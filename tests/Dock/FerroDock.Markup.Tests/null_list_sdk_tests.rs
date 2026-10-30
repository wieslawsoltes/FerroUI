//! Nullable source collection values, original indexer behavior and native query cursors.
use crate::markup_extension_tests::run;
use ferrodock_model::{core::*, FactoryBase, FactoryBaseImpl};
use ferrodock_model_ferro::{DockableListMetadata, DockableQueryReference};
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock},
    impl_factory, Factory, FactoryImpl,
};
use ferroui_base::{
    metadata::{from_markup_value, MarkupTyped},
    threading::Dispatcher,
    BoxedValue, Ref,
};
use ferroui_controls::{ContentControl, ItemsControl, StackPanel, TextBlock};
use std::rc::Rc;

fn flush() {
    Dispatcher::ui_thread().run_jobs(None);
}
fn check(panel: &Ref<StackPanel>) {
    let factory = Factory::new();
    let model = DocumentDock::new();
    let list = factory.create_list::<Option<Rc<dyn IDockable>>>(vec![None]);
    model.set_visible_dockables(Some(list.clone()));
    let context: BoxedValue = Rc::new(model.clone() as Rc<dyn IDock>);
    panel.set_data_context(Some(context.clone()));
    flush();
    let count = panel.children().get(0).cast::<TextBlock>().unwrap();
    let content = panel.children().get(1).cast::<ContentControl>().unwrap();
    let items = panel.children().get(2).cast::<ItemsControl>().unwrap();
    assert_eq!(count.text().as_deref(), Some("1"));
    assert!(content.content().is_none());
    assert_eq!(items.items().count(), 1);
    let child: Rc<dyn IDockable> = Document::new();
    list.set(0, Some(child.clone()));
    flush();
    // Unchanged C# compiled IList indexers retain their previous value until rebind.
    assert!(content.content().is_none());
    panel.set_data_context(None);
    panel.set_data_context(Some(context.clone()));
    flush();
    assert!(Rc::ptr_eq(
        &from_markup_value::<Rc<dyn IDockable>>(&content.content()).unwrap(),
        &child
    ));
    list.set(0, None);
    list.add(None);
    flush();
    assert_eq!(count.text().as_deref(), Some("2"));
    assert_eq!(items.items().count(), 2);
    assert!(Rc::ptr_eq(
        &from_markup_value::<Rc<dyn IDockable>>(&content.content()).unwrap(),
        &child
    ));
    panel.set_data_context(None);
    panel.set_data_context(Some(context));
    flush();
    assert!(content.content().is_none());
    assert!(list.remove(&None));
    flush();
    assert_eq!(count.text().as_deref(), Some("1"));
    assert_eq!(items.items().count(), 1);
    panel.set_data_context(None);
}
#[test]
fn compiled_nullable_list_bindings_preserve_null_slots_counts_and_original_indexer_rebind() {
    run(|| {
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://ferrodock_markup_tests/CoreNullListBindings.xaml",
        )
        .unwrap()
        .unwrap();
        check(value.downcast_ref::<Ref<StackPanel>>().unwrap());
    });
}
#[test]
fn runtime_nullable_list_bindings_preserve_original_collection_and_indexer_behavior() {
    run(|| {
        let mut config = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default();
        config.local_assembly = Some(&crate::locator_factory::ASSEMBLY);
        config.use_compiled_bindings_by_default = true;
        let source = include_str!("CoreNullListBindings.xaml");
        let emitted = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &[("NullList.xaml", source)],
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
        check(value.downcast_ref::<Ref<StackPanel>>().unwrap());
    });
}
struct NullInitFactory {
    component: Factory,
}
impl NullInitFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
        })
    }
}
impl FactoryBaseImpl for NullInitFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for NullInitFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn init_dockable(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        owner: Option<Rc<dyn IDockable>>,
    ) {
        if item.is_some() {
            this.component.base().init_dockable(item, owner);
        }
    }
}
impl_factory!(NullInitFactory);
#[test]
fn native_insertion_views_keep_negative_null_noop_and_original_null_list_mutation() {
    run(|| {
        use ferrodock_model_ferro::{FactoryBaseReference, IFactoryMetadata};
        let factory = NullInitFactory::new();
        for (receiver, metadata) in [
            (
                Rc::new(factory.clone() as Rc<dyn IFactory>) as BoxedValue,
                <dyn IFactoryMetadata as MarkupTyped>::MARKUP,
            ),
            (
                Rc::new(FactoryBaseReference::new(factory.clone())) as BoxedValue,
                FactoryBaseReference::MARKUP,
            ),
        ] {
            let method = metadata.find_methods("InsertDockable").next().unwrap();
            assert!(
                (method.invoke)(&[Some(receiver.clone()), None, None, Some(Rc::new(-1i32)),])
                    .unwrap()
                    .is_none()
            );
            let model = DocumentDock::new();
            let list = factory.create_list::<Option<Rc<dyn IDockable>>>(vec![]);
            model.set_visible_dockables(Some(list.clone()));
            assert!((method.invoke)(&[
                Some(receiver),
                Some(Rc::new(model.clone() as Rc<dyn IDock>)),
                None,
                Some(Rc::new(0i32)),
            ])
            .unwrap()
            .is_none());
            assert!(list.ptr_eq(&model.visible_dockables().unwrap()));
            assert_eq!(list.count(), 1);
            assert!(list.get(0).is_none());
            assert!(model.is_empty());
            assert_eq!(model.opened_dockables_count(), 1);
        }
    });
}

struct NullMutationFactory {
    component: Factory,
    adds: std::cell::Cell<usize>,
    inserts: std::cell::Cell<usize>,
    removals: std::cell::Cell<usize>,
}
impl NullMutationFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            adds: std::cell::Cell::new(0),
            inserts: std::cell::Cell::new(0),
            removals: std::cell::Cell::new(0),
        })
    }
}
impl FactoryBaseImpl for NullMutationFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for NullMutationFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn add_dockable(this: &Self, dock: Option<Rc<dyn IDock>>, item: Option<Rc<dyn IDockable>>) {
        assert!(dock.is_none());
        assert!(item.is_none());
        this.adds.set(this.adds.get() + 1);
    }
    fn insert_dockable(
        this: &Self,
        dock: Option<Rc<dyn IDock>>,
        item: Option<Rc<dyn IDockable>>,
        index: i32,
    ) {
        assert!(dock.is_none());
        assert!(item.is_none());
        assert_eq!(index, 2);
        this.inserts.set(this.inserts.get() + 1);
    }
    fn remove_dockable(this: &Self, item: Option<Rc<dyn IDockable>>, collapse: bool) {
        assert!(item.is_none());
        assert!(collapse);
        this.removals.set(this.removals.get() + 1);
    }
}
impl_factory!(NullMutationFactory);
#[test]
fn native_mutation_views_dispatch_actual_null_arguments_to_enclosing_overrides() {
    run(|| {
        use ferrodock_model_ferro::{FactoryBaseReference, IFactoryMetadata};
        let factory = NullMutationFactory::new();
        for (receiver, metadata) in [
            (
                Rc::new(factory.clone() as Rc<dyn IFactory>) as BoxedValue,
                <dyn IFactoryMetadata as MarkupTyped>::MARKUP,
            ),
            (
                Rc::new(FactoryBaseReference::new(factory.clone())) as BoxedValue,
                FactoryBaseReference::MARKUP,
            ),
        ] {
            let add = metadata.find_methods("AddDockable").next().unwrap();
            (add.invoke)(&[Some(receiver.clone()), None, None]).unwrap();
            let insert = metadata.find_methods("InsertDockable").next().unwrap();
            (insert.invoke)(&[Some(receiver.clone()), None, None, Some(Rc::new(2i32))]).unwrap();
            let remove = metadata.find_methods("RemoveDockable").next().unwrap();
            (remove.invoke)(&[Some(receiver), None, Some(Rc::new(true))]).unwrap();
        }
        assert_eq!(factory.adds.get(), 2);
        assert_eq!(factory.inserts.get(), 2);
        assert_eq!(factory.removals.get(), 2);
    });
}

#[test]
fn native_list_contract_and_query_cursor_keep_null_as_a_value_through_original_base_add() {
    run(|| {
        let factory = NullInitFactory::new();
        let model = DocumentDock::new();
        factory.add_dockable(Some(model.clone()), None);
        let list = model.visible_dockables().unwrap();
        assert_eq!(list.count(), 1);
        assert!(list.get(0).is_none());
        let metadata = <dyn DockableListMetadata as MarkupTyped>::MARKUP;
        let receiver: BoxedValue = Rc::new(list.clone());
        let call = |name: &str, values: Vec<Option<BoxedValue>>| {
            let mut args = vec![Some(receiver.clone())];
            args.extend(values);
            (metadata.find_methods(name).next().unwrap().invoke)(&args).unwrap()
        };
        call("Add", vec![None]);
        assert_eq!(
            from_markup_value::<i32>(&call("IndexOf", vec![None])),
            Some(0)
        );
        assert_eq!(
            from_markup_value::<bool>(&call("Contains", vec![None])),
            Some(true)
        );
        assert_eq!(
            from_markup_value::<bool>(&call("Remove", vec![None])),
            Some(true)
        );
        let child: Rc<dyn IDockable> = Document::new();
        list.add(Some(child.clone()));
        let query = DockableQueryReference::new(
            factory.find_in_dock(Some(model.clone()), Some(Rc::new(|_| true))),
        );
        let cursor = query.get_enumerator();
        assert!(cursor.current().is_none());
        assert!(cursor.move_next());
        let root: Rc<dyn IDockable> = model;
        assert!(Rc::ptr_eq(&cursor.current().unwrap(), &root));
        assert!(cursor.move_next());
        assert!(cursor.current().is_none());
        assert!(cursor.move_next());
        assert!(Rc::ptr_eq(&cursor.current().unwrap(), &child));
        assert!(!cursor.move_next());
        assert!(Rc::ptr_eq(&cursor.current().unwrap(), &child));
        cursor.dispose();
    });
}

struct NullRootFactory {
    component: Factory,
    root: Rc<dyn ferrodock_model::controls::IRootDock>,
}
impl NullRootFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            root: ferrodock_model_reactiveui::controls::RootDock::new(),
        })
    }
}
impl FactoryBaseImpl for NullRootFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for NullRootFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn find_root(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        predicate: Option<RootPredicate>,
    ) -> Option<Rc<dyn ferrodock_model::controls::IRootDock>> {
        if item.is_none() {
            Some(this.root.clone())
        } else {
            this.component.base().find_root(item, predicate)
        }
    }
}
impl_factory!(NullRootFactory);
#[test]
fn native_factory_views_forward_null_root_arguments_and_original_null_pin_membership() {
    run(|| {
        use ferrodock_model::controls::IRootDock;
        use ferrodock_model_ferro::{FactoryBaseReference, IFactoryMetadata};
        let factory = NullRootFactory::new();
        factory
            .root
            .set_left_pinned_dockables(Some(factory.create_list(vec![None])));
        for (receiver, metadata) in [
            (
                Rc::new(factory.clone() as Rc<dyn IFactory>) as BoxedValue,
                <dyn IFactoryMetadata as MarkupTyped>::MARKUP,
            ),
            (
                Rc::new(FactoryBaseReference::new(factory.clone())) as BoxedValue,
                FactoryBaseReference::MARKUP,
            ),
        ] {
            let call = |name: &str| {
                let method = metadata
                    .find_methods(name)
                    .find(|method| method.parameters.len() == 1)
                    .unwrap();
                (method.invoke)(&[Some(receiver.clone()), None]).unwrap()
            };
            let root = from_markup_value::<Rc<dyn IRootDock>>(&call("FindRoot")).unwrap();
            assert!(Rc::ptr_eq(&root, &factory.root));
            assert_eq!(
                from_markup_value::<bool>(&call("IsDockablePinned")),
                Some(true)
            );
        }
    });
}

#[test]
fn native_add_null_target_preserves_original_init_side_effects_before_failure() {
    run(|| {
        use ferrodock_model_ferro::{FactoryBaseReference, IFactoryMetadata};
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let factory = Factory::new();
        for (receiver, metadata) in [
            (
                Rc::new(factory.clone() as Rc<dyn IFactory>) as BoxedValue,
                <dyn IFactoryMetadata as MarkupTyped>::MARKUP,
            ),
            (
                Rc::new(FactoryBaseReference::new(factory.clone())) as BoxedValue,
                FactoryBaseReference::MARKUP,
            ),
        ] {
            let method = metadata.find_methods("AddDockable").next().unwrap();
            assert_eq!(
                (method.parameters[0])(),
                ferroui_base::data::core::ValueType::of::<Option<Rc<dyn IDock>>>()
            );
            let item = Document::new();
            item.set_owner(Some(DocumentDock::new()));
            assert!(catch_unwind(AssertUnwindSafe(|| {
                (method.invoke)(&[
                    Some(receiver.clone()),
                    None,
                    Some(Rc::new(item.clone() as Rc<dyn IDockable>)),
                ])
                .unwrap();
            }))
            .is_err());
            assert!(item.owner().is_none());
            assert!(Rc::ptr_eq(
                &item.factory().unwrap(),
                &(factory.clone() as Rc<dyn IFactory>)
            ));
        }
    });
}

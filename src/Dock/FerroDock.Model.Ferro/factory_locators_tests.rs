//! Source locator contracts through metadata, without selecting a UI control library.
use crate::*;
use ferrodock_model::{core::*, FactoryBase, FactoryBaseImpl};
use ferrodock_model_reactiveui::{impl_factory, Factory, FactoryImpl};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{from_markup_value, MarkupDelegate, MarkupType, MarkupTyped, MarkupValue},
    BoxedValue,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn set(metadata: &'static MarkupType, value: &BoxedValue, name: &str, argument: MarkupValue) {
    (metadata.find_property(name).unwrap().set.unwrap())(&[Some(value.clone()), argument]).unwrap();
}
fn get<T: Clone + 'static>(metadata: &'static MarkupType, value: &BoxedValue, name: &str) -> T {
    let raw = (metadata.find_property(name).unwrap().get.unwrap())(&[Some(value.clone())]).unwrap();
    from_markup_value(&raw).unwrap()
}
fn call(
    metadata: &'static MarkupType,
    value: BoxedValue,
    name: &str,
    args: &[MarkupValue],
) -> MarkupValue {
    let method = metadata
        .find_methods(name)
        .find(|m| m.parameters.len() == args.len())
        .unwrap();
    let mut all = vec![Some(value)];
    all.extend_from_slice(args);
    (method.invoke)(&all).unwrap()
}
#[test]
fn core_sdk_owns_the_single_factory_contract_and_helpers_and_preserves_original_delegate_identity()
{
    register_types();
    for metadata in [
        <dyn IFactoryMetadata as MarkupTyped>::MARKUP,
        FactoryExtensionsMetadata::MARKUP,
    ] {
        assert_eq!(
            ferroui_base::metadata::MarkupAssembly::of_module(metadata.module_path)
                .unwrap()
                .name,
            "FerroDock.Model"
        );
    }
    let factory = Factory::new();
    let base: BoxedValue = Rc::new(FactoryBaseReference::new(factory.clone()));
    let interface: BoxedValue = Rc::new(factory.clone() as Rc<dyn IFactory>);
    let metadata = <dyn IFactoryMetadata as MarkupTyped>::MARKUP;
    let count = Rc::new(Cell::new(0));
    let calls = count.clone();
    let context: ferrodock_model::BoxedValue = Rc::new("Original context".to_owned());
    let expected = context.clone();
    let source: ContextFactory = Rc::new(move || {
        calls.set(calls.get() + 1);
        Some(expected.clone())
    });
    for (metadata, value) in [(FactoryBaseReference::MARKUP, base), (metadata, interface)] {
        set(
            metadata,
            &value,
            "DefaultContextLocator",
            Some(Rc::new(ContextFactoryReference::new(source.clone()))),
        );
        assert_eq!(count.get(), 0);
        let view: Option<ContextFactoryReference> = get(metadata, &value, "DefaultContextLocator");
        let view = view.unwrap();
        assert!(Rc::ptr_eq(&view.model(), &source));
        let boxed: BoxedValue = Rc::new(view.clone());
        let again: BoxedValue = Rc::new(view);
        assert!(ValueTypes::reference_equals(&boxed, &again));
        let result = call(
            metadata,
            value.clone(),
            "GetContext",
            &[Some(Rc::new("missing".to_owned()))],
        )
        .unwrap();
        assert_eq!(result.downcast_ref::<String>().unwrap(), "Original context");
        set(metadata, &value, "DefaultContextLocator", None);
        assert!(
            get::<Option<ContextFactoryReference>>(metadata, &value, "DefaultContextLocator")
                .is_none()
        );
        count.set(0);
    }
}
#[test]
fn locator_maps_distinguish_missing_keys_null_delegates_and_delegates_returning_null() {
    register_types();
    let factory = Factory::new();
    let value: BoxedValue = Rc::new(FactoryBaseReference::new(factory.clone()));
    let contexts = ContextLocatorMapReference::create();
    let hosts = HostWindowLocatorMapReference::create();
    let dockables = DockableLocatorMapReference::create();
    let context_defaults = Rc::new(Cell::new(0));
    let calls = context_defaults.clone();
    factory.set_default_context_locator(Some(Rc::new(move || {
        calls.set(calls.get() + 1);
        None
    })));
    let host_defaults = Rc::new(Cell::new(0));
    let calls = host_defaults.clone();
    factory.set_default_host_window_locator(Some(Rc::new(move || {
        calls.set(calls.get() + 1);
        None
    })));
    contexts.add("null".into(), None).unwrap();
    contexts
        .add(
            "none".into(),
            Some(ContextFactoryReference::new(Rc::new(|| None))),
        )
        .unwrap();
    hosts.add("null".into(), None).unwrap();
    hosts
        .add(
            "none".into(),
            Some(HostWindowFactoryReference::new(Rc::new(|| None))),
        )
        .unwrap();
    dockables.add("null".into(), None).unwrap();
    dockables
        .add(
            "none".into(),
            Some(DockableFactoryReference::new(Rc::new(|| None))),
        )
        .unwrap();
    set(
        FactoryBaseReference::MARKUP,
        &value,
        "ContextLocator",
        Some(Rc::new(contexts.clone())),
    );
    set(
        FactoryBaseReference::MARKUP,
        &value,
        "HostWindowLocator",
        Some(Rc::new(hosts.clone())),
    );
    set(
        FactoryBaseReference::MARKUP,
        &value,
        "DockableLocator",
        Some(Rc::new(dockables.clone())),
    );
    assert!(Rc::ptr_eq(
        &factory.context_locator().unwrap(),
        &contexts.model()
    ));
    assert!(Rc::ptr_eq(
        &factory.host_window_locator().unwrap(),
        &hosts.model()
    ));
    assert!(Rc::ptr_eq(
        &factory.dockable_locator().unwrap(),
        &dockables.model()
    ));
    for key in ["", "null", "none"] {
        assert!(factory.get_context(key.into()).is_none());
        assert!(factory.get_host_window(key.into()).is_none());
        assert!(factory.get_dockable::<dyn IDockable>(key.into()).is_none());
    }
    assert_eq!((context_defaults.get(), host_defaults.get()), (0, 0));
    factory.get_context("missing".into());
    factory.get_host_window("missing".into());
    assert_eq!((context_defaults.get(), host_defaults.get()), (1, 1));
    assert!(contexts.model().borrow().get("null").unwrap().is_none());
    let document: Rc<dyn IDockable> = factory.create_document();
    let expected = document.clone();
    let source: DockableFactory = Rc::new(move || Some(expected.clone()));
    let view = DockableFactoryReference::new(source.clone());
    dockables.set("document".into(), Some(view.clone()));
    assert!(Rc::ptr_eq(
        &factory
            .get_dockable::<dyn IDockable>("document".into())
            .unwrap(),
        &document
    ));
    assert!(Rc::ptr_eq(
        &dockables.get("document".into()).unwrap().unwrap().model(),
        &source
    ));
    assert!(Rc::ptr_eq(&view.invoke().unwrap(), &document));
    assert!(factory
        .get_dockable::<dyn ferrodock_model::controls::IToolDock>("document".into())
        .is_none());
}
#[test]
fn each_native_dictionary_constructor_indexer_and_mutator_operates_on_the_original_container() {
    register_types();
    macro_rules! check {
        ($map:ty,$delegate:ty) => {{
            let metadata = <$map as MarkupTyped>::MARKUP;
            let raw = (metadata.constructors[0].invoke)(&[]).unwrap().unwrap();
            let model = raw.downcast_ref::<$map>().unwrap().clone();
            let callback = <$delegate>::new(Rc::new(|| None));
            let pointer = callback.model();
            call(
                metadata,
                raw.clone(),
                "Add",
                &[Some(Rc::new("null".to_owned())), None],
            );
            assert_eq!(model.count(), 1);
            assert!(model.contains_key("null".into()));
            let setter = metadata.find_indexer(1).unwrap().set.unwrap();
            setter(&[
                Some(raw.clone()),
                Some(Rc::new("value".to_owned())),
                Some(Rc::new(callback)),
            ])
            .unwrap();
            assert_eq!(model.count(), 2);
            assert!(Rc::ptr_eq(
                &model.get("value".into()).unwrap().unwrap().model(),
                &pointer
            ));
            assert!((metadata.find_indexer(1).unwrap().get.unwrap())(&[
                Some(raw.clone()),
                Some(Rc::new("missing".to_owned()))
            ])
            .is_err());
            let add = metadata.find_methods("Add").next().unwrap();
            assert!(
                (add.invoke)(&[Some(raw.clone()), Some(Rc::new("null".to_owned())), None]).is_err()
            );
            assert_eq!(model.count(), 2);
            assert_eq!(
                call(
                    metadata,
                    raw.clone(),
                    "ContainsKey",
                    &[Some(Rc::new("null".to_owned()))]
                )
                .unwrap()
                .downcast_ref::<bool>(),
                Some(&true)
            );
            assert_eq!(
                call(
                    metadata,
                    raw.clone(),
                    "Remove",
                    &[Some(Rc::new("null".to_owned()))]
                )
                .unwrap()
                .downcast_ref::<bool>(),
                Some(&true)
            );
            assert_eq!(
                call(
                    metadata,
                    raw.clone(),
                    "Remove",
                    &[Some(Rc::new("null".to_owned()))]
                )
                .unwrap()
                .downcast_ref::<bool>(),
                Some(&false)
            );
            let original = model.model();
            call(metadata, raw, "Clear", &[]);
            assert_eq!(model.count(), 0);
            assert!(Rc::ptr_eq(&original, &model.model()));
        }};
    }
    check!(ContextLocatorMapReference, ContextFactoryReference);
    check!(HostWindowLocatorMapReference, HostWindowFactoryReference);
    check!(DockableLocatorMapReference, DockableFactoryReference);
}
struct TransientHostFactory {
    component: Factory,
    reads: Cell<usize>,
}
impl TransientHostFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self {
            component: Factory::construct(runtime.clone()),
            reads: Cell::new(0),
        })
    }
}
impl FactoryBaseImpl for TransientHostFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for TransientHostFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn default_host_window_locator(this: &Self) -> Option<HostWindowFactory> {
        let read = this.reads.get();
        this.reads.set(read + 1);
        if read == 0 {
            Some(Rc::new(|| None))
        } else {
            None
        }
    }
}
impl_factory!(TransientHostFactory);
#[test]
fn initialization_keeps_a_null_dictionary_entry_when_the_virtual_default_getter_changes() {
    let factory = TransientHostFactory::new();
    let root = factory.create_root_dock();
    factory.init_layout(root);
    assert!(factory
        .host_window_locator()
        .unwrap()
        .borrow()
        .get("IDockWindow")
        .unwrap()
        .is_none());
    assert!(factory.reads.get() >= 2);
}
#[test]
fn delegate_adapters_reject_incompatible_known_signatures_without_invoking_them() {
    register_types();
    let factory: Rc<dyn IFactory> = Factory::new();
    let value: BoxedValue = Rc::new(factory);
    let metadata = <dyn IFactoryMetadata as MarkupTyped>::MARKUP;
    let one_argument = MarkupDelegate::for_method(
        Some(value.clone()),
        metadata,
        metadata.find_methods("GetContext").next().unwrap(),
    );
    assert!(ContextFactoryReference::from_delegate(one_argument).is_none());
    let wrong_return = MarkupDelegate::for_method(
        Some(value),
        metadata,
        metadata.find_methods("CreateLayout").next().unwrap(),
    );
    assert!(HostWindowFactoryReference::from_delegate(wrong_return.clone()).is_none());
    assert!(ContextFactoryReference::from_delegate(wrong_return).is_some());
}
#[test]
fn dictionary_replacement_removal_and_clear_release_captured_values_after_borrowing_ends() {
    register_types();
    struct OnDrop(
        std::rc::Weak<
            RefCell<
                ferroui_system::collections::generic::Dictionary<String, Option<ContextFactory>>,
            >,
        >,
        Rc<Cell<usize>>,
    );
    impl Drop for OnDrop {
        fn drop(&mut self) {
            assert!(self.0.upgrade().unwrap().try_borrow_mut().is_ok());
            self.1.set(self.1.get() + 1);
        }
    }
    let map = ContextLocatorMapReference::create();
    let count = Rc::new(Cell::new(0));
    for operation in [0, 1, 2] {
        let captured = OnDrop(Rc::downgrade(&map.model()), count.clone());
        let callback: ContextFactory = Rc::new(move || {
            let _keep = &captured;
            None
        });
        map.set(
            "callback".into(),
            Some(ContextFactoryReference::new(callback)),
        );
        match operation {
            0 => map.set("callback".into(), None),
            1 => {
                assert!(map.remove("callback".into()));
            }
            _ => map.clear(),
        }
    }
    assert_eq!(count.get(), 3);
}

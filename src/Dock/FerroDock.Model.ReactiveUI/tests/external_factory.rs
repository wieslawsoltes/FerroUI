//! Factory composition and typed overrides from an actual downstream crate.
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::{BoxedValue, FactoryBase, FactoryBaseImpl};
use ferrodock_model_reactiveui::core::{DockableBase, DockableBaseImpl};
use ferrodock_model_reactiveui::{impl_dockable, impl_factory, Factory, FactoryImpl};
use ferroui_reactiveui::reactive_object;
use ferroui_system::collections::generic::Dictionary;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

struct ApplicationFactory {
    component: Factory,
    list_calls: Cell<usize>,
    root_calls: Cell<usize>,
    tool_calls: Cell<usize>,
    added: RefCell<Vec<String>>,
    closed: Cell<usize>,
    locator_calls: Cell<usize>,
}
impl ApplicationFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|runtime: &Weak<Self>| Self {
            component: Factory::construct(runtime.clone()),
            list_calls: Cell::new(0),
            root_calls: Cell::new(0),
            tool_calls: Cell::new(0),
            added: RefCell::new(Vec::new()),
            closed: Cell::new(0),
            locator_calls: Cell::new(0),
        })
    }
}
impl FactoryBaseImpl for ApplicationFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
    fn close_dockable_core(&self, item: Rc<dyn IDockable>) {
        self.closed.set(self.closed.get() + 1);
        self.component.base().close_dockable_core(item);
    }
}
impl FactoryImpl for ApplicationFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
    fn create_list<T: Clone + PartialEq + 'static>(this: &Self, items: Vec<T>) -> ListHandle<T> {
        this.list_calls.set(this.list_calls.get() + 1);
        ListHandle::from_plain(items)
    }
    fn create_root_dock(this: &Self) -> Rc<dyn IRootDock> {
        this.root_calls.set(this.root_calls.get() + 1);
        let root = this.component.create_root_dock();
        root.set_id("application-root".into());
        root
    }
    fn create_tool(this: &Self) -> Rc<dyn ITool> {
        this.tool_calls.set(this.tool_calls.get() + 1);
        let tool = this.component.create_tool();
        tool.set_id("application-tool".into());
        tool
    }
    fn get_dockable<T: IDockable + ?Sized + 'static>(this: &Self, id: String) -> Option<Rc<T>> {
        this.locator_calls.set(this.locator_calls.get() + 1);
        this.component.base().get_dockable(id)
    }
    fn on_dockable_added(this: &Self, item: Option<Rc<dyn IDockable>>) {
        this.added.borrow_mut().push(item.as_ref().unwrap().id());
        this.component.base().on_dockable_added(item);
    }
}
impl_factory!(ApplicationFactory);

struct ApplicationLeaf {
    base: DockableBase,
    marker: u32,
}
reactive_object!(ApplicationLeaf, base);
impl DockableBaseImpl for ApplicationLeaf {}
trait ApplicationItem: IDockable {
    fn marker(&self) -> u32;
}
impl ApplicationItem for ApplicationLeaf {
    fn marker(&self) -> u32 {
        self.marker
    }
}
impl_dockable!(
    ApplicationLeaf,
    fn boxed_interface(self: Rc<Self>, target: std::any::TypeId) -> Option<BoxedValue> {
        if target == std::any::TypeId::of::<dyn ApplicationItem>() {
            Some(Rc::new(self as Rc<dyn ApplicationItem>))
        } else {
            None
        }
    }
);
impl ApplicationLeaf {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|runtime: &Weak<Self>| Self {
            base: DockableBase::construct(runtime.clone()),
            marker: 123,
        })
    }
}

#[test]
fn downstream_layout_and_pinned_lists_dispatch_virtual_root_and_generic_list_creation() {
    let factory = ApplicationFactory::new();
    let root = factory.create_layout().unwrap();
    assert_eq!(root.id(), "application-root");
    assert_eq!(factory.root_calls.get(), 1);
    assert_eq!(factory.list_calls.get(), 4);
    for list in [
        root.left_pinned_dockables(),
        root.right_pinned_dockables(),
        root.top_pinned_dockables(),
        root.bottom_pinned_dockables(),
    ] {
        assert!(list.unwrap().as_notify_collection_changed().is_none());
    }
    let values = factory.create_list(vec![1_i32, 2_i32]);
    assert_eq!(values.to_vec(), [1, 2]);
    assert_eq!(factory.list_calls.get(), 5);
}

#[test]
fn erased_dockable_list_specialization_preserves_the_downstream_generic_override() {
    let factory = ApplicationFactory::new();
    let erased: Rc<dyn IFactory> = factory.clone();
    let item: Rc<dyn IDockable> = ApplicationLeaf::new();
    let list = erased.create_dockable_list(vec![Some(item.clone())]);
    assert_eq!(factory.list_calls.get(), 1);
    assert!(list.as_notify_collection_changed().is_none());
    assert!(Rc::ptr_eq(
        list.get(0).as_ref().expect("nonnull erased-list fixture"),
        &item
    ));
    assert!(erased.as_factory_base().is_some());
}

#[test]
fn downstream_public_and_protected_overrides_run_through_engine_and_parameter_adapter_calls() {
    let factory = ApplicationFactory::new();
    let root = factory.create_root_dock();
    factory.init_layout(root.clone());
    let tool = factory.create_tool();
    factory.add_dockable(Some(root.clone()), Some(tool.clone()));
    assert_eq!(factory.tool_calls.get(), 1);
    assert_eq!(*factory.added.borrow(), ["application-tool"]);
    let erased: Rc<dyn IFactory> = factory.clone();
    assert!(Rc::ptr_eq(&tool.factory().unwrap(), &erased));
    erased.close_dockable_parameter(Some(Rc::new(tool.clone() as Rc<dyn IDockable>)));
    assert_eq!(factory.closed.get(), 1);
    assert_eq!(root.visible_dockables().unwrap().count(), 0);
}

#[test]
fn downstream_concrete_locator_uses_the_original_custom_model_reference() {
    let factory = ApplicationFactory::new();
    let model = ApplicationLeaf::new();
    let located = model.clone();
    let registry: DockableLocatorMap = Rc::new(RefCell::new(Dictionary::new()));
    registry.borrow_mut().insert(
        "custom".into(),
        Some(Rc::new(move || Some(located.clone()))),
    );
    factory.set_dockable_locator(Some(registry));
    let actual = factory
        .get_dockable::<ApplicationLeaf>("custom".into())
        .unwrap();
    assert!(Rc::ptr_eq(&actual, &model));
    assert_eq!(actual.marker, 123);
    assert!(Rc::ptr_eq(
        &factory
            .get_dockable::<dyn IDockable>("custom".into())
            .unwrap(),
        &(model as Rc<dyn IDockable>)
    ));
    assert!(factory.get_dockable::<dyn ITool>("custom".into()).is_none());
}

#[test]
fn production_factory_has_no_default_ownership_cycle_through_models_or_creation_thunks() {
    let factory = Factory::new();
    let weak = Rc::downgrade(&factory);
    let root = factory.create_root_dock();
    factory.init_layout(root.clone());
    drop(factory);
    assert!(weak.upgrade().is_none());
    assert!(root.factory().is_none());
}

#[test]
fn downstream_application_interface_locator_casts_preserve_identity_through_registered_hooks() {
    let factory = Factory::new();
    let model = ApplicationLeaf::new();
    let located = model.clone();
    let registry: DockableLocatorMap = Rc::new(RefCell::new(Dictionary::new()));
    registry.borrow_mut().insert(
        "custom".into(),
        Some(Rc::new(move || Some(located.clone()))),
    );
    factory.set_dockable_locator(Some(registry));
    let actual = factory
        .get_dockable::<dyn ApplicationItem>("custom".into())
        .unwrap();
    assert_eq!(actual.marker(), 123);
    assert!(Rc::ptr_eq(
        &(actual as Rc<dyn IDockable>),
        &(model as Rc<dyn IDockable>)
    ));
}

#[test]
fn downstream_factory_can_supply_host_interface_membership_without_id_locator_fallback() {
    struct HostFactory {
        component: Factory,
        requests: Cell<usize>,
    }
    impl HostFactory {
        fn new() -> Rc<Self> {
            Rc::new_cyclic(|runtime: &Weak<Self>| Self {
                component: Factory::construct(runtime.clone()),
                requests: Cell::new(0),
            })
        }
    }
    impl FactoryBaseImpl for HostFactory {
        fn factory_base(&self) -> &FactoryBase {
            self.component.base()
        }
    }
    impl FactoryImpl for HostFactory {
        fn factory(&self) -> &Factory {
            &self.component
        }
        fn as_dock_window_host_factory(this: Rc<Self>) -> Option<Rc<dyn IDockWindowHostFactory>> {
            Some(this)
        }
    }
    impl IDockWindowHostFactory for HostFactory {
        fn create_host_window(
            &self,
            _window: Option<Rc<dyn IDockWindow>>,
        ) -> Option<Rc<dyn IHostWindow>> {
            self.requests.set(self.requests.get() + 1);
            None
        }
    }
    impl_factory!(HostFactory);
    let factory = HostFactory::new();
    factory.set_default_host_window_locator(Some(Rc::new(|| {
        panic!("host interface returned null: no fallback")
    })));
    let window = factory.create_dock_window();
    factory.init_dock_window(Some(window.clone()), None);
    assert_eq!(factory.requests.get(), 1);
    assert!(window.host().is_none());
    let _: Option<BoxedValue> = factory.get_context(String::new());
}

#[test]
fn fluent_helpers_preserve_downstream_factory_overrides_and_typed_model_handles() {
    use ferrodock_model::fluent_extensions::*;
    let factory = ApplicationFactory::new();
    let root = factory.clone().root_dock(None);
    assert_eq!(root.id(), "application-root");
    assert_eq!(factory.root_calls.get(), 1);
    assert_eq!(factory.list_calls.get(), 4);
    let runtime: Rc<dyn IFactory> = factory.clone();
    let mut tool = None;
    let returned = runtime.clone().tool_into(
        &mut tool,
        Some(&|model| {
            assert_eq!(model.id(), "application-tool");
            model.set_title("configured by application".into());
        }),
    );
    assert!(Rc::ptr_eq(&runtime, &returned));
    assert_eq!(factory.tool_calls.get(), 1);
    let tool: Rc<dyn IDockable> = tool.unwrap();
    let root = root
        .with_visible_dockables([Some(tool.clone())])
        .with_active_dockable(Some(tool.clone()));
    assert_eq!(factory.list_calls.get(), 4); // fluent fallback is a plain List, not virtual CreateList
    assert!(root
        .visible_dockables()
        .unwrap()
        .as_observable_collection()
        .is_none());
    assert!(tool.owner().is_none());
    assert_eq!(tool.title(), "configured by application");
    let model = ApplicationLeaf::new();
    let same: Rc<ApplicationLeaf> = model
        .clone()
        .with_id("typed".into())
        .with_title("downstream".into());
    assert!(Rc::ptr_eq(&model, &same));
    assert_eq!(same.id(), "typed");
}

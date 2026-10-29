use super::*;
use crate::overlays::hosting::hosting_test_support::Screen;
use crate::test_environment::Environment;
use ferroui_splat::{
    DependencyResolverMixins, ErasedService, LazyDisposal, ModernDependencyResolver,
    RegistrationCallback, ServiceFactory, ServiceType,
};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
struct Container(Rc<ModernDependencyResolver>);
impl Container {
    fn new() -> Self {
        Self(Rc::new(ModernDependencyResolver::new()))
    }
}
impl Drop for Container {
    fn drop(&mut self) {
        self.0.dispose();
    }
}
struct LocatorScope(Rc<dyn IDisposable>);
impl LocatorScope {
    fn new(resolver: &Rc<ModernDependencyResolver>) -> Self {
        Self(resolver.with_resolver())
    }
}
impl Drop for LocatorScope {
    fn drop(&mut self) {
        self.0.dispose();
    }
}
fn same<A: ?Sized, B: ?Sized>(a: &Rc<A>, b: &Rc<B>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}
fn required<S: ?Sized + 'static>(resolver: &ModernDependencyResolver) -> Rc<S> {
    resolver
        .get_service::<S>()
        .expect("registered test service")
}
fn singleton<S: ?Sized + 'static>(resolver: &ModernDependencyResolver) {
    assert!(same(&required::<S>(resolver), &required::<S>(resolver)));
}
fn keys() -> [ServiceType; 9] {
    [
        ServiceType::of::<dyn IDockGlobalBusyService>(),
        ServiceType::of::<dyn IDockGlobalDialogService>(),
        ServiceType::of::<dyn IDockGlobalConfirmationService>(),
        ServiceType::of::<dyn IHostServiceResolver>(),
        ServiceType::of::<HostOverlayServicesFactoryFn>(),
        ServiceType::of::<dyn IHostOverlayServicesProvider>(),
        ServiceType::of::<dyn IWindowLifecycleService>(),
        ServiceType::of::<dyn IActivationBridge>(),
        ServiceType::of::<dyn IDockDispatcher>(),
    ]
}
#[test]
fn all_nine_defaults_are_lazy_singletons_registered_in_source_order_and_repeat_registration_is_noop(
) {
    let _env = Environment::new();
    let container = Container::new();
    let services = &container.0;
    let order = Rc::new(RefCell::new(Vec::new()));
    for key in keys() {
        let seen = order.clone();
        services.service_registration_callback_erased(
            key,
            None,
            Rc::new(move |_| seen.borrow_mut().push(key)),
        );
    }
    let result = services.register_dock_overlay_services();
    assert!(Rc::ptr_eq(&result, services));
    assert_eq!(&*order.borrow(), &keys());
    services.register_dock_overlay_services();
    assert_eq!(order.borrow().len(), 9);
    for key in keys() {
        assert!(services.has_registration_erased(key, None));
        assert_eq!(services.get_services_erased(key, None).len(), 1);
    }
    singleton::<dyn IDockGlobalBusyService>(services);
    singleton::<dyn IDockGlobalDialogService>(services);
    singleton::<dyn IDockGlobalConfirmationService>(services);
    singleton::<dyn IHostServiceResolver>(services);
    singleton::<HostOverlayServicesFactoryFn>(services);
    singleton::<dyn IHostOverlayServicesProvider>(services);
    singleton::<dyn IWindowLifecycleService>(services);
    singleton::<dyn IActivationBridge>(services);
    singleton::<dyn IDockDispatcher>(services);
}
#[test]
fn registration_preserves_existing_factories_without_resolving_and_named_contract_does_not_hide_missing_default(
) {
    let _env = Environment::new();
    let container = Container::new();
    let services = &container.0;
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    let custom = DockGlobalBusyService::new(Some("custom".into()), None);
    let captured = custom.clone();
    services.register::<dyn IDockGlobalBusyService>(move || {
        seen.set(seen.get() + 1);
        Some(captured.clone())
    });
    let named = DockGlobalDialogService::new(None, None);
    services.register_constant_with_contract::<dyn IDockGlobalDialogService>(
        named.clone(),
        Some("named"),
    );
    services.register_dock_overlay_services();
    assert_eq!(count.get(), 0);
    assert!(same(
        &required::<dyn IDockGlobalBusyService>(services),
        &custom
    ));
    assert_eq!(count.get(), 1);
    assert!(!same(
        &required::<dyn IDockGlobalDialogService>(services),
        &named
    ));
    assert!(same(
        &services
            .get_service_with_contract::<dyn IDockGlobalDialogService>(Some("named"))
            .unwrap(),
        &named
    ));
    services.register_dock_overlay_services();
    assert_eq!(count.get(), 1);
    assert_eq!(
        services.get_services::<dyn IDockGlobalBusyService>().len(),
        1
    );
    assert_eq!(count.get(), 2);
}
#[test]
fn factory_creates_distinct_local_services_sharing_all_three_global_services_and_provider_caches_per_screen(
) {
    let _env = Environment::new();
    let container = Container::new();
    let services = &container.0;
    services.register_dock_overlay_services();
    let factory = required::<HostOverlayServicesFactoryFn>(services);
    let a = factory();
    let b = factory();
    assert!(!same(&a, &b));
    assert!(!same(&a.busy(), &b.busy()));
    assert!(!same(&a.dialogs(), &b.dialogs()));
    assert!(!same(&a.confirmations(), &b.confirmations()));
    assert!(same(
        &a.global_busy_service(),
        &required::<dyn IDockGlobalBusyService>(services)
    ));
    assert!(same(
        &a.global_dialog_service(),
        &required::<dyn IDockGlobalDialogService>(services)
    ));
    assert!(same(
        &a.global_confirmation_service(),
        &required::<dyn IDockGlobalConfirmationService>(services)
    ));
    assert!(same(&a.global_busy_service(), &b.global_busy_service()));
    assert!(same(&a.global_dialog_service(), &b.global_dialog_service()));
    assert!(same(
        &a.global_confirmation_service(),
        &b.global_confirmation_service()
    ));
    let busy = a.busy().begin("local".into());
    assert!(a.busy().is_busy());
    assert!(!b.busy().is_busy());
    assert!(b.global_busy_service().is_busy());
    busy.dispose();
    assert!(!b.global_busy_service().is_busy());
    let pending = a
        .confirmations()
        .confirm_async("t".into(), "m".into(), "y".into(), "n".into());
    assert!(b.global_confirmation_service().is_confirmation_open());
    a.confirmations().cancel_all();
    assert!(!pending.result().unwrap());
    assert!(!b.global_confirmation_service().is_confirmation_open());
    let provider = required::<dyn IHostOverlayServicesProvider>(services);
    let first = Screen::new("first");
    let second = Screen::new("second");
    let first_services = provider.get_services(first.clone());
    assert!(same(&first_services, &provider.get_services(first)));
    assert!(!same(&first_services, &provider.get_services(second)));
}
#[test]
fn resolved_host_factory_requeries_globals_in_original_six_argument_order_instead_of_prefetching() {
    let _env = Environment::new();
    let container = Container::new();
    let services = &container.0;
    let log = Rc::new(RefCell::new(Vec::new()));
    let busy_values = Rc::new(RefCell::new(Vec::new()));
    let seen = log.clone();
    let captured = busy_values.clone();
    services.register::<dyn IDockGlobalBusyService>(move || {
        seen.borrow_mut().push("busy");
        let value = DockGlobalBusyService::new(None, None);
        captured.borrow_mut().push(value.clone());
        Some(value)
    });
    let seen = log.clone();
    services.register::<dyn IDockGlobalDialogService>(move || {
        seen.borrow_mut().push("dialogs");
        Some(DockGlobalDialogService::new(None, None))
    });
    let seen = log.clone();
    services.register::<dyn IDockGlobalConfirmationService>(move || {
        seen.borrow_mut().push("confirmations");
        Some(DockGlobalConfirmationService::new(None, None))
    });
    services.register_dock_overlay_services();
    let factory = required::<HostOverlayServicesFactoryFn>(services);
    assert!(log.borrow().is_empty());
    let overlay = factory();
    assert_eq!(
        &*log.borrow(),
        &[
            "busy",
            "dialogs",
            "confirmations",
            "busy",
            "dialogs",
            "confirmations"
        ]
    );
    assert!(same(
        &overlay.global_busy_service(),
        &busy_values.borrow()[1]
    ));
    let busy = overlay.busy().begin("local".into());
    assert!(busy_values.borrow()[0].is_busy());
    assert!(!busy_values.borrow()[1].is_busy());
    busy.dispose();
}
#[test]
fn existing_null_registration_is_not_replaced_and_readonly_missing_service_does_not_fall_back_to_locator(
) {
    let _env = Environment::new();
    let global = Container::new();
    global.0.register_dock_overlay_services();
    let _scope = LocatorScope::new(&global.0);
    let local = Container::new();
    local.0.register::<dyn IDockGlobalBusyService>(|| None);
    local.0.register_dock_overlay_services();
    let factory = required::<HostOverlayServicesFactoryFn>(&local.0);
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| factory()))
        .err()
        .expect("missing required service");
    assert_eq!(
        failure.downcast_ref::<String>().unwrap(),
        &format!(
            "Service '{}' is not registered.",
            std::any::type_name::<dyn IDockGlobalBusyService>()
        )
    );
    assert_eq!(
        local
            .0
            .get_services_erased(ServiceType::of::<dyn IDockGlobalBusyService>(), None)
            .len(),
        0
    );
    local.0.unregister_all::<dyn IDockGlobalBusyService>();
    let replacement = DockGlobalBusyService::new(None, None);
    local
        .0
        .register_constant::<dyn IDockGlobalBusyService>(replacement.clone());
    assert!(same(&factory().global_busy_service(), &replacement));
}
struct MutableOnly {
    inner: Rc<ModernDependencyResolver>,
}
impl IMutableDependencyResolver for MutableOnly {
    fn has_registration_erased(&self, t: ServiceType, c: Option<&str>) -> bool {
        self.inner.has_registration_erased(t, c)
    }
    fn register_erased(&self, f: ServiceFactory, t: ServiceType, c: Option<&str>) {
        self.inner.register_erased(f, t, c);
    }
    fn register_constant_erased(
        &self,
        v: Option<ErasedService>,
        d: Option<Rc<dyn Fn()>>,
        t: ServiceType,
        c: Option<&str>,
    ) {
        self.inner.register_constant_erased(v, d, t, c);
    }
    fn register_lazy_singleton_erased(
        &self,
        f: ServiceFactory,
        d: Option<LazyDisposal>,
        t: ServiceType,
        c: Option<&str>,
    ) {
        self.inner.register_lazy_singleton_erased(f, d, t, c);
    }
    fn unregister_current_erased(&self, t: ServiceType, c: Option<&str>) {
        self.inner.unregister_current_erased(t, c);
    }
    fn unregister_all_erased(&self, t: ServiceType, c: Option<&str>) {
        self.inner.unregister_all_erased(t, c);
    }
    fn service_registration_callback_erased(
        &self,
        t: ServiceType,
        c: Option<&str>,
        cb: RegistrationCallback,
    ) -> Rc<dyn IDisposable> {
        self.inner.service_registration_callback_erased(t, c, cb)
    }
}
#[test]
fn mutable_only_branch_reads_current_locator_on_each_factory_invocation_not_local_registrations() {
    let _env = Environment::new();
    let a = Container::new();
    let b = Container::new();
    a.0.register_dock_overlay_services();
    b.0.register_dock_overlay_services();
    let local = Container::new();
    let mutable = Rc::new(MutableOnly {
        inner: local.0.clone(),
    });
    let result =
        DependencyResolverExtensions::register_dock_overlay_services_mutable_only(mutable.clone());
    assert!(same(&result, &mutable));
    let factory = required::<HostOverlayServicesFactoryFn>(&local.0);
    let first = {
        let _scope = LocatorScope::new(&a.0);
        factory()
    };
    let second = {
        let _scope = LocatorScope::new(&b.0);
        factory()
    };
    assert!(same(
        &first.global_busy_service(),
        &required::<dyn IDockGlobalBusyService>(&a.0)
    ));
    assert!(same(
        &second.global_busy_service(),
        &required::<dyn IDockGlobalBusyService>(&b.0)
    ));
    assert!(!same(
        &first.global_busy_service(),
        &second.global_busy_service()
    ));
    assert!(!same(
        &first.global_busy_service(),
        &required::<dyn IDockGlobalBusyService>(&local.0)
    ));
}
#[test]
fn erased_dual_interface_extension_preserves_original_handle_and_rejects_foreign_readonly_projection(
) {
    let _env = Environment::new();
    let container = Container::new();
    let erased: Rc<dyn IDependencyResolver> = container.0.clone();
    assert!(same(&erased.register_dock_overlay_services(), &erased));
    let factory = required::<HostOverlayServicesFactoryFn>(&container.0);
    assert!(same(
        &factory().global_busy_service(),
        &required::<dyn IDockGlobalBusyService>(&container.0)
    ));
    let untouched = Container::new();
    let other = Container::new();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        DependencyResolverExtensions::register_dock_overlay_services_with_readonly(
            untouched.0.clone(),
            Some(other.0.clone()),
        )
    }))
    .is_err());
    for key in keys() {
        assert!(!untouched.0.has_registration_erased(key, None));
    }
}
#[test]
fn duplicated_registration_and_resolved_func_retain_the_original_source_resolution_context() {
    let _env = Environment::new();
    let resolver = Rc::new(ModernDependencyResolver::new());
    let weak = Rc::downgrade(&resolver);
    resolver.register_dock_overlay_services();
    let original_global = required::<dyn IDockGlobalBusyService>(&resolver);
    let duplicate = Rc::new(resolver.duplicate());
    duplicate
        .register_constant::<dyn IDockGlobalBusyService>(DockGlobalBusyService::new(None, None));
    drop(resolver);
    // Copied registrations close over their ORIGINAL resolver.
    let factory = required::<HostOverlayServicesFactoryFn>(&duplicate);
    let overlay = factory();
    assert!(same(&overlay.global_busy_service(), &original_global));
    assert!(!same(
        &overlay.global_busy_service(),
        &required::<dyn IDockGlobalBusyService>(&duplicate)
    ));
    weak.upgrade().unwrap().dispose();
    duplicate.dispose();
    drop(overlay);
    drop(factory);
    drop(duplicate);
    assert!(weak.upgrade().is_none());
}
#[test]
fn preexisting_all_nine_services_are_preserved_after_registration() {
    let _env = Environment::new();
    let container = Container::new();
    let services = &container.0;
    services.register_dock_overlay_services();
    let originals: Vec<_> = keys()
        .into_iter()
        .map(|key| (key, services.get_service_erased(key, None).unwrap()))
        .collect();
    let other = Container::new();
    for (key, value) in &originals {
        other
            .0
            .register_constant_erased(Some(value.clone()), None, *key, None);
    }
    other.0.register_dock_overlay_services();
    for (key, value) in &originals {
        assert!(Rc::ptr_eq(
            value,
            &other.0.get_service_erased(*key, None).unwrap()
        ));
        assert_eq!(other.0.get_services_erased(*key, None).len(), 1);
    }
}

#[test]
fn registered_services_integrate_busy_root_lookup_and_factory_window_cleanup() {
    use crate::overlays::controls::BusyRootDock;
    use ferrodock_model::core::IFactoryEvents;
    use ferrodock_model_reactiveui::core::DockWindow;
    use ferrodock_model_reactiveui::Factory;
    let _env = Environment::new();
    let container = Container::new();
    let services = &container.0;
    services.register_dock_overlay_services();
    let resolver = required::<dyn IHostServiceResolver>(services);
    let types = resolver.native_types().unwrap();
    let host = Screen::new("host");
    let root = BusyRootDock::with_types(
        host,
        required::<HostOverlayServicesFactoryFn>(services),
        None,
        types,
    );
    let provider = required::<dyn IHostOverlayServicesProvider>(services);
    assert!(same(
        &provider.get_services(root.clone()).busy(),
        &root.busy()
    ));
    let dialog = root
        .dialogs()
        .show_async_boxed(DialogContent::new(Rc::new(5)), None);
    let confirmation = root.confirmations().confirm_async(
        "title".into(),
        "message".into(),
        "ok".into(),
        "cancel".into(),
    );
    let factory = Factory::new();
    factory
        .base()
        .window_lifecycle_services()
        .add(Some(required::<dyn IWindowLifecycleService>(services)));
    let before = Rc::new(Cell::new(false));
    let seen = before.clone();
    let probe = root.clone();
    let _event = factory.window_closed(Rc::new(move |_| {
        seen.set(probe.dialogs().has_dialogs() && probe.confirmations().has_confirmations())
    }));
    let changes = Rc::new(Cell::new(0));
    let seen = changes.clone();
    root.busy().property_changed().add(Rc::new(move |name| {
        if name == "CanReload" {
            seen.set(seen.get() + 1);
        }
    }));
    let window = DockWindow::new();
    window.set_layout(Some(root.clone()));
    factory.on_window_closed(Some(window.clone()));
    factory.on_window_removed(Some(window));
    assert!(before.get());
    assert!(dialog.result().unwrap().is_none());
    assert!(!confirmation.result().unwrap());
    assert!(!root.busy().can_reload());
    assert_eq!(changes.get(), 1);
    assert!(!root.dialogs().has_dialogs());
    assert!(!root.confirmations().has_confirmations());
}

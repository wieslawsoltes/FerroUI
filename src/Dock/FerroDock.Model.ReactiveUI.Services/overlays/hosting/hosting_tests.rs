use super::hosting_test_support::*;
use super::*;
use crate::lifecycle::DockWindowLifecycleService;
use crate::test_environment::Environment;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::core::*;
use ferrodock_model::services::*;
use ferrodock_model_reactiveui::controls::{Document, RootDock};
use ferrodock_model_reactiveui::core::DockWindow;
use ferroui_reactiveui::IScreen;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};

#[test]
fn registry_preserves_concrete_interface_identity_rejects_false_membership_and_does_not_retain_objects(
) {
    let _environment = Environment::new();
    let types = HostTypeRegistry::new();
    let screen = ScreenRoot::new("screen");
    ScreenRoot::register(&screen, &types);
    types.register_interface(&screen, screen.clone() as Rc<dyn Marker>);
    let resolver = OwnerChainHostServiceResolver::with_types(types.clone());
    let typed = resolver.resolve::<ScreenRoot>(screen.clone()).unwrap();
    assert!(Rc::ptr_eq(&typed, &screen));
    let service = resolver.resolve::<dyn Marker>(screen.clone()).unwrap();
    assert_eq!(service.label(), "screen");
    let other = ScreenRoot::new("other");
    assert!(catch_unwind(AssertUnwindSafe(
        || types.register_interface(&screen, other.clone() as Rc<dyn Marker>)
    ))
    .is_err());
    drop(service);
    drop(typed);
    let weak = Rc::downgrade(&screen);
    drop(screen);
    types.prune();
    assert!(weak.upgrade().is_none());
}
#[test]
fn resolver_prefers_last_owner_then_router_reverse_order_and_stops_host_cycles() {
    let _environment = Environment::new();
    let resolver = OwnerChainHostServiceResolver::new();
    let types = resolver.types();
    let inner = ScreenRoot::new("inner");
    let outer = ScreenRoot::new("outer");
    ScreenRoot::register(&inner, &types);
    ScreenRoot::register(&outer, &types);
    inner.set_owner(Some(outer.clone()));
    assert_eq!(
        resolver
            .resolve::<dyn Marker>(inner.clone())
            .unwrap()
            .label(),
        "outer"
    );
    let host = Screen::new("plain");
    types.register_screen(&host);
    host.router.navigation_stack().add(inner.clone());
    host.router.navigation_stack().add(outer.clone());
    assert_eq!(
        resolver
            .resolve::<dyn Marker>(host.clone())
            .unwrap()
            .label(),
        "outer"
    );
    host.router.navigation_stack().clear();
    inner.set_owner(None);
    inner
        .host
        .replace(Some(Rc::downgrade(&(outer.clone() as Rc<dyn IScreen>))));
    outer
        .host
        .replace(Some(Rc::downgrade(&(inner.clone() as Rc<dyn IScreen>))));
    assert!(resolver
        .resolve::<dyn IHostOverlayServices>(inner)
        .is_none());
}
#[test]
fn resolver_host_window_precedes_owner_and_factory_scans_primary_then_floating_roots() {
    let _environment = Environment::new();
    let resolver = OwnerChainHostServiceResolver::new();
    let types = resolver.types();
    let factory = factory();
    let screen = ScreenRoot::new("owner");
    let hosted = ScreenRoot::new("hosted");
    ScreenRoot::register(&screen, &types);
    ScreenRoot::register(&hosted, &types);
    screen.set_factory(Some(factory.clone()));
    hosted.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(screen.clone())])));
    let window = DockWindow::new();
    window.set_layout(Some(hosted.clone()));
    let host = Host::new(window.clone());
    factory.host_windows().add(host.clone());
    assert_eq!(
        resolver
            .resolve::<dyn Marker>(screen.clone())
            .unwrap()
            .label(),
        "hosted"
    );
    assert_eq!(host.calls.get(), 1);
    factory.host_windows().clear();
    // FindInFactory can locate the screen even when its owner link is stale/null.
    // A screen root without IHostOverlayServices must be found through the actual factory.
    let plain = ScreenRoot::new("not-a-marker-projection");
    types.register_dockable_screen(&plain);
    plain.set_factory(Some(factory.clone()));
    let overlays = Overlays::new();
    let primary = OverlayRoot::new("primary", overlays.host.clone());
    OverlayRoot::register(&primary, &types);
    primary.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(plain.clone())])));
    factory.dock_controls().add(Control::new(primary.clone()));
    assert!(Rc::ptr_eq(
        &resolver
            .resolve::<dyn IHostOverlayServices>(plain.clone())
            .unwrap(),
        &(primary.clone() as Rc<dyn IHostOverlayServices>)
    ));
    primary.set_visible_dockables(None);
    let floating = OverlayRoot::new("floating", overlays.host.clone());
    OverlayRoot::register(&floating, &types);
    floating.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(plain.clone())])));
    let window = DockWindow::new();
    window.set_layout(Some(floating.clone()));
    primary.set_windows(Some(ListHandle::from_plain(vec![Some(window)])));
    assert!(Rc::ptr_eq(
        &resolver.resolve::<dyn IHostOverlayServices>(plain).unwrap(),
        &(floating as Rc<dyn IHostOverlayServices>)
    ));
}
#[test]
fn resolver_normalizes_unhosted_routed_root_to_outer_root_but_keeps_window_roots() {
    let _environment = Environment::new();
    let resolver = OwnerChainHostServiceResolver::new();
    let types = resolver.types();
    let inner = ScreenRoot::new("inner");
    let outer = ScreenRoot::new("outer");
    ScreenRoot::register(&inner, &types);
    ScreenRoot::register(&outer, &types);
    inner
        .host
        .replace(Some(Rc::downgrade(&(outer.clone() as Rc<dyn IScreen>))));
    assert_eq!(
        resolver
            .resolve::<dyn Marker>(inner.clone())
            .unwrap()
            .label(),
        "outer"
    );
    let window = DockWindow::new();
    inner.set_window(Some(window.clone()));
    assert_eq!(
        resolver
            .resolve::<dyn Marker>(inner.clone())
            .unwrap()
            .label(),
        "inner"
    );
    inner.set_window(None);
    outer
        .host
        .replace(Some(Rc::downgrade(&(inner.clone() as Rc<dyn IScreen>))));
    inner.host.replace(None);
    assert_eq!(
        resolver.resolve::<dyn Marker>(inner).unwrap().label(),
        "inner"
    );
}
#[test]
fn provider_caches_per_screen_evicts_on_resolution_and_constant_fallback_is_shared() {
    let _environment = Environment::new();
    let types = HostTypeRegistry::new();
    let resolver = Resolver::new(types);
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    let provider = HostOverlayServicesProvider::new(
        resolver.clone(),
        Rc::new(move || {
            seen.set(seen.get() + 1);
            Overlays::new().host
        }),
    );
    let a = Screen::new("a");
    let b = Screen::new("a");
    assert_eq!(a.name, b.name);
    assert!(provider.try_get_cached(a.clone()).is_none());
    let first = provider.get_services(a.clone());
    assert!(Rc::ptr_eq(&first, &provider.get_services(a.clone())));
    assert!(!Rc::ptr_eq(&first, &provider.get_services(b.clone())));
    assert_eq!(calls.get(), 2);
    let resolved = Overlays::new().host;
    resolver.value.replace(Some(resolved.clone()));
    assert!(Rc::ptr_eq(&resolved, &provider.get_services(a.clone())));
    assert!(provider.try_get_cached(a.clone()).is_none());
    resolver.value.replace(None);
    assert!(!Rc::ptr_eq(&first, &provider.get_services(a.clone())));
    assert_eq!(calls.get(), 3);
    let constant = HostOverlayServicesProvider::with_fallback(resolver, resolved.clone());
    assert!(Rc::ptr_eq(
        &constant.get_services(a),
        &constant.get_services(b)
    ));
}
#[test]
fn provider_reentrant_fallback_factory_retains_the_first_inserted_value() {
    let _environment = Environment::new();
    let resolver = Resolver::new(HostTypeRegistry::new());
    let screen = Screen::new("screen");
    let link = Rc::new(RefCell::new(None::<Weak<HostOverlayServicesProvider>>));
    let weak = link.clone();
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    let nested = Rc::new(RefCell::new(None));
    let result = nested.clone();
    let target = screen.clone();
    let provider = HostOverlayServicesProvider::new(
        resolver,
        Rc::new(move || {
            let count = seen.get();
            seen.set(count + 1);
            if count == 0 {
                result.replace(Some(
                    weak.borrow()
                        .as_ref()
                        .unwrap()
                        .upgrade()
                        .unwrap()
                        .get_services(target.clone()),
                ));
            }
            Overlays::new().host
        }),
    );
    link.replace(Some(Rc::downgrade(&provider)));
    let returned = provider.get_services(screen);
    assert_eq!(calls.get(), 2);
    assert!(Rc::ptr_eq(&returned, nested.borrow().as_ref().unwrap()));
}
#[test]
fn provider_dockable_context_uses_nearest_root_or_window_layout_before_screen_fallback() {
    let _environment = Environment::new();
    let types = HostTypeRegistry::new();
    let resolver = Resolver::new(types.clone());
    let fallback = Overlays::new();
    let provider =
        HostOverlayServicesProvider::with_fallback(resolver.clone(), fallback.host.clone());
    let host = Screen::new("host");
    let local = Overlays::new();
    let root = OverlayRoot::new("root", local.host.clone());
    OverlayRoot::register(&root, &types);
    let factory = factory();
    factory.init_layout(root.clone());
    root.set_is_focusable_root(false);
    let doc = Document::new();
    doc.set_owner(Some(root.clone()));
    assert!(Rc::ptr_eq(
        &provider.get_services_for_dockable(host.clone(), doc.clone()),
        &(root.clone() as Rc<dyn IHostOverlayServices>)
    ));
    let plain = ScreenRoot::new("plain");
    ScreenRoot::register(&plain, &types);
    factory.init_layout(plain.clone());
    doc.set_owner(Some(plain.clone()));
    let window = DockWindow::new();
    window.set_layout(Some(root.clone()));
    plain.set_window(Some(window.clone()));
    assert!(Rc::ptr_eq(
        &provider.get_services_for_dockable(host.clone(), doc.clone()),
        &(root as Rc<dyn IHostOverlayServices>)
    ));
    doc.set_owner(None);
    assert!(Rc::ptr_eq(
        &provider.get_services_for_dockable(host, doc),
        &fallback.host
    ));
}
#[test]
fn lifecycle_original_cleanup_and_idempotence_use_actual_services_and_layout() {
    let _environment = Environment::new();
    let types = HostTypeRegistry::new();
    let overlays = Overlays::new();
    let provider = Rc::new(Provider {
        host: overlays.host.clone(),
        types: types.clone(),
        calls: Cell::new(0),
    });
    let lifecycle = DockWindowLifecycleService::new(provider.clone(), None);
    let root = ScreenRoot::new("root");
    ScreenRoot::register(&root, &types);
    let window = DockWindow::new();
    window.set_layout(Some(root.clone()));
    let dialog = overlays
        .dialogs
        .show_async::<i32>(DialogContent::new(Rc::new("content")), None);
    let confirmation = overlays
        .confirmations
        .confirm_async_default("t".into(), "m".into());
    lifecycle.on_window_closed(Some(window.clone()));
    lifecycle.on_window_removed(Some(window));
    assert_eq!(overlays.dialogs.calls.get(), 1);
    assert_eq!(overlays.confirmations.calls.get(), 1);
    assert_eq!(root.disposed.get(), 1);
    assert_eq!(provider.calls.get(), 1);
    assert_eq!(&*overlays.log.borrow(), &["dialogs", "confirmations"]);
    assert_eq!(&*root.log.borrow(), &["dispose"]);
    assert_eq!(dialog.result().unwrap(), 0);
    assert!(!confirmation.result().unwrap());
}
#[test]
fn lifecycle_cached_provider_does_not_create_fallback_and_marks_before_missing_services() {
    let _environment = Environment::new();
    let types = HostTypeRegistry::new();
    let resolver = Resolver::new(types.clone());
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    let provider = HostOverlayServicesProvider::new(
        resolver,
        Rc::new(move || {
            seen.set(seen.get() + 1);
            Overlays::new().host
        }),
    );
    let lifecycle = DockWindowLifecycleService::new(provider.clone(), None);
    let root = ScreenRoot::new("root");
    ScreenRoot::register(&root, &types);
    let window = DockWindow::new();
    window.set_layout(Some(root.clone()));
    lifecycle.on_window_closed(Some(window.clone()));
    assert_eq!(calls.get(), 0);
    assert_eq!(root.disposed.get(), 0);
    provider.get_services(root.clone());
    assert_eq!(calls.get(), 1);
    lifecycle.on_window_removed(Some(window));
    assert_eq!(root.disposed.get(), 0);
    let next = DockWindow::new();
    next.set_layout(Some(root.clone()));
    lifecycle.on_window_closed(Some(next));
    assert_eq!(root.disposed.get(), 1);
    assert_eq!(calls.get(), 1);
}
#[test]
fn lifecycle_direct_then_resolver_then_provider_precedence_and_layout_reread_are_preserved() {
    let _environment = Environment::new();
    let types = HostTypeRegistry::new();
    let overlays = Overlays::new();
    let other = Overlays::new();
    let resolver = Resolver::new(types.clone());
    resolver.value.replace(Some(other.host.clone()));
    let provider = Rc::new(Provider {
        host: other.host.clone(),
        types: types.clone(),
        calls: Cell::new(0),
    });
    let lifecycle = DockWindowLifecycleService::new(provider.clone(), Some(resolver.clone()));
    let root = OverlayRoot::new("root", overlays.host.clone());
    OverlayRoot::register(&root, &types);
    let replacement = ScreenRoot::new("replacement");
    ScreenRoot::register(&replacement, &types);
    let window = DockWindow::new();
    window.set_layout(Some(root.clone()));
    let task = overlays
        .dialogs
        .show_async::<i32>(DialogContent::new(Rc::new("content")), None);
    let target = window.clone();
    let next = replacement.clone();
    task.continue_with(move |_| target.set_layout(Some(next)));
    lifecycle.on_window_closed(Some(window.clone()));
    assert_eq!(resolver.calls.get(), 0);
    assert_eq!(provider.calls.get(), 0);
    assert_eq!(overlays.dialogs.calls.get(), 1);
    assert_eq!(root.disposed.get(), 0);
    assert_eq!(replacement.disposed.get(), 1);
    let window = DockWindow::new();
    window.set_layout(Some(replacement));
    lifecycle.on_window_closed(Some(window));
    assert_eq!(resolver.calls.get(), 1);
    assert_eq!(provider.calls.get(), 0);
    assert_eq!(other.dialogs.calls.get(), 1);
}
#[test]
fn lifecycle_failure_is_marked_once_and_no_layout_or_nonscreen_does_not_mark_window() {
    let _environment = Environment::new();
    let types = HostTypeRegistry::new();
    let overlays = Overlays::new();
    let provider = Rc::new(Provider {
        host: overlays.host.clone(),
        types: types.clone(),
        calls: Cell::new(0),
    });
    let lifecycle = DockWindowLifecycleService::new(provider, None);
    let window = DockWindow::new();
    lifecycle.on_window_closed(None);
    lifecycle.on_window_closed(Some(window.clone()));
    let plain = RootDock::new();
    window.set_layout(Some(plain));
    lifecycle.on_window_closed(Some(window.clone()));
    let root = ScreenRoot::new("root");
    ScreenRoot::register(&root, &types);
    window.set_layout(Some(root.clone()));
    overlays.dialogs.fail.set(true);
    assert!(catch_unwind(AssertUnwindSafe(
        || lifecycle.on_window_closed(Some(window.clone()))
    ))
    .is_err());
    overlays.dialogs.fail.set(false);
    lifecycle.on_window_removed(Some(window));
    assert_eq!(overlays.dialogs.calls.get(), 1);
    assert_eq!(overlays.confirmations.calls.get(), 0);
    assert_eq!(root.disposed.get(), 0);
}

#[test]
fn provider_screen_dockable_resolution_evicts_that_screen_cache_and_retains_fallback_screen_cache()
{
    let _environment = Environment::new();
    let types = HostTypeRegistry::new();
    let resolver = Resolver::new(types.clone());
    let provider =
        HostOverlayServicesProvider::new(resolver.clone(), Rc::new(|| Overlays::new().host));
    let fallback = Screen::new("fallback");
    let dockable = ScreenRoot::new("dockable");
    ScreenRoot::register(&dockable, &types);
    let fallback_value = provider.get_services(fallback.clone());
    provider.get_services(dockable.clone());
    let resolved = Overlays::new().host;
    resolver.value.replace(Some(resolved.clone()));
    assert!(Rc::ptr_eq(
        &provider.get_services_for_dockable(fallback.clone(), dockable.clone()),
        &resolved
    ));
    assert!(provider.try_get_cached(dockable).is_none());
    assert!(Rc::ptr_eq(
        &provider.try_get_cached(fallback).unwrap(),
        &fallback_value
    ));
}
#[test]
fn provider_prunes_expired_noncyclic_screen_keys_and_releases_values_outside_borrows() {
    let _environment = Environment::new();
    let resolver = Resolver::new(HostTypeRegistry::new());
    let provider = HostOverlayServicesProvider::new(resolver, Rc::new(|| Overlays::new().host));
    let screen = Screen::new("expired");
    let value = provider.get_services(screen.clone());
    let weak_value = Rc::downgrade(&value);
    let weak_screen = Rc::downgrade(&screen);
    drop(value);
    drop(screen);
    assert!(weak_screen.upgrade().is_none());
    assert!(weak_value.upgrade().is_some());
    assert!(provider.try_get_cached(Screen::new("new")).is_none());
    assert!(weak_value.upgrade().is_none());
}
#[test]
fn lifecycle_is_marked_before_reentrant_request_completion_and_preserves_cancel_order() {
    let _environment = Environment::new();
    let types = HostTypeRegistry::new();
    let overlays = Overlays::new();
    let provider = Rc::new(Provider {
        host: overlays.host.clone(),
        types: types.clone(),
        calls: Cell::new(0),
    });
    let lifecycle = DockWindowLifecycleService::new(provider, None);
    let root = ScreenRoot::new("root");
    ScreenRoot::register(&root, &types);
    let window = DockWindow::new();
    window.set_layout(Some(root.clone()));
    let task = overlays
        .dialogs
        .show_async::<i32>(DialogContent::new(Rc::new("content")), None);
    let weak = Rc::downgrade(&lifecycle);
    let target = window.clone();
    task.continue_with(move |_| weak.upgrade().unwrap().on_window_removed(Some(target)));
    lifecycle.on_window_closed(Some(window));
    assert_eq!(overlays.dialogs.calls.get(), 1);
    assert_eq!(overlays.confirmations.calls.get(), 1);
    assert_eq!(root.disposed.get(), 1);
    assert_eq!(&*overlays.log.borrow(), &["dialogs", "confirmations"]);
}

#[test]
fn production_factory_window_events_invoke_lifecycle_after_event_delivery_only_once_per_window() {
    let _environment = Environment::new();
    let types = HostTypeRegistry::new();
    let overlays = Overlays::new();
    let provider = Rc::new(Provider {
        host: overlays.host.clone(),
        types: types.clone(),
        calls: Cell::new(0),
    });
    let lifecycle = DockWindowLifecycleService::new(provider, None);
    let factory = factory();
    factory
        .base()
        .window_lifecycle_services()
        .add(Some(lifecycle));
    let root = ScreenRoot::new("root");
    ScreenRoot::register(&root, &types);
    let window = DockWindow::new();
    window.set_layout(Some(root.clone()));
    let log = overlays.log.clone();
    let _subscription =
        factory.window_closed(Rc::new(move |_| log.borrow_mut().push("closed event")));
    factory.on_window_closed(Some(window.clone()));
    factory.on_window_removed(Some(window));
    assert_eq!(
        &*overlays.log.borrow(),
        &["closed event", "dialogs", "confirmations"]
    );
    assert_eq!(root.disposed.get(), 1);
}

// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::lifecycle::{DockActivationBridge, DockWindowLifecycleService, IActivationBridge};
use crate::overlays::hosting::*;
use crate::overlays::services::*;
use crate::threading::MainThreadDispatcher;
use ferrodock_model::services::*;
use ferrodock_model_reactiveui::services::IDockDispatcher;
use ferroui_splat::{
    IDependencyResolver, IMutableDependencyResolver, IReadonlyDependencyResolver, Locator,
    MutableDependencyResolverExt, ReadonlyDependencyResolverExt,
};
use std::rc::Rc;

/// Source Splat registration helpers. No local service container or locator is supplied.
pub struct DependencyResolverExtensions;
/// Native extension-method form for a resolver with known mutable/read-only projections.
pub trait DockOverlayResolverExt: Sized {
    /// Registers missing overlay services and returns the same resolver handle.
    fn register_dock_overlay_services(&self) -> Self;
}
impl<R: IMutableDependencyResolver + IReadonlyDependencyResolver + 'static> DockOverlayResolverExt
    for Rc<R>
{
    fn register_dock_overlay_services(&self) -> Self {
        DependencyResolverExtensions::register_dock_overlay_services(self.clone())
    }
}
impl DockOverlayResolverExt for Rc<dyn IDependencyResolver> {
    fn register_dock_overlay_services(&self) -> Self {
        DependencyResolverExtensions::register_dock_overlay_services_with_readonly(
            self.clone(),
            Some(self.clone()),
        );
        self.clone()
    }
}
impl DependencyResolverExtensions {
    /// Registers the nine source defaults, lazily, without replacing existing
    /// default-contract registrations or resolving dependencies during registration.
    pub fn register_dock_overlay_services<
        R: IMutableDependencyResolver + IReadonlyDependencyResolver + 'static,
    >(
        services: Rc<R>,
    ) -> Rc<R> {
        Self::register_dock_overlay_services_with_readonly(
            services.clone(),
            Some(services.clone()),
        );
        services
    }
    /// Native erased overload carrying the source runtime IReadonly cast explicitly.
    /// A supplied projection must be the SAME resolver allocation. None represents
    /// a mutable-only resolver and uses Locator.Current at each required lookup.
    pub fn register_dock_overlay_services_with_readonly(
        services: Rc<dyn IMutableDependencyResolver>,
        readonly: Option<Rc<dyn IReadonlyDependencyResolver>>,
    ) -> Rc<dyn IMutableDependencyResolver> {
        if let Some(readonly) = &readonly {
            assert!(
                std::ptr::addr_eq(Rc::as_ptr(&services), Rc::as_ptr(readonly)),
                "Read-only projection must refer to the same resolver."
            );
        }
        let access = Resolver {
            _mutable: services.clone(),
            readonly,
        };
        register_lazy_singleton_if_missing::<dyn IDockGlobalBusyService>(services.as_ref(), || {
            DockGlobalBusyService::new(None, None)
        });
        register_lazy_singleton_if_missing::<dyn IDockGlobalDialogService>(
            services.as_ref(),
            || DockGlobalDialogService::new(None, None),
        );
        register_lazy_singleton_if_missing::<dyn IDockGlobalConfirmationService>(
            services.as_ref(),
            || DockGlobalConfirmationService::new(None, None),
        );
        register_lazy_singleton_if_missing::<dyn IHostServiceResolver>(services.as_ref(), || {
            OwnerChainHostServiceResolver::new()
        });
        let resolve = access.clone();
        register_lazy_singleton_if_missing::<HostOverlayServicesFactoryFn>(
            services.as_ref(),
            move || {
                // Only the resolved Func retains the resolver, as the source closure
                // does. Registration callbacks hold a weak back-reference to avoid an
                // unresolvable native cycle before any singleton was even requested.
                let resolve = resolve.clone();
                Rc::new(move || {
                    HostOverlayServicesAdapter::new(
                        DockBusyService::new(
                            Some(resolve.required::<dyn IDockGlobalBusyService>()),
                            None,
                        ),
                        DockDialogService::new(
                            Some(resolve.required::<dyn IDockGlobalDialogService>()),
                            None,
                        ),
                        DockConfirmationService::new(
                            Some(resolve.required::<dyn IDockGlobalConfirmationService>()),
                            None,
                        ),
                        resolve.required::<dyn IDockGlobalBusyService>(),
                        resolve.required::<dyn IDockGlobalDialogService>(),
                        resolve.required::<dyn IDockGlobalConfirmationService>(),
                    )
                })
            },
        );
        let resolve = access.clone();
        register_lazy_singleton_if_missing::<dyn IHostOverlayServicesProvider>(
            services.as_ref(),
            move || {
                let resolve = resolve.clone();
                HostOverlayServicesProvider::new(
                    resolve.required::<dyn IHostServiceResolver>(),
                    resolve.required::<HostOverlayServicesFactoryFn>(),
                )
            },
        );
        register_lazy_singleton_if_missing::<dyn IWindowLifecycleService>(
            services.as_ref(),
            move || {
                let resolve = access.clone();
                DockWindowLifecycleService::new(
                    resolve.required::<dyn IHostOverlayServicesProvider>(),
                    Some(resolve.required::<dyn IHostServiceResolver>()),
                )
            },
        );
        register_lazy_singleton_if_missing::<dyn IActivationBridge>(services.as_ref(), || {
            DockActivationBridge::new()
        });
        register_lazy_singleton_if_missing::<dyn IDockDispatcher>(services.as_ref(), || {
            MainThreadDispatcher::new()
        });
        services
    }
    /// Native source mutable-only branch: required dependencies come from the
    /// CURRENT shared locator. Use with_readonly for an erased dual-interface resolver.
    pub fn register_dock_overlay_services_mutable_only(
        services: Rc<dyn IMutableDependencyResolver>,
    ) -> Rc<dyn IMutableDependencyResolver> {
        Self::register_dock_overlay_services_with_readonly(services, None)
    }
}
fn register_lazy_singleton_if_missing<S: ?Sized + 'static>(
    services: &dyn IMutableDependencyResolver,
    factory: impl Fn() -> Rc<S> + 'static,
) {
    if services.has_registration::<S>() {
        return;
    }
    services.register_lazy_singleton::<S>(move || Some(factory()));
}
// Source registration and Func closures retain their resolver. Explicit container
// disposal releases registrations; CLR-equivalent collection of otherwise
// unreachable resolver/closure cycles is still a shared-runtime parity gap.
#[derive(Clone)]
struct Resolver {
    _mutable: Rc<dyn IMutableDependencyResolver>,
    readonly: Option<Rc<dyn IReadonlyDependencyResolver>>,
}
impl Resolver {
    fn required<S: ?Sized + 'static>(&self) -> Rc<S> {
        let value = match &self.readonly {
            Some(resolver) => resolver.get_service::<S>(),
            None => Locator::current().get_service::<S>(),
        };
        value.unwrap_or_else(|| {
            panic!(
                "Service '{}' is not registered.",
                std::any::type_name::<S>()
            )
        })
    }
}
#[cfg(test)]
#[path = "dependency_resolver_extensions_tests.rs"]
mod tests;

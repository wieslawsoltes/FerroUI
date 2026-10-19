//! `MutableDependencyResolverAOTExtensions` (`ReactiveUI.Shared/Mixins/MutableDependencyResolverAOTExtensions.cs`).

use crate::{IViewForOf, RxAppBuilder};
use ferroui_splat::{IMutableDependencyResolver, MutableDependencyResolverExt};
use std::cell::Cell;
use std::rc::Rc;

ferroui_system::ferro_thread_local! {
    /// Whether the static constructor of the class ran (`static MutableDependencyResolverAOTExtensions()`).
    static STATIC_CONSTRUCTOR_RAN: Cell<bool> = const { Cell::new(false) };
}

/// The static constructor of the class: checks once that ReactiveUI was initialized
/// (`RxAppBuilder.EnsureInitialized()`).
fn static_constructor() {
    if STATIC_CONSTRUCTOR_RAN.with(Cell::get) {
        return;
    }
    RxAppBuilder::ensure_initialized();
    STATIC_CONSTRUCTOR_RAN.with(|ran| ran.set(true));
}

/// The legacy names of the registration of views without reflection
/// (`MutableDependencyResolverAOTExtensions`), with an optional contract.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's class is internal; it is public and hidden
/// from the documentation here, so that its tests, in their own crate, reach it.
#[doc(hidden)]
pub trait MutableDependencyResolverAotExtensions: IMutableDependencyResolver {
    /// Registers `V` as the view of `VM` under `contract`, if any, a new view at each resolution
    /// (`RegisterViewForViewModelAOT<TView, TViewModel>(string?)`).
    fn register_view_for_view_model_aot<V: IViewForOf<VM> + Default + 'static, VM: 'static>(&self, contract: Option<&str>) -> &Self {
        static_constructor();
        self.register_with_contract::<dyn IViewForOf<VM>>(|| Some(Rc::new(V::default()) as Rc<dyn IViewForOf<VM>>), contract);
        self
    }

    /// Registers `V` as the view of `VM` under `contract`, if any, one view created at the first
    /// resolution (`RegisterSingletonViewForViewModelAOT<TView, TViewModel>(string?)`).
    fn register_singleton_view_for_view_model_aot<V: IViewForOf<VM> + Default + 'static, VM: 'static>(
        &self,
        contract: Option<&str>,
    ) -> &Self {
        static_constructor();
        self.register_lazy_singleton_with_contract::<dyn IViewForOf<VM>>(
            || Some(Rc::new(V::default()) as Rc<dyn IViewForOf<VM>>),
            contract,
        );
        self
    }
}

impl<R: IMutableDependencyResolver + ?Sized> MutableDependencyResolverAotExtensions for R {}

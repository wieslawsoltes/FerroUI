//! `MutableDependencyResolverExtensions` (`ReactiveUI.Shared/Mixins/MutableDependencyResolverExtensions.cs`).

use crate::{IViewForOf, RxAppBuilder};
use ferroui_splat::{IMutableDependencyResolver, MutableDependencyResolverExt};
use std::cell::Cell;
use std::rc::Rc;

ferroui_system::ferro_thread_local! {
    /// Whether the static constructor of the class ran (`static MutableDependencyResolverExtensions()`).
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

/// The registration of the views of view models without reflection
/// (`MutableDependencyResolverExtensions`): a view type with a default constructor is registered
/// as the `dyn IViewForOf<VM>` of its view model.
///
/// As upstream's static constructor, the first call on a thread checks that ReactiveUI was
/// initialized ([`RxAppBuilder::ensure_initialized`]), and panics if it was not.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's static constructor runs once per process;
/// the check runs once per thread, as the state of ReactiveUI is per thread.
pub trait MutableDependencyResolverExtensions: IMutableDependencyResolver {
    /// Registers `V` as the view of `VM`, a new view at each resolution
    /// (`RegisterViewForViewModel<TView, TViewModel>()`); returns the resolver, to chain the
    /// calls.
    fn register_view_for_view_model<V: IViewForOf<VM> + Default + 'static, VM: 'static>(&self) -> &Self {
        self.register_view_for_view_model_with_contract::<V, VM>(None)
    }

    /// Registers `V` as the view of `VM` under `contract`, a new view at each resolution
    /// (`RegisterViewForViewModel<TView, TViewModel>(string?)`).
    fn register_view_for_view_model_with_contract<V: IViewForOf<VM> + Default + 'static, VM: 'static>(&self, contract: Option<&str>) -> &Self {
        static_constructor();
        self.register_with_contract::<dyn IViewForOf<VM>>(|| Some(Rc::new(V::default()) as Rc<dyn IViewForOf<VM>>), contract);
        self
    }

    /// Registers `V` as the view of `VM`, one view created at the first resolution
    /// (`RegisterSingletonViewForViewModel<TView, TViewModel>()`).
    fn register_singleton_view_for_view_model<V: IViewForOf<VM> + Default + 'static, VM: 'static>(&self) -> &Self {
        self.register_singleton_view_for_view_model_with_contract::<V, VM>(None)
    }

    /// Registers `V` as the view of `VM` under `contract`, one view created at the first
    /// resolution (`RegisterSingletonViewForViewModel<TView, TViewModel>(string?)`).
    fn register_singleton_view_for_view_model_with_contract<V: IViewForOf<VM> + Default + 'static, VM: 'static>(
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

impl<R: IMutableDependencyResolver + ?Sized> MutableDependencyResolverExtensions for R {}

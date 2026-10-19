//! `ViewRegistrar` (`ViewRegistrar.cs`): registers the views of ReactiveUI with a dependency
//! resolver.
//!
//! Deviation (DEVIATIONS.md, ReactiveUI controls): upstream scans assemblies by reflection for the
//! types that implement `IViewFor<TViewModel>` and reads their `ViewContract` attribute. Rust has no
//! reflection; an "assembly" is a [`ViewAssembly`], the list of the types an application declares
//! (`ViewAssembly::new("App").view::<LoginViewModel, LoginView>(..)`), with each view's view model
//! type, contract and constructor (upstream's `Activator`). Registration, its contract, the
//! preference of the service locator over the constructor and the skipping of duplicate
//! assemblies are as upstream.

use ferroui_reactiveui::{IViewForOf, RegisterViewForExt};
use ferroui_splat::{AppLocator, ILogManager, IMutableDependencyResolver, IStaticFullLogger, LogHost};
use std::any::{type_name, TypeId};
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// Registers one view type with a resolver, under its contract.
type RegisterView = Rc<dyn Fn(&dyn IMutableDependencyResolver)>;

/// A type of a [`ViewAssembly`]: its name and, for a view, how it is registered.
#[derive(Clone)]
pub struct ViewAssemblyType {
    /// The name of the type.
    pub name: &'static str,
    /// The type.
    pub type_id: TypeId,
    /// Whether the type is abstract or an interface, which is never registered.
    pub is_abstract: bool,
    /// The registration of a view (`IViewFor<TViewModel>`); `None` for any other type.
    register: Option<RegisterView>,
    /// The contract of the view (`ViewContractAttribute.Contract`).
    contract: Option<String>,
}

impl fmt::Debug for ViewAssemblyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewAssemblyType")
            .field("name", &self.name)
            .field("is_abstract", &self.is_abstract)
            .field("is_view", &self.register.is_some())
            .field("contract", &self.contract)
            .finish()
    }
}

/// The types of an application that hold its views (an assembly scanned upstream).
#[derive(Debug, Default)]
pub struct ViewAssembly {
    /// The name of the assembly.
    pub name: &'static str,
    types: Vec<ViewAssemblyType>,
}

impl ViewAssembly {
    /// An assembly without types.
    pub fn new(name: &'static str) -> Self {
        Self { name, types: Vec::new() }
    }

    /// Adds the view type `V` of the view model type `VM` (a class that implements
    /// `IViewFor<TViewModel>`), registered under `contract` (`[ViewContract("...")]`; `None` for
    /// the default), and created by `activator` when the service locator has no `V`.
    pub fn view<VM: 'static, V: IViewForOf<VM> + 'static>(mut self, contract: Option<&str>, activator: fn() -> Rc<V>) -> Self {
        let contract = contract.map(str::to_owned);
        let registered = contract.clone();
        let register: RegisterView = Rc::new(move |resolver: &dyn IMutableDependencyResolver| {
            resolver.register_view_for::<VM>(move || ViewRegistrar::create_view::<V>(activator) as Rc<dyn IViewForOf<VM>>, registered.as_deref());
        });
        self.types.push(ViewAssemblyType {
            name: type_name::<V>(),
            type_id: TypeId::of::<V>(),
            is_abstract: false,
            register: Some(register),
            contract,
        });
        self
    }

    /// Adds a type that is not a view (it does not implement `IViewFor<TViewModel>`), or an
    /// abstract one: the registrar skips it.
    pub fn other_type<T: ?Sized + 'static>(mut self, is_abstract: bool) -> Self {
        self.types.push(ViewAssemblyType { name: type_name::<T>(), type_id: TypeId::of::<T>(), is_abstract, register: None, contract: None });
        self
    }

    /// The types of the assembly (`GetTypes()`).
    pub fn types(&self) -> &[ViewAssemblyType] {
        &self.types
    }
}

/// Registers the views of assemblies with a dependency resolver (`ViewRegistrar`).
///
/// A view is registered as `IViewFor<TViewModel>` of the view model it implements, under the
/// contract it declares. Resolution prefers the service locator and falls back to the
/// constructor, so a view with no registration of its own is still constructible.
pub struct ViewRegistrar;

impl ViewRegistrar {
    /// Registers the views of `assemblies` with `resolver` when both are available
    /// (`RegisterViews`).
    pub fn register_views(resolver: Option<&dyn IMutableDependencyResolver>, assemblies: Option<&[Rc<ViewAssembly>]>) {
        let (Some(resolver), Some(assemblies)) = (resolver, assemblies) else {
            return;
        };
        if assemblies.is_empty() {
            return;
        }

        Self::register_views_internal(resolver, assemblies);
    }

    /// Registers every view of `assemblies` that is neither abstract nor an interface
    /// (`RegisterViewsInternal`). An assembly given twice is registered once.
    pub fn register_views_internal(resolver: &dyn IMutableDependencyResolver, assemblies: &[Rc<ViewAssembly>]) {
        let mut unique_assemblies: Vec<&Rc<ViewAssembly>> = Vec::new();
        for assembly in assemblies {
            if unique_assemblies.iter().any(|seen| Rc::ptr_eq(seen, assembly)) {
                continue;
            }
            unique_assemblies.push(assembly);

            for view_type in assembly.types() {
                if view_type.is_abstract {
                    continue;
                }

                let Some(register) = Self::find_view_for_interface(view_type) else {
                    continue;
                };
                register(resolver);
            }
        }
    }

    /// The registration of `IViewFor<TViewModel>` of a type, `None` when the type is not a view
    /// (`FindViewForInterface`).
    pub fn find_view_for_interface(view_type: &ViewAssemblyType) -> Option<&RegisterView> {
        view_type.register.as_ref()
    }

    /// The contract a view declares, `None` for the default (`GetViewContract`).
    pub fn get_view_contract(view_type: &ViewAssemblyType) -> Option<&str> {
        view_type.contract.as_deref()
    }

    /// Resolves a view of type `V` from the service locator, or creates it with `activator`
    /// (`CreateView`). A failure of the locator is logged and falls back to the constructor.
    pub fn create_view<V: 'static>(activator: fn() -> Rc<V>) -> Rc<V> {
        match catch_unwind(AssertUnwindSafe(|| AppLocator::get_service::<V>())) {
            Ok(Some(resolved)) => resolved,
            Ok(None) => Self::create_view_with_activator(activator),
            Err(payload) => {
                let message = payload
                    .downcast_ref::<&str>()
                    .map(|text| (*text).to_owned())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "the resolver panicked".to_owned());
                Self::create_view_after_resolution_failure(activator, &message)
            }
        }
    }

    /// Logs a failure of the service locator and creates the view with its constructor
    /// (`CreateViewAfterResolutionFailure`).
    pub fn create_view_after_resolution_failure<V: 'static>(activator: fn() -> Rc<V>, error: &str) -> Rc<V> {
        if AppLocator::get_service::<dyn ILogManager>().is_some() {
            LogHost::default_logger().warn(
                &format!("Failed to resolve view type '{}' from the service locator. Falling back to Activator. {error}", type_name::<V>()),
                Some("CreateViewAfterResolutionFailure"),
            );
        }
        Self::create_view_with_activator(activator)
    }

    /// Creates the view with its constructor (`CreateViewWithActivator`).
    pub fn create_view_with_activator<V: 'static>(activator: fn() -> Rc<V>) -> Rc<V> {
        activator()
    }
}

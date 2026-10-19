//! `DefaultViewLocator` (ReactiveUI.Binding, `View/DefaultViewLocator.cs`).

use super::view_mapping_key::ViewMappingKey;
use super::ViewMappingBuilder;
use crate::binding::interfaces::{view_for_of, IViewFor, IViewForOf, IViewLocator, ViewModelType};
use ferroui_splat::{AppLocator, IEnableLogger, ILogManager, ReadonlyDependencyResolverExt};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

/// A lookup of the views of view models, registered with
/// [`DefaultViewLocator::set_generated_view_dispatch`]: it takes the view model and the contract
/// (empty for the default) and gives the view, or `None` for a view model it has no view for.
///
/// Upstream's source generator writes one per assembly that contains views
/// (`Func<object, string, IViewFor?>`); here [`ViewRegistrations`](crate::ViewRegistrations) builds
/// one from registered views, or an application writes its own.
pub type GeneratedViewDispatch = Rc<dyn Fn(&dyn Any, &str) -> Option<Rc<dyn IViewFor>>>;

/// A factory of the view of a mapping.
type ViewFactory = Rc<dyn Fn() -> Rc<dyn IViewFor>>;

// Deviation (DEVIATIONS.md, ReactiveUI): the generated lookups are a static of the process
// upstream; here they are per thread, as the service locator is.
ferroui_system::ferro_thread_local! {
    /// The generated lookups, in the order of their registration (`_generatedDispatches`). The
    /// list is replaced, not changed, so that a resolution walks the list it started with.
    static GENERATED_DISPATCHES: RefCell<Rc<Vec<GeneratedViewDispatch>>> = RefCell::new(Rc::new(Vec::new()));
    /// The view model types known at run time by their `TypeId`: what closing `IViewFor<>` over the
    /// run-time type of a view model needs (`MakeGenericType`).
    static VIEW_MODEL_TYPES: RefCell<HashMap<TypeId, ViewModelType>> = RefCell::new(HashMap::new());
}

/// The warning logged when a view model held as `dyn Any` has no generated or mapped view
/// (`UnresolvedObjectViewMessage`).
fn unresolved_object_view_message(type_name: &str) -> String {
    format!(
        "Found no generated or mapped view for {type_name}. A view registered in the service locator as IViewFor<{type_name}> is only \
         found by ResolveViewUnsafe (resolve_view_unsafe); add it with map, or register it with ViewRegistrations."
    )
}

/// The view locator of ReactiveUI.Binding (`DefaultViewLocator`). It resolves the view of a view
/// model in three tiers:
///
/// 1. the generated lookups ([`set_generated_view_dispatch`](Self::set_generated_view_dispatch)),
///    the latest registered first;
/// 2. the mappings of the locator ([`map`](Self::map), [`map_factory`](Self::map_factory),
///    [`ViewMappingBuilder`]), by the type of the view model and the contract;
/// 3. the service locator, for a view registered as `dyn IViewForOf<VM>`.
///
/// A view model held as `Rc<dyn Any>` goes through the first two tiers
/// ([`IViewLocator::resolve_view`]); [`IViewLocator::resolve_view_unsafe`] adds the third for the
/// view model types the locator knows by their `TypeId`.
///
/// Not ported: the lookup of a view by the name of the type of its view model, which needs
/// reflection.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): closing `IViewFor<>` over the run-time type of a view
/// model (`MakeGenericType`) needs reflection. [`IViewLocator::resolve_view_unsafe`] asks the
/// service locator for the view model types recorded at run time instead: those of the mappings,
/// of [`register_view_for`](crate::RegisterViewForExt::register_view_for), of a typed resolution,
/// and of [`register_view_model_type`](Self::register_view_model_type). The mappings are kept in a
/// `RefCell`, as everything is single-threaded (docs/porting/reactive.md, section 5.1).
pub struct DefaultViewLocator {
    /// `_mappings`.
    mappings: RefCell<HashMap<ViewMappingKey, ViewFactory>>,
}

impl Default for DefaultViewLocator {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DefaultViewLocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultViewLocator").field("mappings", &self.mappings.borrow().len()).finish()
    }
}

impl IEnableLogger for DefaultViewLocator {}

impl DefaultViewLocator {
    /// A locator with no mapping.
    pub fn new() -> Self {
        Self { mappings: RefCell::new(HashMap::new()) }
    }

    /// Adds a generated lookup (`SetGeneratedViewDispatch`). Every lookup is asked, the most
    /// recently registered first, and the first view one gives wins; adding a lookup already
    /// registered (the same `Rc`) does nothing.
    pub fn set_generated_view_dispatch(dispatch: GeneratedViewDispatch) {
        GENERATED_DISPATCHES.with(|dispatches| {
            let current = dispatches.borrow().clone();
            if current.iter().any(|registered| Rc::ptr_eq(registered, &dispatch)) {
                return;
            }
            let mut next = Vec::with_capacity(current.len() + 1);
            next.extend(current.iter().cloned());
            next.push(dispatch);
            *dispatches.borrow_mut() = Rc::new(next);
        });
    }

    /// Removes every generated lookup (`ResetGeneratedViewDispatchForTesting`). For tests.
    #[doc(hidden)]
    pub fn reset_generated_view_dispatch_for_testing() {
        GENERATED_DISPATCHES.with(|dispatches| *dispatches.borrow_mut() = Rc::new(Vec::new()));
    }

    /// Records the view model type `VM`, so that
    /// [`resolve_view_unsafe`](IViewLocator::resolve_view_unsafe) asks the service locator for
    /// `dyn IViewForOf<VM>` when a view model of that type is held as `dyn Any`.
    ///
    /// Not an upstream member: it stands for the run-time construction of `IViewFor<TViewModel>`.
    pub fn register_view_model_type<VM: 'static>() {
        record_view_model_type(ViewModelType::of::<VM>());
    }

    /// Maps the view model type `VM`, under `contract` (`None` for the default), to a new view of
    /// type `V`, replacing an existing mapping (`Map<TViewModel, TView>(string?)`).
    pub fn map<VM: 'static, V: IViewFor + Default>(&self, contract: Option<&str>) {
        self.map_factory::<VM>(|| Rc::new(V::default()) as Rc<dyn IViewFor>, contract);
    }

    /// Maps the view model type `VM`, under `contract`, to the view `factory` creates, replacing an
    /// existing mapping (`Map<TViewModel>(Func<IViewFor>, string?)`).
    pub fn map_factory<VM: 'static>(&self, factory: impl Fn() -> Rc<dyn IViewFor> + 'static, contract: Option<&str>) {
        record_view_model_type(ViewModelType::of::<VM>());
        let key = ViewMappingKey::new(TypeId::of::<VM>(), contract);
        self.mappings.borrow_mut().insert(key, Rc::new(factory));
    }

    /// Removes the mapping of the view model type `VM` under `contract`; whether there was one
    /// (`Unmap<TViewModel>(string?)`).
    pub fn unmap<VM: 'static>(&self, contract: Option<&str>) -> bool {
        let key = ViewMappingKey::new(TypeId::of::<VM>(), contract);
        self.mappings.borrow_mut().remove(&key).is_some()
    }

    /// Resolves the view of `view_model` through its compile-time type `VM`, in the three tiers,
    /// and sets the view's view model (`ResolveView<TViewModel>(TViewModel, string?)`).
    pub fn resolve_view_of<VM: 'static>(&self, view_model: &Rc<VM>, contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
        self.resolve_view_typed(view_model.clone() as Rc<dyn Any>, &ViewModelType::of::<VM>(), contract)
    }

    /// Resolves a view of the type `VM` without a view model, from the mappings, then the service
    /// locator; a mapped view that is not an `IViewForOf<VM>` is skipped, and the view's view model
    /// is left unset (`ResolveView<TViewModel>(string?)`).
    pub fn resolve_view_for<VM: 'static>(&self, contract: Option<&str>) -> Option<Rc<dyn IViewForOf<VM>>> {
        let contract = contract.unwrap_or_default();
        let type_name = std::any::type_name::<VM>();

        if let Some(mapped) = self.try_resolve_from_mappings(TypeId::of::<VM>(), contract).and_then(view_for_of::<VM>) {
            self.log_debug(&format!("Resolved IViewFor<{type_name}> from an explicit mapping"));
            return Some(mapped);
        }

        let service_contract = if contract.is_empty() { None } else { Some(contract) };
        if let Some(view) = AppLocator::current().get_service_with_contract::<dyn IViewForOf<VM>>(service_contract) {
            self.log_debug(&format!("Resolved IViewFor<{type_name}> from the service locator"));
            return Some(view);
        }

        self.log_warn(&format!(
            "Failed to resolve a view for {type_name}. Map it on the locator or register IViewFor<{type_name}> in the service locator."
        ));
        None
    }

    /// A [`ViewMappingBuilder`] that registers its mappings on this locator
    /// (`CreateMappingBuilder`).
    pub fn create_mapping_builder(&self) -> ViewMappingBuilder<'_> {
        ViewMappingBuilder::new(self)
    }

    /// Asks each generated lookup for a view, the most recently registered first
    /// (`TryResolveFromGeneratedDispatches`).
    fn try_resolve_from_generated_dispatches(view_model: &dyn Any, contract: &str) -> Option<Rc<dyn IViewFor>> {
        let dispatches = GENERATED_DISPATCHES.with(|dispatches| dispatches.borrow().clone());
        dispatches.iter().rev().find_map(|dispatch| dispatch(view_model, contract))
    }

    /// The view of the mapping of `view_model_type` under `contract`, if any
    /// (`TryResolveFromMappings`). The factory runs outside of the borrow of the mappings.
    fn try_resolve_from_mappings(&self, view_model_type: TypeId, contract: &str) -> Option<Rc<dyn IViewFor>> {
        let key = ViewMappingKey { view_model_type, contract: contract.to_owned() };
        let factory = self.mappings.borrow().get(&key).cloned()?;
        Some(factory())
    }

    /// The generated lookups, then the mappings, for the run-time type of `view_model`; the view's
    /// view model is set (`ResolveGeneratedOrMapped`).
    fn resolve_generated_or_mapped(&self, view_model: &Rc<dyn Any>, contract: &str) -> Option<Rc<dyn IViewFor>> {
        let view = Self::try_resolve_from_generated_dispatches(&**view_model, contract)
            .or_else(|| self.try_resolve_from_mappings((**view_model).type_id(), contract))?;
        set_view_model_on_view(&view, view_model);
        Some(view)
    }

    /// Logs `message` as a warning, when the service locator has a log manager.
    // Deviation (DEVIATIONS.md, ReactiveUI): upstream's `this.Log()` throws without a log manager;
    // the locator works without one, as before the application is built.
    fn log_warn(&self, message: &str) {
        if AppLocator::get_service::<dyn ILogManager>().is_some() {
            self.log().warn(message);
        }
    }

    /// Logs `message` at the level debug, when the service locator has a log manager.
    fn log_debug(&self, message: &str) {
        if AppLocator::get_service::<dyn ILogManager>().is_some() {
            self.log().debug(message);
        }
    }
}

impl IViewLocator for DefaultViewLocator {
    /// The three tiers for the compile-time type of the view model; the view's view model is set
    /// (`ResolveView<TViewModel>(TViewModel, string?)`).
    fn resolve_view_typed(&self, view_model: Rc<dyn Any>, view_model_type: &ViewModelType, contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
        record_view_model_type(*view_model_type);
        let contract = contract.unwrap_or_default();

        // 1. The generated lookups.
        if let Some(view) = Self::try_resolve_from_generated_dispatches(&*view_model, contract) {
            set_view_model_on_view(&view, &view_model);
            return Some(view);
        }

        // 2. The mappings.
        if let Some(view) = self.try_resolve_from_mappings(view_model_type.id(), contract) {
            set_view_model_on_view(&view, &view_model);
            return Some(view);
        }

        // 3. The service locator.
        let view = view_model_type.service_view(Some(contract))?;
        set_view_model_on_view(&view, &view_model);
        Some(view)
    }

    /// The generated lookups, then the mappings, for the run-time type of the view model; never
    /// the service locator. A warning is logged when neither has a view (`ResolveView(object?,
    /// string?)`).
    fn resolve_view(&self, view_model: Option<Rc<dyn Any>>, contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
        let view_model = view_model?;
        let view = self.resolve_generated_or_mapped(&view_model, contract.unwrap_or_default());
        if view.is_none() {
            let type_name = known_type_name((*view_model).type_id());
            self.log_warn(&unresolved_object_view_message(&type_name));
        }
        view
    }

    /// [`resolve_view`](IViewLocator::resolve_view), then the service locator for the view model
    /// types known at run time (`ResolveViewUnsafe(object?, string?)`).
    fn resolve_view_unsafe(&self, view_model: Option<Rc<dyn Any>>, contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
        let view_model = view_model?;
        let contract = contract.unwrap_or_default();
        if let Some(view) = self.resolve_generated_or_mapped(&view_model, contract) {
            return Some(view);
        }

        // `TryResolveViaReflection`: the service locator, for the run-time type.
        let view_model_type = VIEW_MODEL_TYPES.with(|types| types.borrow().get(&(*view_model).type_id()).copied())?;
        let view = view_model_type.service_view(Some(contract))?;
        set_view_model_on_view(&view, &view_model);
        Some(view)
    }
}

/// Sets the view model of the resolved view (`SetViewModelOnView`).
fn set_view_model_on_view(view: &Rc<dyn IViewFor>, view_model: &Rc<dyn Any>) {
    view.set_view_model(Some(view_model.clone()));
}

/// Records a view model type known at run time.
pub(crate) fn record_view_model_type(view_model_type: ViewModelType) {
    VIEW_MODEL_TYPES.with(|types| {
        types.borrow_mut().entry(view_model_type.id()).or_insert(view_model_type);
    });
}

/// The name of a view model type known at run time, or a description of an unknown one (a
/// `dyn Any` has no type name).
fn known_type_name(type_id: TypeId) -> String {
    VIEW_MODEL_TYPES.with(|types| types.borrow().get(&type_id).map(|known| known.type_name().to_owned())).unwrap_or_else(|| format!("{type_id:?}"))
}

//! `ViewRegistrations` and `ViewRegistrationOptions`: the views that upstream's source generator
//! finds (the classes implementing `IViewFor<T>`, with their attributes `ViewContract`,
//! `SingleInstanceView` and `ExcludeFromViewRegistration`), registered by hand.
//!
//! Not an upstream file: upstream generates the view dispatch of an assembly
//! (`__ReactiveUIGeneratedBindings`) and registers it with
//! `DefaultViewLocator.SetGeneratedViewDispatch`.

use super::{DefaultViewLocator, ExcludeFromViewRegistration, GeneratedViewDispatch, SingleInstanceView, ViewContract};
use crate::binding::interfaces::{IViewFor, IViewForOf, ViewModelType};
use std::any::{Any, TypeId};
use std::cell::OnceCell;
use std::fmt;
use std::rc::Rc;

/// The options of a view registration: upstream's attributes of a view class, as data.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ViewRegistrationOptions {
    /// The contract the view is registered under (`[ViewContract("...")]`); `None` for the default.
    pub contract: Option<ViewContract>,
    /// Whether the view is created once and reused (`[SingleInstanceView]`).
    pub single_instance: Option<SingleInstanceView>,
    /// Whether the view is left out of the dispatch (`[ExcludeFromViewRegistration]`).
    pub exclude: Option<ExcludeFromViewRegistration>,
}

impl ViewRegistrationOptions {
    /// No option: the default contract, a new view for each resolution.
    pub fn new() -> Self {
        Self::default()
    }

    /// The options with `contract`.
    pub fn with_contract(mut self, contract: ViewContract) -> Self {
        self.contract = Some(contract);
        self
    }

    /// The options with [`SingleInstanceView`].
    pub fn with_single_instance(mut self) -> Self {
        self.single_instance = Some(SingleInstanceView);
        self
    }

    /// The options with [`ExcludeFromViewRegistration`].
    pub fn with_exclude(mut self) -> Self {
        self.exclude = Some(ExcludeFromViewRegistration);
        self
    }
}

/// One registered view.
struct Registration {
    view_model_type: TypeId,
    contract: String,
    factory: Rc<dyn Fn() -> Rc<dyn IViewFor>>,
}

/// The views of view models, collected into one [`GeneratedViewDispatch`]: what upstream's source
/// generator builds from the classes that implement `IViewFor<T>`.
///
/// ```
/// use ferroui_reactiveui::{view_for, DefaultViewLocator, IViewLocator, ViewContract, ViewRegistrationOptions, ViewRegistrations};
/// use std::any::Any;
/// use std::cell::RefCell;
/// use std::rc::Rc;
///
/// struct TodoViewModel;
///
/// #[derive(Default)]
/// struct TodoView {
///     view_model: RefCell<Option<Rc<TodoViewModel>>>,
/// }
/// view_for!(TodoView, TodoViewModel, view_model);
///
/// ViewRegistrations::new()
///     .register::<TodoViewModel, TodoView>(ViewRegistrationOptions::new())
///     .register::<TodoViewModel, TodoView>(ViewRegistrationOptions::new().with_contract(ViewContract::new("compact")))
///     .apply();
///
/// let view = DefaultViewLocator::new().resolve_view(Some(Rc::new(TodoViewModel) as Rc<dyn Any>), Some("compact"));
/// assert!(view.is_some());
/// ```
#[derive(Default)]
pub struct ViewRegistrations {
    registrations: Vec<Registration>,
}

impl fmt::Debug for ViewRegistrations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewRegistrations").field("registrations", &self.registrations.len()).finish()
    }
}

impl ViewRegistrations {
    /// No registration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the view type `V` for the view model type `VM`, created with `Default`.
    pub fn register<VM: 'static, V: IViewForOf<VM> + Default>(self, options: ViewRegistrationOptions) -> Self {
        self.register_with::<VM, V>(|| Rc::new(V::default()), options)
    }

    /// Registers the views `factory` creates for the view model type `VM`. A registration with
    /// [`ExcludeFromViewRegistration`] is skipped; one with [`SingleInstanceView`] creates its view
    /// once, on its first resolution.
    pub fn register_with<VM: 'static, V: IViewForOf<VM>>(mut self, factory: impl Fn() -> Rc<V> + 'static, options: ViewRegistrationOptions) -> Self {
        if options.exclude.is_some() {
            return self;
        }
        DefaultViewLocator::register_view_model_type::<VM>();
        let factory: Rc<dyn Fn() -> Rc<dyn IViewFor>> = if options.single_instance.is_some() {
            let instance: OnceCell<Rc<dyn IViewFor>> = OnceCell::new();
            Rc::new(move || instance.get_or_init(|| factory() as Rc<dyn IViewFor>).clone())
        } else {
            Rc::new(move || factory() as Rc<dyn IViewFor>)
        };
        self.registrations.push(Registration {
            view_model_type: ViewModelType::of::<VM>().id(),
            contract: options.contract.map(|contract| contract.contract().to_owned()).unwrap_or_default(),
            factory,
        });
        self
    }

    /// The lookup of the registered views: the first registration of the type of the view model
    /// under the contract gives the view.
    pub fn dispatch(self) -> GeneratedViewDispatch {
        let registrations = self.registrations;
        Rc::new(move |view_model: &dyn Any, contract: &str| {
            let view_model_type = Any::type_id(view_model);
            registrations
                .iter()
                .find(|registration| registration.view_model_type == view_model_type && registration.contract == contract)
                .map(|registration| (registration.factory)())
        })
    }

    /// Registers the lookup with [`DefaultViewLocator::set_generated_view_dispatch`], as upstream's
    /// generated module initializer does.
    pub fn apply(self) {
        DefaultViewLocator::set_generated_view_dispatch(self.dispatch());
    }
}

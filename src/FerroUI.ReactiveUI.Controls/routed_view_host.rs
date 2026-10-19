//! `RoutedViewHost` (`RoutedViewHost.cs`): hosts the view associated with the current ReactiveUI
//! routing state.

use crate::navigation_target::NavigationTarget;
use crate::view_host_navigation::{log_info, log_warn, ViewHostNavigation};
use crate::ViewModelViewHost;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObject,
    FerroObjectExtensions, FerroObjectImpl, FerroProperty, Ref, StaticType, StyledElementImpl, StyledProperty, TypeInfo,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, TransitioningContentControl};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_reactiveui::{IViewLocator, RoutableViewModel, RoutingState, ViewModelValue};
use ferroui_splat::IEnableLogger;
use ferroui_system::{IDisposable, IObservable};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// Hosts the view associated with the current ReactiveUI routing state (`RoutedViewHost`).
///
/// ReactiveUI routing consists of an `IScreen` containing a [`RoutingState`], several
/// `IRoutableViewModel`s, and a platform-specific `RoutedViewHost` control. The routing state
/// manages the view-model navigation stack; the host monitors that state and embeds the view for
/// the current view model. Bind `IScreen.Router` to [`router`](Self::router) in a view whose view
/// model implements `IScreen`.
///
/// See the ReactiveUI routing documentation, <https://reactiveui.net/docs/handbook/routing/>.
#[repr(C)]
pub struct RoutedViewHost {
    base: TransitioningContentControl,
    /// The view locator used to resolve views (`ViewLocator`); `None` uses the current one.
    view_locator: RefCell<Option<Rc<dyn IViewLocator>>>,
    /// The active navigation subscriptions while the host is attached to the visual tree
    /// (`_navigationDisposables`).
    navigation_disposables: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(RoutedViewHost: TransitioningContentControl);
ferro_class_info!(RoutedViewHost { new: RoutedViewHost::new });
ferro_impl_classes!(
    RoutedViewHost: FerroObjectImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl IEnableLogger for RoutedViewHost {}

impl StyledElementImpl for RoutedViewHost {
    /// `StyleKeyOverride`: the host is styled as a `TransitioningContentControl`.
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <TransitioningContentControl as StaticType>::TYPE
    }
}

impl VisualImpl for RoutedViewHost {
    /// `OnAttachedToVisualTree`: creates the navigation subscriptions once (`??=`); the base
    /// handler runs when they are created.
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        if this.navigation_disposables.borrow().is_some() {
            return;
        }
        let disposables = this.create_navigation_disposables(e);
        *this.navigation_disposables.borrow_mut() = Some(disposables);
    }

    /// `OnDetachedFromVisualTree`.
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.dispose_navigation_disposables();
    }
}

ferro_properties! {
    impl RoutedViewHost {
        /// Identifies the `Router` property (`RouterProperty`).
        pub fn router_property() -> StyledProperty<Option<Rc<RoutingState>>> {
            FerroProperty::register::<RoutedViewHost, _>("Router", None)
        }

        /// Identifies the `ViewContract` property (`ViewContractProperty`).
        pub fn view_contract_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<RoutedViewHost, _>("ViewContract", None)
        }

        /// Identifies the `DefaultContent` property (`DefaultContentProperty`), the one of
        /// [`ViewModelViewHost`] with this owner added.
        pub fn default_content_property() -> StyledProperty<Option<BoxedValue>> {
            ViewModelViewHost::default_content_property().add_owner::<RoutedViewHost>()
        }
    }
}

impl RoutedViewHost {
    /// Field initialisation.
    pub fn construct() -> Self {
        Self {
            base: TransitioningContentControl::construct(),
            view_locator: RefCell::new(None),
            navigation_disposables: RefCell::new(None),
        }
    }

    /// Creates the host.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The current routing state (`Router`).
    pub fn router(&self) -> Option<Rc<RoutingState>> {
        self.get_value(Self::router_property())
    }

    /// Sets the routing state to follow.
    pub fn set_router(&self, value: Option<Rc<RoutingState>>) {
        self.set_value(Self::router_property(), value)
    }

    /// The contract of the views to resolve (`ViewContract`).
    pub fn view_contract(&self) -> Option<String> {
        self.get_value(Self::view_contract_property())
    }

    /// Sets the contract of the views to resolve.
    pub fn set_view_contract(&self, value: Option<String>) {
        self.set_value(Self::view_contract_property(), value)
    }

    /// The content shown when no view is available for the current view model
    /// (`DefaultContent`).
    pub fn default_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::default_content_property())
    }

    /// Sets the content shown when no view is available for the current view model.
    pub fn set_default_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::default_content_property(), value)
    }

    /// The view locator used to resolve views (`ViewLocator`); `None` uses the current one.
    pub fn view_locator(&self) -> Option<Rc<dyn IViewLocator>> {
        self.view_locator.borrow().clone()
    }

    /// Sets the view locator used to resolve views.
    pub fn set_view_locator(&self, value: Option<Rc<dyn IViewLocator>>) {
        *self.view_locator.borrow_mut() = value;
    }

    /// Navigates to the view associated with `view_model` and `contract` (`NavigateToViewModel`):
    /// a missing router, view model or view displays the default content; a resolved view
    /// receives the view model through its `ViewModel` and `DataContext`.
    ///
    /// Deviation (DEVIATIONS.md, ReactiveUI controls): internal upstream, public here so that the
    /// tests, in another module, reach it.
    pub fn navigate_to_view_model(&self, view_model: Option<Rc<dyn Any>>, contract: Option<&str>) {
        if self.router().is_none() {
            log_warn(self, "Router property is null. Falling back to default content.");
            self.set_content(self.default_content());
            return;
        }

        let Some(view_model) = view_model else {
            log_info(self, "ViewModel is null. Falling back to default content.");
            self.set_content(self.default_content());
            return;
        };

        let content = ViewHostNavigation::resolve_view(self, view_model, contract, self.view_locator())
            .map(|view| ViewHostNavigation::content_of_view(&view))
            .or_else(|| self.default_content());
        self.set_content(content);
    }

    /// Disposes the active navigation subscriptions when they exist
    /// (`DisposeNavigationDisposables`).
    pub fn dispose_navigation_disposables(&self) {
        ViewHostNavigation::release(&self.navigation_disposables);
    }

    /// The current routed view model of the navigation stack, `None` when the stack is empty
    /// (`GetCurrentViewModel`).
    fn get_current_view_model(router: &RoutingState) -> ViewModelValue {
        let stack = router.navigation_stack();
        let count = stack.count();
        if count == 0 {
            return None;
        }
        let view_model: RoutableViewModel = stack.get(count - 1);
        Some(view_model as Rc<dyn Any>)
    }

    /// A stream that begins with the router's current view model and then follows future
    /// navigation (`CreateRouterViewModelObservable`).
    fn create_router_view_model_observable(router: &RoutingState) -> Rc<dyn IObservable<ViewModelValue>> {
        Observable::return_(Self::get_current_view_model(router)).merge(
            router
                .current_view_model()
                .select(|view_model: Option<RoutableViewModel>| view_model.map(|view_model| view_model as Rc<dyn Any>)),
        )
    }

    /// Creates the active navigation subscriptions for an attached host
    /// (`CreateNavigationDisposables`): the base handler runs first.
    fn create_navigation_disposables(&self, e: &VisualTreeAttachmentEventArgs) -> Rc<dyn IDisposable> {
        Self::parent_on_attached_to_visual_tree(self, e);

        let object: &FerroObject = self;
        let router_changes = FerroObjectExtensions::get_observable(object, Self::router_property());
        let view_contract = FerroObjectExtensions::get_observable(object, Self::view_contract_property());
        let view_model_sources = router_changes.select(|router: Option<Rc<RoutingState>>| match router {
            None => Observable::return_(None::<Rc<dyn Any>>),
            Some(router) => Self::create_router_view_model_observable(&router),
        });
        let view_models = view_model_sources.switch::<ViewModelValue>();
        let navigation = view_models
            .combine_latest(view_contract, |view_model: ViewModelValue, contract: Option<String>| NavigationTarget::new(view_model, contract));

        let weak = self.to_ref().downgrade();
        ViewHostNavigation::subscribe(navigation, move |view_model, contract| {
            if let Some(this) = weak.upgrade() {
                this.navigate_to_view_model(view_model, contract.as_deref());
            }
        })
    }
}

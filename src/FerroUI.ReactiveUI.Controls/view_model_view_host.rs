//! `ViewModelViewHost` (`ViewModelViewHost.cs`): loads and displays the view associated with the
//! `ViewModel` property.

use crate::navigation_target::NavigationTarget;
use crate::reactive_view::{IReactiveViewClass, ReactiveViewFor};
use crate::view_host_navigation::{log_info, ViewHostNavigation};
use crate::view_model_value::view_model_of_optional;
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
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{IViewFor, IViewLocator};
use ferroui_splat::IEnableLogger;
use ferroui_system::IDisposable;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// Loads and displays the view associated with the `ViewModel` property (`ViewModelViewHost`).
///
/// Assigning a view model resolves its view through the view locator ([`view_locator`] or the
/// current one) under the [`view_contract`]; the view gets the view model through its `ViewModel`
/// and `DataContext`. Without a view model, or without a view for it, the host shows its
/// [`default_content`].
///
/// [`view_locator`]: Self::view_locator
/// [`view_contract`]: Self::view_contract
/// [`default_content`]: Self::default_content
#[repr(C)]
pub struct ViewModelViewHost {
    base: TransitioningContentControl,
    /// The view locator used to resolve views (`ViewLocator`); `None` uses the current one.
    view_locator: RefCell<Option<Rc<dyn IViewLocator>>>,
    /// The active navigation subscriptions while the host is attached to the visual tree
    /// (`_navigationDisposables`).
    navigation_disposables: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(ViewModelViewHost: TransitioningContentControl);
ferro_class_info!(ViewModelViewHost { new: ViewModelViewHost::new });
ferro_impl_classes!(
    ViewModelViewHost: FerroObjectImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl IEnableLogger for ViewModelViewHost {}

impl StyledElementImpl for ViewModelViewHost {
    /// `StyleKeyOverride`: the host is styled as a `TransitioningContentControl`.
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <TransitioningContentControl as StaticType>::TYPE
    }
}

impl VisualImpl for ViewModelViewHost {
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
    impl ViewModelViewHost {
        /// Identifies the `ViewModel` property (`ViewModelProperty`).
        pub fn view_model_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ViewModelViewHost, _>("ViewModel", None)
        }

        /// Identifies the `ViewContract` property (`ViewContractProperty`).
        pub fn view_contract_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ViewModelViewHost, _>("ViewContract", None)
        }

        /// Identifies the `DefaultContent` property (`DefaultContentProperty`).
        pub fn default_content_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ViewModelViewHost, _>("DefaultContent", None)
        }
    }
}

impl ViewModelViewHost {
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

    /// The view model whose view is shown (`ViewModel`).
    pub fn view_model(&self) -> Option<BoxedValue> {
        self.get_value(Self::view_model_property())
    }

    /// Sets the view model whose view is shown.
    pub fn set_view_model(&self, value: Option<BoxedValue>) {
        self.set_value(Self::view_model_property(), value)
    }

    /// The contract of the view to resolve (`ViewContract`).
    pub fn view_contract(&self) -> Option<String> {
        self.get_value(Self::view_contract_property())
    }

    /// Sets the contract of the view to resolve.
    pub fn set_view_contract(&self, value: Option<String>) {
        self.set_value(Self::view_contract_property(), value)
    }

    /// The content shown when there is no view model or no view for it (`DefaultContent`).
    pub fn default_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::default_content_property())
    }

    /// Sets the content shown when there is no view model or no view for it.
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

    /// The host as a view of ReactiveUI (`(IViewFor)host`), which keeps it alive.
    pub fn view_for(this: &Ref<Self>) -> Rc<dyn IViewFor> {
        ReactiveViewFor::new(this.clone())
    }

    /// Navigates to the view associated with `view_model` and `contract` (`NavigateToViewModel`):
    /// a missing view model or view displays the default content; a resolved view receives the
    /// view model through its `ViewModel` and `DataContext`.
    ///
    /// Deviation (DEVIATIONS.md, ReactiveUI controls): internal upstream, public here so that the
    /// tests, in another module, reach it.
    pub fn navigate_to_view_model(&self, view_model: Option<Rc<dyn Any>>, contract: Option<&str>) {
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

    /// Creates the active navigation subscriptions for an attached host
    /// (`CreateNavigationDisposables`): the base handler runs first.
    fn create_navigation_disposables(&self, e: &VisualTreeAttachmentEventArgs) -> Rc<dyn IDisposable> {
        Self::parent_on_attached_to_visual_tree(self, e);

        let object: &FerroObject = self;
        let view_contract = FerroObjectExtensions::get_observable(object, Self::view_contract_property());
        let navigation = FerroObjectExtensions::get_observable(object, Self::view_model_property())
            .combine_latest(view_contract, |view_model: Option<BoxedValue>, contract: Option<String>| {
                NavigationTarget::new(view_model_of_optional(view_model.as_ref()), contract)
            });

        let weak = self.to_ref().downgrade();
        ViewHostNavigation::subscribe(navigation, move |view_model, contract| {
            if let Some(this) = weak.upgrade() {
                this.navigate_to_view_model(view_model, contract.as_deref());
            }
        })
    }
}

impl IReactiveViewClass for ViewModelViewHost {
    fn view_model_property() -> &'static StyledProperty<Option<BoxedValue>> {
        ViewModelViewHost::view_model_property()
    }
}

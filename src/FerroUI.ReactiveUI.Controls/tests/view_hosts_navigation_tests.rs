//! `ViewHostsNavigationTests` (`ViewHostsNavigationTests.cs`): the navigation of
//! [`ViewModelViewHost`] and [`RoutedViewHost`].
//!
//! The reference's testable hosts expose the protected members (`StyleKeyOverride`,
//! `OnAttachedToVisualTree`, `OnDetachedFromVisualTree`); here the style key is public
//! (`style_key`) and the overrides are called through their class traits, as a derived class
//! would. The views of the tests are user controls with a view model of their own
//! ([`IControlViewFor`]), shown to ReactiveUI through [`ControlViewFor`].

use super::view_support::{content_control, is_text, run_jobs, same, start, text, window_with};
use crate::{ControlViewFor, IControlViewFor, RoutedViewHost, ViewModelViewHost};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::rendering::IPresentationSource;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StaticType, StyledElementImpl, VisualImpl,
    VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, Control, ControlImpl, TransitioningContentControl, UserControl, Window};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{
    reactive_object, DefaultViewLocator, IRoutableViewModel, IScreen, IViewFor, IViewLocator, ReactiveObject, RoutingState,
    ViewLocator, ViewModelType,
};
use ferroui_splat::IEnableLogger;
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The default content used by the tests of `ViewModelViewHost` (`DefaultContentValue`).
const DEFAULT_CONTENT_VALUE: &str = "default";

/// The view contract used by the navigation tests (`ViewContractValue`).
const VIEW_CONTRACT_VALUE: &str = "contract";

// ---------------------------------------------------------------------------------------------
// View models, views and the locator of the tests
// ---------------------------------------------------------------------------------------------

/// A screen for testing routing (`ScreenImpl`).
struct ScreenImpl {
    reactive: ReactiveObject,
    router: Rc<RoutingState>,
}
reactive_object!(ScreenImpl);

impl IScreen for ScreenImpl {
    fn router(&self) -> Rc<RoutingState> {
        self.router.clone()
    }
}

impl ScreenImpl {
    fn new() -> Rc<Self> {
        Rc::new(Self { reactive: ReactiveObject::new(), router: RoutingState::new() })
    }
}

/// A routable view model for testing (`VmA`).
pub struct VmA {
    reactive: ReactiveObject,
    host_screen: Weak<ScreenImpl>,
}
reactive_object!(VmA);

impl IRoutableViewModel for VmA {
    fn url_path_segment(&self) -> Option<String> {
        Some("A".to_owned())
    }

    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.host_screen.upgrade().map(|screen| screen as Rc<dyn IScreen>)
    }
}

impl PartialEq for VmA {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl VmA {
    fn new(screen: &Rc<ScreenImpl>) -> Rc<Self> {
        Rc::new(Self { reactive: ReactiveObject::new(), host_screen: Rc::downgrade(screen) })
    }
}

/// A simple view model for testing non-routable navigation (`VmB`).
pub struct VmB {
    reactive: ReactiveObject,
}
reactive_object!(VmB);

impl PartialEq for VmB {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl VmB {
    fn new() -> Rc<Self> {
        Rc::new(Self { reactive: ReactiveObject::new() })
    }
}

/// An object without a view (`new()`).
#[derive(PartialEq)]
struct Plain;

fn plain() -> Option<Rc<dyn Any>> {
    Some(Rc::new(Plain))
}

/// A view for `VmA` (`ViewA : UserControl, IViewFor<VmA>`).
#[repr(C)]
pub(crate) struct ViewA {
    base: UserControl,
    view_model: RefCell<Option<Rc<VmA>>>,
}

ferro_class!(ViewA: UserControl);
ferro_impl_classes!(
    ViewA: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl ViewA {
    fn construct() -> Self {
        Self { base: UserControl::construct(), view_model: RefCell::new(None) }
    }

    fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The view as `IViewFor`.
    fn view_for(this: &Ref<Self>) -> Rc<dyn IViewFor> {
        ControlViewFor::new(this.clone())
    }
}

impl IControlViewFor for ViewA {
    type ViewModel = VmA;

    fn view_model_of(&self) -> Option<Rc<VmA>> {
        self.view_model.borrow().clone()
    }

    fn set_view_model_of(&self, view_model: Option<Rc<VmA>>) {
        *self.view_model.borrow_mut() = view_model;
    }
}

/// A view for `VmB` (`ViewB : UserControl, IViewFor<VmB>`).
#[repr(C)]
pub(crate) struct ViewB {
    base: UserControl,
    view_model: RefCell<Option<Rc<VmB>>>,
}

ferro_class!(ViewB: UserControl);
ferro_impl_classes!(
    ViewB: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl ViewB {
    fn construct() -> Self {
        Self { base: UserControl::construct(), view_model: RefCell::new(None) }
    }

    fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The view as `IViewFor`.
    fn view_for(this: &Ref<Self>) -> Rc<dyn IViewFor> {
        ControlViewFor::new(this.clone())
    }
}

impl IControlViewFor for ViewB {
    type ViewModel = VmB;

    fn view_model_of(&self) -> Option<Rc<VmB>> {
        self.view_model.borrow().clone()
    }

    fn set_view_model_of(&self, view_model: Option<Rc<VmB>>) {
        *self.view_model.borrow_mut() = view_model;
    }
}

/// A minimal view locator for the host tests (`StaticViewLocator`): it gives `view` for the
/// contract `contract`.
struct StaticViewLocator {
    view: Option<Rc<dyn IViewFor>>,
    contract: Option<String>,
}

impl StaticViewLocator {
    fn new(view: Option<Rc<dyn IViewFor>>, contract: Option<&str>) -> Rc<dyn IViewLocator> {
        Rc::new(Self { view, contract: contract.map(str::to_owned) })
    }

    /// Whether the requested contract matches this locator (`IsMatch`).
    fn is_match(&self, contract: Option<&str>) -> bool {
        self.contract.as_deref() == contract
    }
}

impl IEnableLogger for StaticViewLocator {}

impl IViewLocator for StaticViewLocator {
    fn resolve_view_typed(&self, _view_model: Rc<dyn Any>, _view_model_type: &ViewModelType, contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
        if self.is_match(contract) {
            self.view.clone()
        } else {
            None
        }
    }

    fn resolve_view(&self, _view_model: Option<Rc<dyn Any>>, contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
        if self.is_match(contract) {
            self.view.clone()
        } else {
            None
        }
    }

    fn resolve_view_unsafe(&self, view_model: Option<Rc<dyn Any>>, contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
        self.resolve_view(view_model, contract)
    }
}

/// Registers the test views into the current locator (`RegisterViews`).
fn register_views() {
    let locator: Rc<dyn Any> = ViewLocator::current();
    let locator = locator.downcast::<DefaultViewLocator>().expect("the current view locator is the default one");
    locator.map_factory::<VmA>(|| ViewA::view_for(&ViewA::new()), None);
    locator.map_factory::<VmB>(|| ViewB::view_for(&ViewB::new()), None);
}

/// A real presentation source from a shown window (`GetPresentationSource`).
fn get_presentation_source() -> (Ref<Window>, Rc<dyn IPresentationSource>) {
    let source: Rc<RefCell<Option<Rc<dyn IPresentationSource>>>> = Rc::new(RefCell::new(None));
    let control = UserControl::new();
    let slot = source.clone();
    let _attached = control.attached_to_visual_tree(move |args| *slot.borrow_mut() = Some(args.presentation_source().clone()));
    let window = window_with(Some(control.upcast()));

    window.show();
    let source = source.borrow().clone().expect("the control was attached");
    (window, source)
}

/// Records the presentation source of `host` when it is attached
/// (`host.AttachedToVisualTree += (_, args) => source = args.PresentationSource`).
fn record_source(host: &Control) -> (Rc<RefCell<Option<Rc<dyn IPresentationSource>>>>, Rc<dyn ferroui_base::reactive::IDisposable>) {
    let source: Rc<RefCell<Option<Rc<dyn IPresentationSource>>>> = Rc::new(RefCell::new(None));
    let slot = source.clone();
    let subscription = host.attached_to_visual_tree(move |args| *slot.borrow_mut() = Some(args.presentation_source().clone()));
    (source, subscription)
}

/// The attachment arguments of `host` in `source` (`new(this, source)`).
fn attachment<T: ferroui_base::ObjectType + ferroui_base::Upcast<ferroui_base::Visual>>(
    host: &Ref<T>,
    source: &Rc<dyn IPresentationSource>,
) -> VisualTreeAttachmentEventArgs {
    VisualTreeAttachmentEventArgs::new(Some(host.clone().upcast()), source.clone())
}

/// Navigates `router` to `view_model` (`router.Navigate.Execute(vm).Subscribe()`).
fn navigate(router: &RoutingState, view_model: &Rc<VmA>) {
    let _ = router.navigate().execute(Some(view_model.clone() as Rc<dyn IRoutableViewModel>)).subscribe_next(|_| {});
    run_jobs();
}

/// The view `ViewA` a content holds, if it holds one (`IsTypeOf<ViewA>()`).
fn view_a(content: Option<BoxedValue>) -> Option<Ref<ViewA>> {
    content_control(content).and_then(|control| control.cast::<ViewA>())
}

/// The view `ViewB` a content holds, if it holds one (`IsTypeOf<ViewB>()`).
fn view_b(content: Option<BoxedValue>) -> Option<Ref<ViewB>> {
    content_control(content).and_then(|control| control.cast::<ViewB>())
}

/// Whether `content` holds the control `view` (`IsSameReferenceAs(view)`).
fn holds_view<T: ferroui_base::ObjectType>(content: Option<BoxedValue>, view: &Ref<T>) -> bool {
    content_control(content).is_some_and(|control| control.ptr_eq(view))
}

fn as_any<T: 'static>(view_model: &Rc<T>) -> Option<Rc<dyn Any>> {
    Some(view_model.clone() as Rc<dyn Any>)
}

// ---------------------------------------------------------------------------------------------
// ViewModelViewHost
// ---------------------------------------------------------------------------------------------

#[test]
fn view_model_view_host_navigates_to_resolved_view() {
    let _app = start();
    register_views();
    let host = ViewModelViewHost::new();
    host.set_default_content(text(DEFAULT_CONTENT_VALUE));
    let vm = VmB::new();

    host.navigate_to_view_model(as_any(&vm), None);

    let view = view_b(host.content()).expect("the content is a ViewB");
    assert!(view.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));
}

#[test]
fn view_model_view_host_properties_return_assigned_values() {
    let _app = start();
    let locator = StaticViewLocator::new(Some(ViewB::view_for(&ViewB::new())), None);
    let host = ViewModelViewHost::new();
    host.set_default_content(text(DEFAULT_CONTENT_VALUE));
    host.set_view_contract(Some(VIEW_CONTRACT_VALUE.to_owned()));
    host.set_view_locator(Some(locator.clone()));
    let vm = VmB::new();

    host.set_view_model(Some(vm.clone() as BoxedValue));

    assert!(host.view_model().is_some_and(|view_model| same(&view_model, &vm)));
    assert!(is_text(&host.default_content(), DEFAULT_CONTENT_VALUE));
    assert_eq!(host.view_contract().as_deref(), Some(VIEW_CONTRACT_VALUE));
    assert!(host.view_locator().is_some_and(|current| same(&current, &locator)));
    assert!(std::ptr::eq(host.style_key(), <TransitioningContentControl as StaticType>::TYPE));
}

#[test]
fn view_model_view_host_attach_and_detach_manage_navigation_subscriptions() {
    let _app = start();
    let host = ViewModelViewHost::new();
    host.set_default_content(text(DEFAULT_CONTENT_VALUE));
    let (source, _attached) = record_source(&host);
    let window = window_with(Some(host.clone().upcast()));

    window.show();
    let source = source.borrow().clone().expect("the host was attached");

    VisualImpl::on_attached_to_visual_tree(&*host, &attachment(&host, &source));
    window.set_content(None);

    assert!(is_text(&host.content(), DEFAULT_CONTENT_VALUE));
    window.close();
}

#[test]
fn view_model_view_host_attached_subscription_navigates_on_view_model_change() {
    let _app = start();
    let view = ViewB::new();
    let host = ViewModelViewHost::new();
    host.set_default_content(text(DEFAULT_CONTENT_VALUE));
    host.set_view_locator(Some(StaticViewLocator::new(Some(ViewB::view_for(&view)), None)));
    let window = window_with(Some(host.clone().upcast()));
    let vm = VmB::new();

    window.show();
    host.set_view_model(Some(vm.clone() as BoxedValue));

    assert!(holds_view(host.content(), &view));
    assert!(view.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));
    window.close();
}

#[test]
fn view_model_view_host_detach_before_attach_does_not_throw() {
    let _app = start();
    let (window, source) = get_presentation_source();
    let host = ViewModelViewHost::new();
    host.set_default_content(text(DEFAULT_CONTENT_VALUE));

    VisualImpl::on_detached_from_visual_tree(&*host, &attachment(&host, &source));

    assert!(host.content().is_none());
    window.close();
}

#[test]
fn view_model_view_host_attach_then_manual_detach_disposes_navigation_subscription() {
    let _app = start();
    let (window, source) = get_presentation_source();
    let host = ViewModelViewHost::new();
    host.set_default_content(text(DEFAULT_CONTENT_VALUE));

    VisualImpl::on_attached_to_visual_tree(&*host, &attachment(&host, &source));
    VisualImpl::on_detached_from_visual_tree(&*host, &attachment(&host, &source));
    VisualImpl::on_detached_from_visual_tree(&*host, &attachment(&host, &source));

    assert!(is_text(&host.content(), DEFAULT_CONTENT_VALUE));
    window.close();
}

#[test]
fn view_model_view_host_dispose_navigation_disposables_when_empty_returns() {
    let _app = start();
    let host = ViewModelViewHost::new();

    host.dispose_navigation_disposables();

    assert!(host.content().is_none());
}

#[test]
fn view_model_view_host_when_no_view_falls_back_to_default() {
    let _app = start();
    let host = ViewModelViewHost::new();
    host.set_default_content(text(DEFAULT_CONTENT_VALUE));

    host.navigate_to_view_model(plain(), None);

    assert!(is_text(&host.content(), DEFAULT_CONTENT_VALUE));
}

#[test]
fn view_model_view_host_when_view_model_null_falls_back_to_default() {
    let _app = start();
    let host = ViewModelViewHost::new();
    host.set_default_content(text(DEFAULT_CONTENT_VALUE));

    host.navigate_to_view_model(None, None);

    assert!(is_text(&host.content(), DEFAULT_CONTENT_VALUE));
}

#[test]
fn view_model_view_host_navigates_to_contract_view() {
    let _app = start();
    let view = ViewB::new();
    let host = ViewModelViewHost::new();
    host.set_default_content(text(DEFAULT_CONTENT_VALUE));
    host.set_view_locator(Some(StaticViewLocator::new(Some(ViewB::view_for(&view)), Some(VIEW_CONTRACT_VALUE))));
    let vm = VmB::new();

    host.navigate_to_view_model(as_any(&vm), Some(VIEW_CONTRACT_VALUE));

    assert!(holds_view(host.content(), &view));
    assert!(view.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));
    assert!(view.data_context().is_some_and(|data_context| same(&data_context, &vm)));
}

#[test]
fn view_model_view_host_when_contract_view_missing_falls_back_to_default() {
    let _app = start();
    let host = ViewModelViewHost::new();
    host.set_default_content(text(DEFAULT_CONTENT_VALUE));
    host.set_view_locator(Some(StaticViewLocator::new(None, Some("different"))));

    host.navigate_to_view_model(as_any(&VmB::new()), Some(VIEW_CONTRACT_VALUE));

    assert!(is_text(&host.content(), DEFAULT_CONTENT_VALUE));
}

// ---------------------------------------------------------------------------------------------
// RoutedViewHost
// ---------------------------------------------------------------------------------------------

#[test]
fn routed_view_host_navigates_to_resolved_view_when_router_set() {
    let _app = start();
    register_views();
    let screen = ScreenImpl::new();
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));
    host.set_router(Some(screen.router()));
    let vm = VmA::new(&screen);

    host.navigate_to_view_model(as_any(&vm), None);

    let view = view_a(host.content()).expect("the content is a ViewA");
    assert!(view.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));
}

#[test]
fn routed_view_host_properties_return_assigned_values() {
    let _app = start();
    let router = RoutingState::new();
    let locator = StaticViewLocator::new(Some(ViewA::view_for(&ViewA::new())), None);
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));
    host.set_router(Some(router.clone()));
    host.set_view_contract(Some(VIEW_CONTRACT_VALUE.to_owned()));
    host.set_view_locator(Some(locator.clone()));

    assert!(host.router().is_some_and(|current| Rc::ptr_eq(&current, &router)));
    assert!(is_text(&host.default_content(), "def"));
    assert_eq!(host.view_contract().as_deref(), Some(VIEW_CONTRACT_VALUE));
    assert!(host.view_locator().is_some_and(|current| same(&current, &locator)));
    assert!(std::ptr::eq(host.style_key(), <TransitioningContentControl as StaticType>::TYPE));
}

#[test]
fn routed_view_host_attach_and_detach_manage_navigation_subscriptions() {
    let _app = start();
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));
    let (source, _attached) = record_source(&host);
    let window = window_with(Some(host.clone().upcast()));

    window.show();
    let source = source.borrow().clone().expect("the host was attached");

    VisualImpl::on_attached_to_visual_tree(&*host, &attachment(&host, &source));
    window.set_content(None);

    assert!(is_text(&host.content(), "def"));
    window.close();
}

#[test]
fn routed_view_host_attached_subscription_navigates_on_router_navigation() {
    let _app = start();
    register_views();
    let screen = ScreenImpl::new();
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));
    host.set_router(Some(screen.router()));
    let window = window_with(Some(host.clone().upcast()));
    let vm = VmA::new(&screen);

    window.show();
    navigate(&screen.router(), &vm);

    let view = view_a(host.content()).expect("the content is a ViewA");
    assert!(view.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));
    window.close();
}

#[test]
fn routed_view_host_attached_subscription_navigates_when_router_already_has_current_view_model() {
    let _app = start();
    register_views();
    let screen = ScreenImpl::new();
    let vm = VmA::new(&screen);
    navigate(&screen.router(), &vm);
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));
    host.set_router(Some(screen.router()));
    let window = window_with(Some(host.clone().upcast()));

    assert_eq!(screen.router().navigation_stack().count(), 1);

    window.show();

    let view = view_a(host.content()).expect("the content is a ViewA");
    assert!(view.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));
    window.close();
}

#[test]
fn routed_view_host_attached_subscription_navigates_when_already_navigated_router_is_assigned() {
    let _app = start();
    register_views();
    let screen = ScreenImpl::new();
    let vm = VmA::new(&screen);
    navigate(&screen.router(), &vm);
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));
    let window = window_with(Some(host.clone().upcast()));

    window.show();
    host.set_router(Some(screen.router()));

    assert_eq!(screen.router().navigation_stack().count(), 1);
    let view = view_a(host.content()).expect("the content is a ViewA");
    assert!(view.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));
    window.close();
}

#[test]
fn routed_view_host_attached_subscription_ignores_previous_router_after_replacement() {
    let _app = start();
    let first_screen = ScreenImpl::new();
    let second_screen = ScreenImpl::new();
    let first_vm = VmA::new(&first_screen);
    let second_vm = VmA::new(&second_screen);
    let view = ViewA::new();
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));
    host.set_router(Some(first_screen.router()));
    host.set_view_locator(Some(StaticViewLocator::new(Some(ViewA::view_for(&view)), None)));
    let window = window_with(Some(host.clone().upcast()));

    window.show();
    navigate(&first_screen.router(), &first_vm);
    assert!(view.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &first_vm)));

    host.set_router(Some(second_screen.router()));
    navigate(&second_screen.router(), &second_vm);
    assert!(view.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &second_vm)));

    navigate(&first_screen.router(), &VmA::new(&first_screen));
    assert!(view.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &second_vm)));
    window.close();
}

#[test]
fn routed_view_host_attached_subscription_ignores_previous_router_after_router_cleared() {
    let _app = start();
    let screen = ScreenImpl::new();
    let vm = VmA::new(&screen);
    let view = ViewA::new();
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));
    host.set_router(Some(screen.router()));
    host.set_view_locator(Some(StaticViewLocator::new(Some(ViewA::view_for(&view)), None)));
    let window = window_with(Some(host.clone().upcast()));

    window.show();
    navigate(&screen.router(), &vm);
    assert!(view.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));

    host.set_router(None);
    navigate(&screen.router(), &VmA::new(&screen));

    assert!(is_text(&host.content(), "def"));
    assert!(view.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));
    window.close();
}

#[test]
fn routed_view_host_attached_subscription_catches_up_to_current_view_model_after_reattach() {
    let _app = start();
    let screen = ScreenImpl::new();
    let first_vm = VmA::new(&screen);
    let second_vm = VmA::new(&screen);
    let view = ViewA::new();
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));
    host.set_router(Some(screen.router()));
    host.set_view_locator(Some(StaticViewLocator::new(Some(ViewA::view_for(&view)), None)));
    let window = window_with(Some(host.clone().upcast()));

    window.show();
    navigate(&screen.router(), &first_vm);
    assert!(view.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &first_vm)));

    window.set_content(None);
    navigate(&screen.router(), &second_vm);
    assert!(view.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &first_vm)));

    window.set_content(Some(Control::boxed(host.clone())));
    run_jobs();
    assert!(view.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &second_vm)));
    window.close();
}

#[test]
fn routed_view_host_attached_subscription_falls_back_when_router_removed() {
    let _app = start();
    let screen = ScreenImpl::new();
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));
    host.set_router(Some(screen.router()));
    let window = window_with(Some(host.clone().upcast()));

    window.show();
    host.set_router(None);

    assert!(is_text(&host.content(), "def"));
    window.close();
}

#[test]
fn routed_view_host_detach_before_attach_does_not_throw() {
    let _app = start();
    let (window, source) = get_presentation_source();
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));

    VisualImpl::on_detached_from_visual_tree(&*host, &attachment(&host, &source));

    assert!(host.content().is_none());
    window.close();
}

#[test]
fn routed_view_host_attach_then_manual_detach_disposes_navigation_subscription() {
    let _app = start();
    let (window, source) = get_presentation_source();
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));
    host.set_router(Some(RoutingState::new()));

    VisualImpl::on_attached_to_visual_tree(&*host, &attachment(&host, &source));
    VisualImpl::on_detached_from_visual_tree(&*host, &attachment(&host, &source));
    VisualImpl::on_detached_from_visual_tree(&*host, &attachment(&host, &source));

    assert!(is_text(&host.content(), "def"));
    window.close();
}

#[test]
fn routed_view_host_dispose_navigation_disposables_when_empty_returns() {
    let _app = start();
    let host = RoutedViewHost::new();

    host.dispose_navigation_disposables();

    assert!(host.content().is_none());
}

#[test]
fn routed_view_host_no_router_falls_back_to_default() {
    let _app = start();
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));

    host.navigate_to_view_model(plain(), None);

    assert!(is_text(&host.content(), "def"));
}

#[test]
fn routed_view_host_when_view_model_null_falls_back_to_default() {
    let _app = start();
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));
    host.set_router(Some(RoutingState::new()));

    host.navigate_to_view_model(None, None);

    assert!(is_text(&host.content(), "def"));
}

#[test]
fn routed_view_host_navigates_to_contract_view() {
    let _app = start();
    let view = ViewA::new();
    let screen = ScreenImpl::new();
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));
    host.set_router(Some(screen.router()));
    host.set_view_locator(Some(StaticViewLocator::new(Some(ViewA::view_for(&view)), Some(VIEW_CONTRACT_VALUE))));
    let vm = VmA::new(&screen);

    host.navigate_to_view_model(as_any(&vm), Some(VIEW_CONTRACT_VALUE));

    assert!(holds_view(host.content(), &view));
    assert!(view.view_model_of().is_some_and(|view_model| Rc::ptr_eq(&view_model, &vm)));
    assert!(view.data_context().is_some_and(|data_context| same(&data_context, &vm)));
}

#[test]
fn routed_view_host_when_default_view_missing_falls_back_to_default() {
    let _app = start();
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));
    host.set_router(Some(RoutingState::new()));
    host.set_view_locator(Some(StaticViewLocator::new(None, None)));

    host.navigate_to_view_model(plain(), None);

    assert!(is_text(&host.content(), "def"));
}

#[test]
fn routed_view_host_when_contract_view_missing_falls_back_to_default() {
    let _app = start();
    let host = RoutedViewHost::new();
    host.set_default_content(text("def"));
    host.set_router(Some(RoutingState::new()));
    host.set_view_locator(Some(StaticViewLocator::new(None, Some("different"))));

    host.navigate_to_view_model(plain(), Some(VIEW_CONTRACT_VALUE));

    assert!(is_text(&host.content(), "def"));
}

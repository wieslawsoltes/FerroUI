use crate::controls::{DockReactiveUserControl, DockReactiveUserControlBase};
use ferrodock_model_reactiveui_services::lifecycle::IActivationBridge;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StaticType,
    StyledElementImpl, TypeInfo, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{AppBuilder, Application, ContentControlImpl, Control, ControlImpl, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactiveui::{
    reactive_object, IActivatableViewModel, IViewFor, ReactiveObject, ViewModelActivationMixins,
    ViewModelActivator,
};
use ferroui_reactiveui_controls::{
    view_control, AppBuilderExtensions, ITypedReactiveViewClass, ReactiveViewFor,
};
use ferroui_splat::{IMutableDependencyResolver, Locator, MutableDependencyResolverExt};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .use_reactive_ui(|_| {})
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-reactive-services-headless",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    let session = HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY));
    session
        .dispatch(
            move || {
                test();
                jobs();
            },
            CancellationToken::none(),
        )
        .wait()
        .expect("headless test completed");
}
fn jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}
struct Model {
    reactive: ReactiveObject,
    activator: ViewModelActivator,
    active: Rc<Cell<bool>>,
    setups: Rc<Cell<usize>>,
}
reactive_object!(Model);
impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl IActivatableViewModel for Model {
    fn activator(&self) -> &ViewModelActivator {
        &self.activator
    }
}
impl Model {
    fn new() -> Rc<Self> {
        let model = Rc::new(Self {
            reactive: ReactiveObject::new(),
            activator: ViewModelActivator::new(),
            active: Rc::new(Cell::new(false)),
            setups: Rc::new(Cell::new(0)),
        });
        let active = model.active.clone();
        let setups = model.setups.clone();
        model.when_activated(move || {
            active.set(true);
            setups.set(setups.get() + 1);
            let active = active.clone();
            vec![Disposable::create(move || active.set(false))]
        });
        model
    }
}
type BridgeLog = Rc<RefCell<Vec<(&'static str, &'static str, usize)>>>;

struct Bridge {
    name: &'static str,
    log: BridgeLog,
    view: RefCell<Option<Ref<Control>>>,
}
impl IActivationBridge for Bridge {
    fn track(
        &self,
        model: Rc<dyn IActivatableViewModel>,
        view: Rc<dyn IViewFor>,
    ) -> Rc<dyn IDisposable> {
        let key = Rc::as_ptr(&model) as *const () as usize;
        self.log.borrow_mut().push((self.name, "track", key));
        self.view.replace(view_control(view.as_ref()));
        let scope = model.activator().activate();
        let log = self.log.clone();
        let name = self.name;
        Disposable::create(move || {
            log.borrow_mut().push((name, "dispose", key));
            scope.dispose();
        })
    }
}
struct BridgeRegistration(Rc<dyn IMutableDependencyResolver>);
impl BridgeRegistration {
    fn install(bridge: Rc<dyn IActivationBridge>) -> Self {
        let services = Locator::current_mutable();
        services.register_constant::<dyn IActivationBridge>(bridge);
        Self(services)
    }
}
impl Drop for BridgeRegistration {
    fn drop(&mut self) {
        self.0.unregister_current::<dyn IActivationBridge>();
    }
}
fn bridge(name: &'static str, log: BridgeLog) -> Rc<Bridge> {
    Rc::new(Bridge {
        name,
        log,
        view: RefCell::new(None),
    })
}
fn window(control: Ref<Control>) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(control)));
    window
}
#[test]
fn bridge_is_resolved_once_per_activation_and_serial_replacement_tracks_new_before_disposing_old() {
    run(|| {
        let view = DockReactiveUserControl::<Model>::new();
        let first = Model::new();
        let second = Model::new();
        view.register_activatable_view_model(&first);
        view.register_activatable_view_model(&second);
        let erased: Rc<dyn std::any::Any> = first.clone();
        assert!(std::ptr::addr_eq(
            Rc::as_ptr(
                &view
                    .native_types()
                    .reference::<dyn IActivatableViewModel>(&erased)
                    .unwrap()
            ),
            Rc::as_ptr(&first)
        ));
        drop(erased);
        let log = Rc::new(RefCell::new(Vec::new()));
        let a = bridge("a", log.clone());
        let _a = BridgeRegistration::install(a.clone());
        view.set_view_model(Some(first.clone()));
        let window = window(view.control().clone().upcast());
        window.show();
        jobs();
        let first_key = Rc::as_ptr(&first) as usize;
        let second_key = Rc::as_ptr(&second) as usize;
        assert_eq!(&*log.borrow(), &[("a", "track", first_key)]);
        assert!(first.active.get());
        assert_eq!(first.setups.get(), 1);
        assert_eq!(
            a.view.borrow().as_ref(),
            Some(&view.control().clone().upcast::<Control>())
        );
        let b = bridge("b", log.clone());
        let _b = BridgeRegistration::install(b.clone());
        view.set_view_model(Some(second.clone()));
        jobs();
        assert_eq!(
            &*log.borrow(),
            &[
                ("a", "track", first_key),
                ("a", "track", second_key),
                ("a", "dispose", first_key)
            ]
        );
        assert!(!first.active.get());
        assert!(second.active.get());
        window.set_content(None);
        jobs();
        assert!(!second.active.get());
        window.set_content(Some(Control::boxed(
            view.control().clone().upcast::<Control>(),
        )));
        jobs();
        assert_eq!(log.borrow().last(), Some(&("b", "track", second_key)));
        window.close();
        jobs();
        assert!(!second.active.get());
        assert_eq!(log.borrow().last(), Some(&("b", "dispose", second_key)));
        a.view.borrow_mut().take();
        b.view.borrow_mut().take();
    });
}
#[test]
fn nonactivatable_and_null_values_clear_the_serial_handle_and_unload_removes_subscription() {
    run(|| {
        let control = <DockReactiveUserControlBase as ITypedReactiveViewClass>::create();
        let model = Model::new();
        control.register_activatable_view_model(&model);
        let view = ReactiveViewFor::new(control.clone());
        let log = Rc::new(RefCell::new(Vec::new()));
        let probe = bridge("probe", log.clone());
        let _registration = BridgeRegistration::install(probe.clone());
        let window = window(control.clone().upcast());
        window.show();
        jobs();
        assert!(log.borrow().is_empty());
        view.set_view_model(Some(model.clone()));
        jobs();
        assert_eq!(log.borrow().len(), 1);
        view.set_view_model(Some(Rc::new("plain".to_owned())));
        jobs();
        assert_eq!(log.borrow().len(), 2);
        assert_eq!(log.borrow()[1].1, "dispose");
        assert!(!model.active.get());
        view.set_view_model(None);
        jobs();
        assert_eq!(log.borrow().len(), 2);
        window.set_content(None);
        jobs();
        view.set_view_model(Some(model.clone()));
        jobs();
        assert_eq!(log.borrow().len(), 2);
        assert!(!model.active.get());
        window.close();
        probe.view.borrow_mut().take();
    });
}
#[test]
fn no_bridge_uses_shared_activation_and_cleared_models_are_released_while_view_stays_alive() {
    run(|| {
        let view = DockReactiveUserControl::<Model>::new();
        let first = Model::new();
        let second = Model::new();
        view.register_activatable_view_model(&first);
        view.register_activatable_view_model(&second);
        let weak = Rc::downgrade(&first);
        view.set_view_model(Some(first.clone()));
        let window = window(view.control().clone().upcast());
        window.show();
        jobs();
        assert!(first.active.get());
        view.set_view_model(Some(second.clone()));
        jobs();
        assert!(!first.active.get());
        assert!(second.active.get());
        drop(first);
        assert!(weak.upgrade().is_none());
        view.set_view_model(None);
        jobs();
        assert!(!second.active.get());
        assert!(view.view_model().is_none());
        assert!(view.data_context().is_none());
        window.close();
        jobs();
    });
}
#[repr(C)]
struct DerivedView {
    base: DockReactiveUserControlBase,
}
ferro_class!(DerivedView:DockReactiveUserControlBase);
ferro_impl_classes!(DerivedView:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
#[test]
fn derived_view_uses_inherited_typed_property_sync_and_abstract_class_metadata() {
    run(|| {
        crate::register_types();
        assert!(std::ptr::eq(
            ferroui_base::metadata::MarkupAssembly::find(crate::ASSEMBLY.name).unwrap(),
            &crate::ASSEMBLY
        ));
        let type_ = TypeInfo::find(
            "FerroDock.Model.ReactiveUI.Services.Ferro.Controls",
            "DockReactiveUserControlBase",
        )
        .unwrap();
        assert!(std::ptr::eq(
            type_,
            <DockReactiveUserControlBase as StaticType>::TYPE
        ));
        assert!(type_.default_constructor().is_none());
        let derived = instantiate(DerivedView {
            base: DockReactiveUserControlBase::construct(),
        });
        let view = DockReactiveUserControl::<Model>::from_control(derived.clone());
        let model = Model::new();
        view.register_activatable_view_model(&model);
        view.set_view_model(Some(model.clone()));
        assert!(view.data_context().is_some());
        assert!(Rc::ptr_eq(&view.view_model().unwrap(), &model));
        let window = window(derived.clone().upcast());
        window.show();
        jobs();
        assert!(model.active.get());
        window.close();
        jobs();
        assert!(!model.active.get());
    });
}

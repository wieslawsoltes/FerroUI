use super::VisualTreeLifecycleBehavior as Behavior;
use ferrodock_model::services::IVisualTreeLifecycle;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl,
    Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::{AppBuilder, Application, Border, Control, ControlImpl, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
type Trace = Rc<RefCell<Vec<String>>>;
type Callback = Option<Rc<dyn Fn()>>;
struct Model {
    name: &'static str,
    attached: Cell<usize>,
    detached: Cell<usize>,
    trace: Trace,
    on_attach: RefCell<Callback>,
    on_detach: RefCell<Callback>,
}
impl Model {
    fn new(name: &'static str, trace: &Trace) -> Rc<Self> {
        Rc::new(Self {
            name,
            attached: Cell::new(0),
            detached: Cell::new(0),
            trace: trace.clone(),
            on_attach: RefCell::new(None),
            on_detach: RefCell::new(None),
        })
    }
}
impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl IVisualTreeLifecycle for Model {
    fn on_attached_to_visual_tree(&self) {
        self.attached.set(self.attached.get() + 1);
        self.trace.borrow_mut().push(format!("{}+", self.name));
        let action = self.on_attach.borrow_mut().take();
        if let Some(action) = action {
            action();
        }
    }
    fn on_detached_from_visual_tree(&self) {
        self.detached.set(self.detached.get() + 1);
        self.trace.borrow_mut().push(format!("{}-", self.name));
        let action = self.on_detach.borrow_mut().take();
        if let Some(action) = action {
            action();
        }
    }
}
#[repr(C)]
struct LifecycleControl {
    base: Control,
    model: Rc<Model>,
    handle: RefCell<Weak<Handle>>,
}
ferro_class!(LifecycleControl:Control);
ferro_impl_classes!(LifecycleControl:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_class_info!(LifecycleControl {interfaces:[Rc<dyn IVisualTreeLifecycle> => LifecycleControl::as_lifecycle]});
impl LifecycleControl {
    fn new(trace: &Trace) -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            model: Model::new("control", trace),
            handle: RefCell::new(Weak::new()),
        })
    }
    fn as_lifecycle(this: Ref<Self>) -> Rc<dyn IVisualTreeLifecycle> {
        if let Some(handle) = this.handle.borrow().upgrade() {
            return handle;
        }
        let handle = Rc::new(Handle(this.clone()));
        this.handle.replace(Rc::downgrade(&handle));
        handle
    }
}
struct Handle(Ref<LifecycleControl>);
impl IVisualTreeLifecycle for Handle {
    fn on_attached_to_visual_tree(&self) {
        self.0.model.on_attached_to_visual_tree();
    }
    fn on_detached_from_visual_tree(&self) {
        self.0.model.on_detached_from_visual_tree();
    }
}
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-visual-lifecycle",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                ValueTypes::register_cast::<Rc<Model>, Rc<dyn IVisualTreeLifecycle>>(|model| {
                    model.clone()
                });
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
fn context(model: &Rc<Model>) -> BoxedValue {
    Rc::new(model.clone())
}
struct Scene {
    window: Ref<Window>,
}
impl Scene {
    fn new(control: Ref<Control>) -> Self {
        let window = Window::new();
        window.set_width(300.0);
        window.set_height(200.0);
        window.set_content(Some(Control::boxed(control)));
        window.show();
        let scene = Self { window };
        scene.update();
        scene
    }
    fn update(&self) {
        Dispatcher::ui_thread().run_jobs(None);
        self.window.update_layout();
    }
    fn remove(&self) {
        self.window.set_content(None);
        self.update();
    }
    fn add(&self, control: Ref<Control>) {
        self.window.set_content(Some(Control::boxed(control)));
        self.update();
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.window.close();
    }
}
fn assert_trace(trace: &Trace, expected: &[&str]) {
    assert_eq!(&*trace.borrow(), expected);
}

#[test]
fn source_reattachment_scenario_invokes_control_and_data_context_once_each() {
    run(|| {
        let trace = Trace::default();
        let control = LifecycleControl::new(&trace);
        let model = Model::new("model", &trace);
        control.set_data_context(Some(context(&model)));
        Behavior::set_is_enabled(&control, true);
        let scene = Scene::new(control.clone().upcast());
        assert_eq!(control.model.attached.get(), 1);
        assert_eq!(model.attached.get(), 1);
        assert_trace(&trace, &["control+", "model+"]);
        scene.remove();
        assert_eq!(control.model.detached.get(), 1);
        assert_eq!(model.detached.get(), 1);
        assert_trace(&trace, &["control+", "model+", "model-", "control-"]);
        scene.add(control.clone().upcast());
        assert_eq!(control.model.attached.get(), 2);
        assert_eq!(model.attached.get(), 2);
    });
}
#[test]
fn enabling_and_disabling_an_attached_control_are_immediate_and_repeat_values_do_not_duplicate() {
    run(|| {
        let trace = Trace::default();
        let control = LifecycleControl::new(&trace);
        let model = Model::new("model", &trace);
        control.set_data_context(Some(context(&model)));
        assert!(!Behavior::get_is_enabled(&control));
        let scene = Scene::new(control.clone().upcast());
        assert!(trace.borrow().is_empty());
        Behavior::set_is_enabled(&control, true);
        Behavior::set_is_enabled(&control, true);
        assert!(Behavior::get_is_enabled(&control));
        assert_trace(&trace, &["control+", "model+"]);
        Behavior::set_is_enabled(&control, false);
        Behavior::set_is_enabled(&control, false);
        assert_trace(&trace, &["control+", "model+", "model-", "control-"]);
        scene.remove();
        scene.add(control.clone().upcast());
        assert_eq!(trace.borrow().len(), 4);
        Behavior::set_is_enabled(&control, true);
        assert_eq!(control.model.attached.get(), 2);
        assert_eq!(model.attached.get(), 2);
    });
}
#[test]
fn data_context_replacement_uses_notification_arguments_across_nested_replacement() {
    run(|| {
        let trace = Trace::default();
        let control = LifecycleControl::new(&trace);
        let first = Model::new("first", &trace);
        let second = Model::new("second", &trace);
        let third = Model::new("third", &trace);
        control.set_data_context(Some(context(&first)));
        Behavior::set_is_enabled(&control, true);
        let _scene = Scene::new(control.clone().upcast());
        trace.borrow_mut().clear();
        let weak = control.downgrade();
        let replacement = third.clone();
        first.on_detach.replace(Some(Rc::new(move || {
            weak.upgrade()
                .unwrap()
                .set_data_context(Some(context(&replacement)))
        })));
        control.set_data_context(Some(context(&second)));
        assert_trace(&trace, &["first-", "second-", "third+", "second+"]);
        assert_eq!(control.model.attached.get(), 1);
        assert_eq!(control.model.detached.get(), 0);
        assert!(Rc::ptr_eq(
            control
                .data_context()
                .unwrap()
                .downcast_ref::<Rc<Model>>()
                .unwrap(),
            &third
        ));
    });
}
#[test]
fn control_attach_hook_rereads_current_context_after_nested_context_notifications() {
    run(|| {
        let trace = Trace::default();
        let control = LifecycleControl::new(&trace);
        let old = Model::new("old", &trace);
        let new = Model::new("new", &trace);
        control.set_data_context(Some(context(&old)));
        let weak = control.downgrade();
        let replacement = new.clone();
        control.model.on_attach.replace(Some(Rc::new(move || {
            weak.upgrade()
                .unwrap()
                .set_data_context(Some(context(&replacement)))
        })));
        Behavior::set_is_enabled(&control, true);
        let _scene = Scene::new(control.clone().upcast());
        assert_trace(&trace, &["control+", "old-", "new+", "new+"]);
        assert_eq!(old.attached.get(), 0);
        assert_eq!(new.attached.get(), 2);
    });
}
#[test]
fn detach_context_hook_keeps_presentation_source_alive_for_nested_context_changes() {
    run(|| {
        let trace = Trace::default();
        let control = LifecycleControl::new(&trace);
        let old = Model::new("old", &trace);
        let new = Model::new("new", &trace);
        control.set_data_context(Some(context(&old)));
        Behavior::set_is_enabled(&control, true);
        let scene = Scene::new(control.clone().upcast());
        trace.borrow_mut().clear();
        let weak = control.downgrade();
        let replacement = new.clone();
        old.on_detach.replace(Some(Rc::new(move || {
            weak.upgrade()
                .unwrap()
                .set_data_context(Some(context(&replacement)))
        })));
        scene.remove();
        // Upstream 12.1.3 and FerroUI clear PresentationSource AFTER raising
        // DetachedFromVisualTree. Nested context replacement therefore still
        // takes the attached-context branch; do not suppress this source behavior.
        assert_trace(&trace, &["old-", "old-", "new+", "control-"]);
        assert_eq!(new.attached.get(), 1);
        scene.add(control.clone().upcast());
        assert_eq!(new.attached.get(), 2);
    });
}
#[test]
fn disable_removes_tree_subscriptions_before_callbacks_reparent_the_element() {
    run(|| {
        let trace = Trace::default();
        let control = LifecycleControl::new(&trace);
        let model = Model::new("model", &trace);
        control.set_data_context(Some(context(&model)));
        Behavior::set_is_enabled(&control, true);
        let scene = Scene::new(control.clone().upcast());
        trace.borrow_mut().clear();
        let window = scene.window.clone();
        model
            .on_detach
            .replace(Some(Rc::new(move || window.set_content(None))));
        Behavior::set_is_enabled(&control, false);
        scene.update();
        assert_trace(&trace, &["model-", "control-"]);
        assert_eq!(model.detached.get(), 1);
        assert_eq!(control.model.detached.get(), 1);
    });
}
#[test]
fn same_element_context_is_suppressed_only_by_tree_helpers_for_all_native_handle_forms() {
    run(|| {
        for representation in 0..3 {
            let trace = Trace::default();
            let control = LifecycleControl::new(&trace);
            let data: BoxedValue = match representation {
                0 => Control::boxed(control.clone()),
                1 => Rc::new(control.clone()),
                _ => Rc::new(LifecycleControl::as_lifecycle(control.clone())),
            };
            control.set_data_context(Some(data.clone()));
            Behavior::set_is_enabled(&control, true);
            let scene = Scene::new(control.clone().upcast());
            assert_trace(&trace, &["control+"]);
            control.set_data_context(None);
            assert_trace(&trace, &["control+", "control-"]);
            control.set_data_context(Some(data));
            assert_trace(&trace, &["control+", "control-", "control+"]);
            scene.remove();
            assert_trace(&trace, &["control+", "control-", "control+", "control-"]);
            // Clear the source self-reference explicitly; native Rc is not a CLR collector.
            control.set_data_context(None);
        }
    });
}
#[test]
fn plain_control_null_and_non_lifecycle_contexts_and_disabled_changes_are_ignored() {
    run(|| {
        let trace = Trace::default();
        let control = Border::new();
        let first = Model::new("first", &trace);
        let second = Model::new("second", &trace);
        control.set_data_context(Some(context(&first)));
        Behavior::set_is_enabled(&control, true);
        control.set_data_context(Some(context(&second)));
        assert!(trace.borrow().is_empty());
        let scene = Scene::new(control.clone().upcast());
        assert_trace(&trace, &["second+"]);
        control.set_data_context(Some(Rc::new("ordinary value".to_owned())));
        assert_trace(&trace, &["second+", "second-"]);
        Behavior::set_is_enabled(&control, false);
        control.set_data_context(Some(context(&first)));
        scene.remove();
        assert_eq!(trace.borrow().len(), 2);
    });
}
#[test]
fn native_token_bookkeeping_does_not_retain_enabled_control_or_its_model() {
    run(|| {
        let trace = Trace::default();
        let control = Border::new();
        let model = Model::new("model", &trace);
        let weak_control = control.downgrade();
        let weak_model = Rc::downgrade(&model);
        control.set_data_context(Some(context(&model)));
        Behavior::set_is_enabled(&control, true);
        let scene = Scene::new(control.clone().upcast());
        scene.remove();
        drop(scene);
        drop(control);
        drop(model);
        assert!(weak_control.upgrade().is_none());
        assert!(weak_model.upgrade().is_none());
        let other = Border::new();
        Behavior::set_is_enabled(&other, true);
        Behavior::set_is_enabled(&other, false);
    });
}

#[test]
fn disabling_inside_attach_does_not_cancel_the_source_remaining_context_hook() {
    run(|| {
        let trace = Trace::default();
        let control = LifecycleControl::new(&trace);
        let model = Model::new("model", &trace);
        control.set_data_context(Some(context(&model)));
        let scene = Scene::new(control.clone().upcast());
        let weak = control.downgrade();
        control.model.on_attach.replace(Some(Rc::new(move || {
            Behavior::set_is_enabled(&weak.upgrade().unwrap(), false)
        })));
        Behavior::set_is_enabled(&control, true);
        assert!(!Behavior::get_is_enabled(&control));
        assert_trace(&trace, &["control+", "model-", "control-", "model+"]);
        scene.remove();
        assert_eq!(trace.borrow().len(), 4);
    });
}

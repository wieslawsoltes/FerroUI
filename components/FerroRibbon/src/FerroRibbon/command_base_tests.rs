use crate::commands::{adapt_core_command, command_parameter, source_core_command};
use crate::controls::{
    IRibbonCommandSource, Ribbon, RibbonControlBase, RibbonControlBaseImpl,
    RibbonControlBaseImplExt, RibbonGroup, RibbonItemHelper, RibbonSplitButton, RibbonTab,
};
use crate::tests::run;
use ferroribbon_core::commands::{ICommand as CoreCommand, RibbonRelayCommand};
use ferroribbon_core::model::RibbonObject;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::{InputElement, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class,
    ferro_impl_classes, instantiate,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{Control, ControlImpl, Window};
use ferroui_system::events::Event;
use serde_json::{Value, json};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
};

#[repr(C)]
struct Probe {
    base: RibbonControlBase,
    uses: Cell<bool>,
    updates: Cell<i32>,
}
ferro_class!(Probe:RibbonControlBase);
ferro_impl_classes!(Probe:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl Probe {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonControlBase::construct(),
            uses: Cell::new(true),
            updates: Cell::new(0),
        })
    }
}
impl RibbonControlBaseImpl for Probe {
    fn uses_command_can_execute(this: &Self) -> bool {
        this.uses.get()
    }
    fn update_can_execute(this: &Self) {
        this.updates.set(this.updates.get() + 1);
        Self::parent_update_can_execute(this)
    }
}
struct TraceCommand {
    can: Cell<bool>,
    positive: Cell<bool>,
    calls: Cell<i32>,
    executions: Cell<i32>,
    adds: Cell<i32>,
    removes: Rc<Cell<i32>>,
    changed: Rc<Event<()>>,
    on_can: RefCell<Option<Rc<dyn Fn()>>>,
    parameters: RefCell<Vec<Option<i32>>>,
}
impl TraceCommand {
    fn new(can: bool) -> Rc<Self> {
        Rc::new(Self {
            can: Cell::new(can),
            positive: Cell::new(false),
            calls: Cell::new(0),
            executions: Cell::new(0),
            adds: Cell::new(0),
            removes: Rc::new(Cell::new(0)),
            changed: Rc::new(Event::new()),
            on_can: RefCell::new(None),
            parameters: RefCell::new(Vec::new()),
        })
    }
    fn raise(&self) {
        self.changed.raise(&())
    }
}
impl CoreCommand for TraceCommand {
    fn can_execute(&self, p: Option<&dyn Any>) -> bool {
        self.calls.set(self.calls.get() + 1);
        let callback = self.on_can.borrow().clone();
        if let Some(callback) = callback {
            callback()
        }
        self.can.get()
            && (!self.positive.get()
                || p.and_then(|p| p.downcast_ref::<i32>())
                    .is_some_and(|n| *n > 0))
    }
    fn execute(&self, p: Option<&dyn Any>) {
        self.executions.set(self.executions.get() + 1);
        self.parameters
            .borrow_mut()
            .push(p.and_then(|p| p.downcast_ref::<i32>()).copied())
    }
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.adds.set(self.adds.get() + 1);
        let token = self.changed.add(Rc::new(move |_| handler()));
        let weak = Rc::downgrade(&self.changed);
        let removes = self.removes.clone();
        Disposable::create(move || {
            removes.set(removes.get() + 1);
            if let Some(event) = weak.upgrade() {
                event.remove(token);
            }
        })
    }
}
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/command-base.json")).unwrap()
}
struct Mounted(Ref<Window>);
impl Mounted {
    fn new(control: &Ref<Probe>) -> Self {
        let window = Window::new();
        window.set_content(Some(Control::boxed(control)));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(control.is_loaded());
        Self(window)
    }
    fn detach(&self) {
        self.0.set_content(None);
        Dispatcher::ui_thread().run_jobs(None)
    }
    fn attach(&self, p: &Ref<Probe>) {
        self.0.set_content(Some(Control::boxed(p)));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(p.is_loaded())
    }
}
impl Drop for Mounted {
    fn drop(&mut self) {
        self.0.close();
        Dispatcher::ui_thread().run_jobs(None)
    }
}
fn snapshot(p: &Probe, a: &TraceCommand, b: &TraceCommand, step: &str) -> Value {
    json!({"step":step,"enabled":p.is_enabled(),"loaded":p.is_loaded(),"updates":p.updates.get(),"aCalls":a.calls.get(),"bCalls":b.calls.get(),"aSubs":a.changed.handler_count(),"bSubs":b.changed.handler_count(),"aAdds":a.adds.get(),"aRemoves":a.removes.get()})
}

#[test]
fn split_command_tracks_ancestor_enablement_without_replacing_local_input() {
    crate::group_tests::run(|| {
        let ribbon = Ribbon::new();
        let tab = RibbonTab::new();
        let group = RibbonGroup::new();
        let split = RibbonSplitButton::new();
        split.set_label(Some("Command enablement probe".into()));
        group.items().add(Some(split.clone().upcast()));
        tab.groups().add(Some(group.clone()));
        ribbon.tabs().add(Some(tab));
        let window = crate::ribbon_tests::mount(&ribbon);
        assert!(split.is_loaded());

        let command = TraceCommand::new(true);
        split.set_command(Some(adapt_core_command(command.clone())));
        let mut rows = Vec::new();
        let mut observe = |phase| {
            rows.push((
                phase,
                split.is_effectively_enabled(),
                split.get_local_value(InputElement::is_enabled_property()),
                command.calls.get(),
            ));
        };
        observe("initial");
        group.set_is_enabled(false);
        observe("group-disabled");
        command.can.set(false);
        command.raise();
        observe("denied-under-disabled-group");
        group.set_is_enabled(true);
        observe("group-restored-with-command-denied");
        command.can.set(true);
        command.raise();
        observe("command-recovered");

        split.set_is_enabled(false);
        group.set_is_enabled(false);
        command.can.set(false);
        command.raise();
        group.set_is_enabled(true);
        command.can.set(true);
        command.raise();
        observe("application-disabled");
        split.set_is_enabled(true);
        observe("application-enabled");
        window.close();

        assert_eq!(
            rows,
            [
                ("initial", true, None, 2),
                ("group-disabled", false, None, 2),
                ("denied-under-disabled-group", false, None, 3),
                ("group-restored-with-command-denied", false, Some(false), 4),
                ("command-recovered", true, Some(true), 5),
                ("application-disabled", false, Some(false), 7),
                ("application-enabled", true, Some(true), 9),
            ]
        );
    });
}

#[test]
fn detached_split_command_checks_direct_enables_and_recovers_after_denial() {
    crate::group_tests::run(|| {
        let split = RibbonSplitButton::new();
        let command = TraceCommand::new(true);
        split.set_command(Some(adapt_core_command(command.clone())));

        let state = || (split.is_effectively_enabled(), command.calls.get());
        assert!(!split.is_loaded());
        assert_eq!(state(), (true, 1));
        split.set_is_enabled(false);
        assert_eq!(state(), (false, 1));
        split.set_is_enabled(true);
        assert_eq!(state(), (true, 3));

        split.set_is_enabled(false);
        command.can.set(false);
        split.set_is_enabled(true);
        assert_eq!(state(), (false, 4));
        assert!(split.classes().contains(":disabled"));
        command.can.set(true);
        split.set_command_parameter(Some(Rc::new(1i32)));
        assert_eq!(state(), (false, 5));
        split.set_is_enabled(true);
        assert_eq!(state(), (true, 7));
        assert!(!split.classes().contains(":disabled"));

        let changing = RibbonSplitButton::new();
        let command = TraceCommand::new(true);
        let weak = Rc::downgrade(&command);
        command.on_can.replace(Some(Rc::new(move || {
            let command = weak.upgrade().unwrap();
            command.can.set(command.calls.get() != 3);
        })));
        changing.set_command(Some(adapt_core_command(command.clone())));
        changing.set_is_enabled(false);
        changing.set_is_enabled(true);
        assert_eq!(
            (changing.is_effectively_enabled(), command.calls.get()),
            (false, 3)
        );
        changing.set_command_parameter(Some(Rc::new(1i32)));
        assert_eq!(
            (changing.is_effectively_enabled(), command.calls.get()),
            (true, 4)
        );
    });
}

#[test]
fn command_enable_application_override_loaded_unloaded_and_virtual_trace_matches_source() {
    run(|| {
        let p = Probe::new();
        let a = TraceCommand::new(false);
        let b = TraceCommand::new(true);
        b.positive.set(true);
        let mut rows = vec![snapshot(&p, &a, &b, "new")];
        p.set_command(Some(adapt_core_command(a.clone())));
        rows.push(snapshot(&p, &a, &b, "set-before-load"));
        a.can.set(true);
        a.raise();
        rows.push(snapshot(&p, &a, &b, "unloaded-change"));
        let mount = Mounted::new(&p);
        rows.push(snapshot(&p, &a, &b, "loaded"));
        a.can.set(false);
        a.raise();
        rows.push(snapshot(&p, &a, &b, "command-disables"));
        p.set_is_enabled(true);
        rows.push(snapshot(&p, &a, &b, "application-cannot-enable"));
        p.set_is_enabled(false);
        a.can.set(true);
        a.raise();
        rows.push(snapshot(&p, &a, &b, "equal-disabled-write"));
        p.set_is_enabled(false);
        a.can.set(false);
        a.raise();
        a.can.set(true);
        a.raise();
        rows.push(snapshot(&p, &a, &b, "application-disabled"));
        p.set_is_enabled(true);
        a.can.set(false);
        a.raise();
        p.set_command(None);
        rows.push(snapshot(&p, &a, &b, "clear-restores"));
        p.set_command(Some(adapt_core_command(b.clone())));
        rows.push(snapshot(&p, &a, &b, "replace-with-parameter"));
        p.set_command_parameter(Some(Rc::new(2i32)));
        rows.push(snapshot(&p, &a, &b, "parameter-enables"));
        mount.detach();
        assert!(!p.is_loaded());
        rows.push(snapshot(&p, &a, &b, "unloaded"));
        b.can.set(false);
        b.raise();
        rows.push(snapshot(&p, &a, &b, "detached-change"));
        mount.attach(&p);
        rows.push(snapshot(&p, &a, &b, "reload"));
        p.uses.set(false);
        p.update_can_execute();
        rows.push(snapshot(&p, &a, &b, "override-bypasses"));
        p.set_is_enabled(false);
        p.update_can_execute();
        rows.push(snapshot(&p, &a, &b, "override-keeps-app-disabled"));
        p.set_command(None);
        mount.detach();
        rows.push(snapshot(&p, &a, &b, "done"));
        let winui: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/command-base-winui-lifecycle.json"
        ))
        .unwrap();
        assert_eq!(json!(rows), winui["rows"]);
    });
}
#[test]
fn reentrant_command_replacement_preserves_source_stale_decision_then_recovers() {
    run(|| {
        let p = Probe::new();
        let first = TraceCommand::new(false);
        let later = TraceCommand::new(true);
        let weak = p.downgrade();
        let next = later.clone();
        let weak_first = Rc::downgrade(&first);
        *first.on_can.borrow_mut() = Some(Rc::new(move || {
            weak_first.upgrade().unwrap().on_can.replace(None);
            weak.upgrade()
                .unwrap()
                .set_command(Some(adapt_core_command(next.clone())))
        }));
        p.set_command(Some(adapt_core_command(first.clone())));
        let original = source_core_command(&p.command().unwrap()).unwrap();
        let expected: Rc<dyn CoreCommand> = later.clone();
        assert_eq!(
            json!({"enabled":p.is_enabled(),"replaced":Rc::ptr_eq(&original,&expected),"updates":p.updates.get(),"firstCalls":first.calls.get(),"laterCalls":later.calls.get()}),
            corpus()["stale"]
        );
        p.update_can_execute();
        assert_eq!(json!(p.is_enabled()), corpus()["recovered"]);
    });
}
#[test]
fn command_source_interface_helper_and_relay_execute_exactly_as_source() {
    run(|| {
        let p = Probe::new();
        p.set_command_parameter(Some(Rc::new(42i32)));
        let command = TraceCommand::new(true);
        p.set_command(Some(adapt_core_command(command.clone())));
        p.set_is_enabled(false);
        let value: BoxedValue = Rc::new(p.clone());
        let source = ValueTypes::try_convert(
            Some(&value),
            ValueType::of::<Rc<dyn IRibbonCommandSource>>(),
        )
        .flatten()
        .unwrap();
        let source = source
            .downcast_ref::<Rc<dyn IRibbonCommandSource>>()
            .unwrap();
        assert!(source.as_object().ptr_eq(&p));
        let parameter: BoxedValue = Rc::new(7i32);
        let result = RibbonItemHelper::execute(source.as_ref(), Some(&parameter));
        command.can.set(false);
        let denied =
            RibbonItemHelper::execute(source.as_ref(), Some(&(Rc::new(9i32) as BoxedValue)));
        assert_eq!(
            json!({"result":result,"denied":denied,"Executions":command.executions.get(),"parameters":*command.parameters.borrow()}),
            corpus()["helper"]
        );
        let checks = Rc::new(Cell::new(0));
        let runs = Rc::new(Cell::new(0));
        let read = checks.clone();
        let execute = runs.clone();
        let relay = RibbonRelayCommand::new_with_parameter(
            move |_| execute.set(execute.get() + 1),
            Some(Rc::new(move |_| {
                read.set(read.get() + 1);
                true
            })),
        );
        p.set_command(Some(adapt_core_command(relay)));
        checks.set(0);
        assert!(RibbonItemHelper::execute(
            source.as_ref(),
            Some(&(Rc::new(11i32) as BoxedValue))
        ));
        assert_eq!(
            json!({"checks":checks.get(),"runs":runs.get()}),
            corpus()["relayTrace"]
        );
    });
}
#[test]
fn enabled_exception_restores_flag_and_normal_unload_removes_handlers() {
    run(|| {
        let p = Probe::new();
        let once = Rc::new(Cell::new(true));
        let state = once.clone();
        p.add_property_changed_handler(Rc::new(move |e| {
            if e.property() == InputElement::is_enabled_property().as_property()
                && state.replace(false)
            {
                panic!("enabled failure")
            }
        }));
        let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            p.set_command(Some(adapt_core_command(TraceCommand::new(false))))
        }))
        .unwrap_err();
        p.set_is_enabled(true);
        assert_eq!(
            json!({"error":error.downcast_ref::<&str>().unwrap(),"enabled":p.is_enabled()}),
            corpus()["finallyTrace"]
        );
        let p = Probe::new();
        let command = TraceCommand::new(true);
        p.set_command(Some(adapt_core_command(command.clone())));
        let mount = Mounted::new(&p);
        assert_eq!(command.changed.handler_count(), 1);
        mount.detach();
        assert_eq!(command.changed.handler_count(), 0);
        mount.attach(&p);
        assert_eq!(command.changed.handler_count(), 1);
        let weak = p.downgrade();
        mount.detach();
        drop(p);
        assert!(weak.upgrade().is_none());
        assert_eq!(command.changed.handler_count(), 0);
    });
}
#[test]
fn bridge_preserves_live_identity_payloads_events_disposal_and_failures() {
    run(|| {
        let source = TraceCommand::new(true);
        let core: Rc<dyn CoreCommand> = source.clone();
        let native = adapt_core_command(core.clone());
        let second = adapt_core_command(core.clone());
        assert!(Rc::ptr_eq(&native, &second));
        assert!(Rc::ptr_eq(&source_core_command(&native).unwrap(), &core));
        let count = Rc::new(Cell::new(0));
        let calls = count.clone();
        let token = native.can_execute_changed(Rc::new(move || calls.set(calls.get() + 1)));
        source.raise();
        assert_eq!(count.get(), 1);
        token.dispose();
        token.dispose();
        source.raise();
        assert_eq!(count.get(), 1);
        assert_eq!(source.removes.get(), 1);
        #[derive(PartialEq)]
        struct Payload(i32);
        let object = Rc::new(Payload(7));
        ValueTypes::register_reference::<Payload>();
        let handle: BoxedValue = Rc::new(object.clone());
        let parameter = command_parameter(Some(&handle)).unwrap();
        assert!(std::ptr::eq(
            parameter.downcast_ref::<Payload>().unwrap(),
            object.as_ref()
        ));
        let core_object = RibbonObject::reference(object.clone());
        let boxed: BoxedValue = Rc::new(core_object.clone());
        assert!(Rc::ptr_eq(
            &command_parameter(Some(&boxed)).unwrap(),
            &core_object.payload()
        ));
        assert!(command_parameter(Some(&(Rc::new(Option::<i32>::None) as BoxedValue))).is_none());
        let weak_source = Rc::downgrade(&core);
        drop(source);
        drop(core);
        assert!(weak_source.upgrade().is_some());
        drop(native);
        drop(second);
        assert!(weak_source.upgrade().is_none());
        let throwing = RibbonRelayCommand::new(|| panic!("command failure"));
        let native = adapt_core_command(throwing);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| native.execute(None)))
                .is_err()
        );
    });
}

//! WinUI command accessor reentry, early removal, and event snapshots.
use ferroribbon::commands::adapt_core_command;
use ferroribbon::controls::{RibbonControlBase, RibbonControlBaseImpl, RibbonControlBaseImplExt};
use ferroribbon_core::commands::ICommand as CoreCommand;
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_impl_classes,
    instantiate,
};
use ferroui_base::{
    input::{ICommand, InputElementImpl},
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    reactive::{Disposable, IDisposable},
    threading::{CancellationToken, Dispatcher},
};
use ferroui_controls::{
    AppBuilder, Application, Control, ControlImpl, Window, primitives::TemplatedControlImpl,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
};
#[repr(C)]
struct PendingProbe {
    base: RibbonControlBase,
    updates: Cell<i32>,
}
ferro_class!(PendingProbe:RibbonControlBase);
ferro_impl_classes!(PendingProbe:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl PendingProbe {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonControlBase::construct(),
            updates: Cell::new(0),
        })
    }
}
impl RibbonControlBaseImpl for PendingProbe {
    fn update_can_execute(this: &Self) {
        this.updates.set(this.updates.get() + 1);
        Self::parent_update_can_execute(this)
    }
}
struct Command {
    handlers: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    on_add: RefCell<Option<Rc<dyn Fn()>>>,
    raise_after: bool,
    publish_early: bool,
    calls: Cell<i32>,
    removes: Rc<Cell<i32>>,
}
impl Command {
    fn new(raise_after: bool, publish_early: bool) -> Rc<Self> {
        Rc::new(Self {
            handlers: Rc::new(RefCell::new(Vec::new())),
            on_add: RefCell::new(None),
            raise_after,
            publish_early,
            calls: Cell::new(0),
            removes: Rc::new(Cell::new(0)),
        })
    }

    fn insert(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.handlers.borrow_mut().push(handler.clone());
        let handlers = Rc::downgrade(&self.handlers);
        let removes = self.removes.clone();
        Disposable::create(move || {
            removes.set(removes.get() + 1);
            if let Some(handlers) = handlers.upgrade() {
                handlers.borrow_mut().retain(|h| !Rc::ptr_eq(h, &handler))
            }
        })
    }

    fn finish_add(&self) {
        let on_add = self.on_add.borrow().clone();
        if let Some(on_add) = on_add {
            on_add()
        }
        if self.raise_after {
            let snapshot = self.handlers.borrow().clone();
            for callback in snapshot {
                callback()
            }
        }
    }

    fn register(&self, handler: Rc<dyn Fn()>, publish: &mut dyn FnMut(Rc<dyn IDisposable>)) {
        let token = self.insert(handler);
        if self.publish_early {
            publish(token);
            self.finish_add();
        } else {
            self.finish_add();
            publish(token);
        }
    }
}

impl ICommand for Command {
    fn can_execute(&self, _: Option<&ferroui_base::BoxedValue>) -> bool {
        self.calls.set(self.calls.get() + 1);
        true
    }
    fn execute(&self, _: Option<&ferroui_base::BoxedValue>) {
        panic!("fixture command not invoked")
    }
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.insert(handler);
        self.finish_add();
        token
    }
    fn register_can_execute_changed(
        &self,
        handler: Rc<dyn Fn()>,
        publish: &mut dyn FnMut(Rc<dyn IDisposable>),
    ) {
        self.register(handler, publish)
    }
}
impl CoreCommand for Command {
    fn can_execute(&self, _: Option<&dyn Any>) -> bool {
        self.calls.set(self.calls.get() + 1);
        true
    }
    fn execute(&self, _: Option<&dyn Any>) {
        panic!("fixture command not invoked")
    }
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        ICommand::can_execute_changed(self, handler)
    }
    fn register_can_execute_changed(
        &self,
        handler: Rc<dyn Fn()>,
        publish: &mut dyn FnMut(Rc<dyn IDisposable>),
    ) {
        self.register(handler, publish)
    }
}

struct LegacyCommand(Rc<Command>);
impl ICommand for LegacyCommand {
    fn can_execute(&self, parameter: Option<&ferroui_base::BoxedValue>) -> bool {
        ICommand::can_execute(self.0.as_ref(), parameter)
    }
    fn execute(&self, parameter: Option<&ferroui_base::BoxedValue>) {
        ICommand::execute(self.0.as_ref(), parameter)
    }
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        ICommand::can_execute_changed(self.0.as_ref(), handler)
    }
}
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "FerroRibbon.PendingSubscription.Reproducer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                ferroribbon::register_types();
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}

fn mount(probe: &Ref<PendingProbe>) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(probe)));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(probe.is_loaded());
    window
}

fn snapshot(probe: &PendingProbe, old: &Command, next: &Command) -> (i32, i32, usize, usize, i32) {
    (
        probe.updates.get(),
        next.calls.get(),
        old.handlers.borrow().len(),
        next.handlers.borrow().len(),
        old.removes.get(),
    )
}

#[test]
fn custom_event_add_reentry_must_remove_old_handler_before_add_returns() {
    run(|| {
        for core in [false, true] {
            let old = Command::new(true, true);
            let command: Rc<dyn ICommand> = if core {
                adapt_core_command(old.clone())
            } else {
                old.clone()
            };
            check_add_reentry(old, command, true);
        }
    });
}

#[test]
fn legacy_add_reentry_retires_callbacks_until_the_removal_handle_arrives() {
    run(|| {
        let old = Command::new(true, false);
        let command = Rc::new(LegacyCommand(old.clone()));
        check_add_reentry(old, command, false);
    });
}

fn check_add_reentry(old: Rc<Command>, command: Rc<dyn ICommand>, publish_early: bool) {
    let probe = PendingProbe::new();
    let next = Command::new(false, false);
    let captured = Rc::new(RefCell::new(Vec::new()));
    let late_callbacks = captured.clone();
    let weak_probe = probe.downgrade();
    let weak_old = Rc::downgrade(&old);
    let replacement = next.clone();
    old.on_add.replace(Some(Rc::new(move || {
        let probe = weak_probe.upgrade().unwrap();
        let old = weak_old.upgrade().unwrap();
        probe.set_command(Some(replacement.clone()));
        let (old_count, removed) = if publish_early { (0, 1) } else { (1, 0) };
        assert_eq!(
            snapshot(&probe, &old, &replacement),
            (3, 2, old_count, 1, removed)
        );
        late_callbacks.replace(old.handlers.borrow().clone());
    })));
    probe.set_command(Some(command));
    let window = mount(&probe);
    assert_eq!(snapshot(&probe, &old, &next), (4, 3, 0, 1, 1));

    for callback in captured.take() {
        callback();
    }
    assert_eq!(snapshot(&probe, &old, &next), (4, 3, 0, 1, 1));

    window.set_content(None);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(snapshot(&probe, &old, &next), (4, 3, 0, 0, 1));
    assert_eq!(next.removes.get(), 1);
    window.close();
}

#[test]
fn replacing_a_command_preserves_callbacks_already_in_an_event_snapshot() {
    run(|| {
        let probe = PendingProbe::new();
        let old = Command::new(false, false);
        let next = Command::new(false, false);
        probe.set_command(Some(old.clone()));
        let window = mount(&probe);
        let captured = old.handlers.borrow().clone();
        probe.set_command(Some(next.clone()));
        for callback in captured {
            callback();
        }
        assert_eq!(snapshot(&probe, &old, &next), (5, 3, 0, 1, 1));
        window.close();
    });
}

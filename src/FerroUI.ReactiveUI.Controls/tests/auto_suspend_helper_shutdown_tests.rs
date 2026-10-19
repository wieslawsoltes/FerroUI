//! `AutoSuspendHelperShutdownTests` (`AutoSuspendHelperShutdownTests.cs`; the file of the
//! `.Reactive.Tests` project is the same but for its namespace): the shutdown of the application
//! with the suspension helper, and the ownership of the exit event.

use super::view_support::{start, ControlledLifetime};
use crate::AutoSuspendHelper;
use ferroui_controls::application_lifetimes::{ControlledApplicationLifetimeExitEventArgs, IControlledApplicationLifetime};
use ferroui_controls::Design;
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// Sets the design mode for the life of the guard, and restores the previous mode after it.
struct DesignModeScope {
    previous: bool,
}

impl DesignModeScope {
    fn enter(value: bool) -> Self {
        let previous = Design::is_design_mode();
        Design::set_is_design_mode(value);
        Self { previous }
    }
}

impl Drop for DesignModeScope {
    fn drop(&mut self) {
        Design::set_is_design_mode(self.previous);
    }
}

/// Records the exit of `fixture` (`fixture.Lifetime.Exit += (_, _) => exited = true`).
fn on_exit(fixture: &ControlledLifetime, exited: &Rc<Cell<bool>>) -> Rc<dyn IDisposable> {
    let exited = exited.clone();
    fixture.exit(Rc::new(move |_: &ControlledApplicationLifetimeExitEventArgs| exited.set(true)))
}

#[test]
fn design_mode_does_not_persist_state() {
    let _app = start();
    let _design_mode = DesignModeScope::enter(true);
    let fixture = ControlledLifetime::new();
    let helper = AutoSuspendHelper::new(Some(&*fixture));

    fixture.raise_exit();

    assert!(Design::is_design_mode());
    helper.dispose();
}

#[test]
fn shutdown_without_persistence_subscribers_completes() {
    let _app = start();
    let fixture = ControlledLifetime::new();
    let helper = AutoSuspendHelper::new(Some(&*fixture));
    let exited = Rc::new(Cell::new(false));
    let _exit = on_exit(&fixture, &exited);

    fixture.raise_exit();

    assert!(exited.get());
    helper.dispose();
}

#[test]
fn dispose_removes_lifetime_handler() {
    let _app = start();
    let fixture = ControlledLifetime::new();
    let helper = AutoSuspendHelper::new(Some(&*fixture));
    helper.dispose();
    let exited = Rc::new(Cell::new(false));
    let _exit = on_exit(&fixture, &exited);

    fixture.raise_exit();

    assert!(exited.get());
}

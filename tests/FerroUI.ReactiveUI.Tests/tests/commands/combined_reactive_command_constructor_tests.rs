//! `CombinedReactiveCommandConstructorTests` (`ReactiveUI.Tests/Commands/CombinedReactiveCommandConstructorTests.cs`):
//! the constructor with a gate and the one with an output scheduler, which are one constructor
//! with two optional arguments here.

use crate::mocks::*;
use ferroui_reactive::linq::Observable;
use ferroui_reactiveui::*;
use ferroui_system::IDisposable;
use std::rc::Rc;

#[test]
fn can_execute_constructor_builds_command() {
    with_immediate_schedulers();
    let child = ReactiveCommand::create_with(|| {}, None, Some(immediate()));
    let children: Vec<Rc<dyn IReactiveCommandOf<(), ()>>> = vec![Rc::new(child)];
    let can_execute = Observable::return_(true);

    let combined = CombinedReactiveCommand::new(children, Some(can_execute), None);

    assert!(first(&combined.can_execute()));
    combined.dispose();
}

#[test]
fn scheduler_constructor_builds_command() {
    with_immediate_schedulers();
    let child = ReactiveCommand::create_with(|| {}, None, Some(immediate()));
    let children: Vec<Rc<dyn IReactiveCommandOf<(), ()>>> = vec![Rc::new(child)];

    let combined = CombinedReactiveCommand::new(children, None, Some(immediate()));

    assert!(first(&combined.can_execute()));
    combined.dispose();
}

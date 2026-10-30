//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/AddEventHandlerRegistryTests.cs` of
//! Xaml.Behaviors.
//!
//! Adapted: the plain event of the test control is a handler list whose subscription is a
//! disposable, which is what the remove function of the handler takes.

use super::support::{boxed, start_app};
use crate::{AddEventHandlerRegistry, FuncAddEventHandler, IAddEventHandler};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::{EventArgs, HandlerList};
use ferroui_base::BoxedValue;
use ferroui_controls::Button;
use std::cell::Cell;
use std::rc::Rc;

/// A plain object with a plain event.
struct TestControl {
    custom_event: HandlerList<dyn Fn(Option<&BoxedValue>, &EventArgs)>,
}

impl PartialEq for TestControl {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl TestControl {
    fn new() -> Rc<Self> {
        Rc::new(Self { custom_event: HandlerList::new() })
    }

    /// `CustomEvent += handler`, with the subscription as a disposable.
    fn add_custom_event(this: &Rc<Self>, handler: Rc<dyn Fn(Option<&BoxedValue>, &EventArgs)>) -> Rc<dyn IDisposable> {
        let token = this.custom_event.add(handler);
        let weak = Rc::downgrade(this);
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.custom_event.remove(token);
            }
        })
    }

    fn raise(this: &Rc<Self>) {
        let sender: BoxedValue = Rc::new(this.clone());
        for (_, handler) in this.custom_event.snapshot().iter() {
            handler(Some(&sender), &EventArgs::EMPTY);
        }
    }
}

#[test]
fn try_register_event_handler_button_click_returns_disposable() {
    let _app = start_app();
    let button = Button::new();
    let called = Rc::new(Cell::new(false));

    let c = called.clone();
    let disposable =
        AddEventHandlerRegistry::try_register_event_handler(&boxed(&button), "Click", Rc::new(move |_: Option<&BoxedValue>, _: &BoxedValue| c.set(true)));

    assert!(disposable.is_some());

    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert!(called.get());

    called.set(false);
    disposable.unwrap().dispose();
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert!(!called.get());
}

#[test]
fn try_register_event_handler_no_match_returns_null() {
    let _app = start_app();
    let button = Button::new();
    let disposable = AddEventHandlerRegistry::try_register_event_handler(&boxed(&button), "NoEvent", Rc::new(|_: Option<&BoxedValue>, _: &BoxedValue| {}));
    assert!(disposable.is_none());
}

#[test]
fn register_unregister_custom_handler() {
    let _app = start_app();
    let control = TestControl::new();
    let called = Rc::new(Cell::new(false));

    let handler: Rc<dyn IAddEventHandler> = Rc::new(FuncAddEventHandler::<Rc<TestControl>, EventArgs>::new(
        "CustomEvent",
        |o, h| TestControl::add_custom_event(o, Rc::new(move |sender: Option<&BoxedValue>, e: &EventArgs| h(sender, e))),
        |_, subscription| subscription.dispose(),
    ));

    let source: BoxedValue = Rc::new(control.clone());

    AddEventHandlerRegistry::register(handler.clone());
    {
        let c = called.clone();
        let disposable =
            AddEventHandlerRegistry::try_register_event_handler(&source, "CustomEvent", Rc::new(move |_: Option<&BoxedValue>, _: &BoxedValue| c.set(true)));
        assert!(disposable.is_some());

        TestControl::raise(&control);
        assert!(called.get());

        disposable.unwrap().dispose();
        called.set(false);
        TestControl::raise(&control);
        assert!(!called.get());
    }
    AddEventHandlerRegistry::unregister(&handler);

    let disposable2 = AddEventHandlerRegistry::try_register_event_handler(&source, "CustomEvent", Rc::new(|_: Option<&BoxedValue>, _: &BoxedValue| {}));
    assert!(disposable2.is_none());
}

//! Port of `Events/AddEventHandlerRegistry.cs`.

use super::handlers::{FlyoutEventHandler, FuncAddEventHandler};
use super::{AddEventHandlerCallback, IAddEventHandler};
use ferroui_base::interactivity::{CancelRoutedEventArgs, RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Button, Control, MenuItem, ToolTip};
use std::cell::RefCell;
use std::rc::Rc;

/// A registry for adding event handlers to various controls: the events an event trigger
/// subscribes by name before it looks for the event in the metadata of the source.
///
/// The registry belongs to the thread (handlers hold objects of the thread).
pub struct AddEventHandlerRegistry;

ferroui_base::ferro_thread_local! {
    static ADD_EVENT_HANDLERS: RefCell<Vec<Rc<dyn IAddEventHandler>>> = RefCell::new(AddEventHandlerRegistry::default_handlers());
}

impl AddEventHandlerRegistry {
    /// The handlers the static constructor of the reference registers.
    fn default_handlers() -> Vec<Rc<dyn IAddEventHandler>> {
        let mut handlers: Vec<Rc<dyn IAddEventHandler>> = Vec::new();

        // handlers.push(Rc::new(ButtonClickEventHandler));

        handlers.push(Rc::new(FuncAddEventHandler::<Ref<Button>, RoutedEventArgs, RoutedEventHandlerToken>::new(
            "Click",
            |o, h| o.click(move |sender, e| h(Some(&(Rc::new(sender.to_ref()) as BoxedValue)), e)),
            |o, h| {
                o.remove_handler(Button::click_event(), h);
            },
        )));

        handlers.push(Rc::new(FuncAddEventHandler::<Ref<MenuItem>, RoutedEventArgs, RoutedEventHandlerToken>::new(
            "Click",
            |o, h| o.click(move |sender, e| h(Some(&(Rc::new(sender.to_ref()) as BoxedValue)), e)),
            |o, h| {
                o.remove_handler(MenuItem::click_event(), h);
            },
        )));

        handlers.push(Rc::new(FlyoutEventHandler));

        handlers.push(Rc::new(FuncAddEventHandler::<Ref<Control>, CancelRoutedEventArgs, RoutedEventHandlerToken>::new(
            "ToolTipOpening",
            |o, h| ToolTip::add_tool_tip_opening_handler(o, move |sender, e| h(Some(&(Rc::new(sender.to_ref()) as BoxedValue)), e)),
            |o, h| ToolTip::remove_tool_tip_opening_handler(o, h),
        )));

        handlers.push(Rc::new(FuncAddEventHandler::<Ref<Control>, RoutedEventArgs, RoutedEventHandlerToken>::new(
            "ToolTipClosing",
            |o, h| ToolTip::add_tool_tip_closing_handler(o, move |sender, e| h(Some(&(Rc::new(sender.to_ref()) as BoxedValue)), e)),
            |o, h| ToolTip::remove_tool_tip_closing_handler(o, h),
        )));

        handlers
    }

    /// Registers an event handler.
    pub fn register(add_event_handler: Rc<dyn IAddEventHandler>) {
        ADD_EVENT_HANDLERS.with(|handlers| {
            let mut handlers = handlers.borrow_mut();
            if !handlers.iter().any(|handler| Rc::ptr_eq(handler, &add_event_handler)) {
                handlers.push(add_event_handler);
            }
        });
    }

    /// Unregisters an event handler.
    pub fn unregister(add_event_handler: &Rc<dyn IAddEventHandler>) {
        ADD_EVENT_HANDLERS.with(|handlers| {
            handlers.borrow_mut().retain(|handler| !Rc::ptr_eq(handler, add_event_handler));
        });
    }

    /// Subscribes `handler` to the event `event_name` of `source` through the first registered
    /// handler that matches them; `None` when none does.
    pub fn try_register_event_handler(
        source: &BoxedValue,
        event_name: &str,
        handler: AddEventHandlerCallback,
    ) -> Option<Rc<dyn IDisposable>> {
        let add_event_handler = ADD_EVENT_HANDLERS.with(|handlers| {
            handlers.borrow().iter().find(|handler| handler.matches(source, event_name)).cloned()
        });

        add_event_handler?.add_handler(source, event_name, handler)
    }
}

//! Focused selected-framework deficiency: a static event has no instance.
//! Regression for the explicit receiver-free static-event declaration.
use ferroui_base::{
    BoxedValue, ferro_markup_type, ferro_static_type,
    metadata::{MarkupDelegate, MarkupTyped, into_markup_value},
    utilities::EventArgs,
};
use std::{cell::Cell, rc::Rc};
pub struct StaticEvents;
ferro_static_type!(StaticEvents);
ferroui_base::ferro_thread_local! {static SUBSCRIBED:Cell<usize>=const{Cell::new(0)};}
ferro_markup_type!(static StaticEvents {
    namespace:"FerroRibbon.FrameworkRepro",type_info:StaticEvents,
    events:[static Changed(Option<BoxedValue>,EventArgs)=>|handler:MarkupDelegate|{
        SUBSCRIBED.set(SUBSCRIBED.get()+1);handler.invoke(&[None,into_markup_value(EventArgs::EMPTY)]);
    }],
});
#[test]
fn static_event_subscription_requires_no_constructible_instance() {
    assert!(StaticEvents::TYPE.default_constructor().is_none());
    let called = Rc::new(Cell::new(0));
    let called2 = called.clone();
    let handler = MarkupDelegate::new(move |args| {
        assert!(args[0].is_none());
        called2.set(called2.get() + 1);
        None
    });
    let event = StaticEvents::MARKUP.find_event("Changed").unwrap();
    // A static event takes only the delegate; no source-class instance exists.
    assert!(event.is_static);
    let result = (event.add)(&[into_markup_value(handler)]);
    assert!(
        result.is_ok(),
        "static event should accept no instance: {result:?}"
    );
    assert_eq!(SUBSCRIBED.get(), 1);
    assert_eq!(called.get(), 1);
}

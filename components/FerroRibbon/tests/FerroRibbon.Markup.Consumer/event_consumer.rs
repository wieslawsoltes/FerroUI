//! Independent compiled and runtime checks of static-event metadata.
use ferroui_base::{
    BoxedValue, input::InputElementImpl, interactivity::InteractiveImpl, layout::LayoutableImpl,
    metadata::MarkupDelegate, utilities::EventArgs,
};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, instantiate,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::{Cell, RefCell};
ferroui_base::ferro_thread_local! {
    static HANDLERS:RefCell<Vec<MarkupDelegate>>=const{RefCell::new(Vec::new())};
    static SIGNAL_HANDLERS:RefCell<Vec<MarkupDelegate>>=const{RefCell::new(Vec::new())};
    static CALLS:Cell<usize>=const{Cell::new(0)};
}
#[repr(C)]
pub struct StaticEventConsumer {
    base: Control,
}
ferro_class!(StaticEventConsumer:Control);
ferro_impl_classes!(StaticEventConsumer:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_class_info!(StaticEventConsumer {new:StaticEventConsumer::new,markup:{namespace:"FerroRibbon.Consumer",
    methods:[fn OnPing()=>StaticEventConsumer::on_ping,
        fn OnChanged(Option<BoxedValue>,EventArgs)=>StaticEventConsumer::on_changed,
        fn OnValue(i32)->i32=>StaticEventConsumer::on_value,
        try fn OnFail(Option<BoxedValue>,EventArgs)=>StaticEventConsumer::on_fail],
    events:[static Ping() {add:StaticEventConsumer::subscribe_optional,remove:StaticEventConsumer::unsubscribe},
        static Signal(Option<BoxedValue>,EventArgs) {add:StaticEventConsumer::subscribe_signal,remove:StaticEventConsumer::unsubscribe_signal}],
}});
impl StaticEventConsumer {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
        })
    }
    pub fn subscribe_optional(handler: Option<MarkupDelegate>) {
        if let Some(handler) = handler {
            Self::subscribe(handler);
        }
    }
    pub fn unsubscribe(handler: Option<MarkupDelegate>) {
        if let Some(handler) = handler {
            let old = HANDLERS.with_borrow_mut(|h| {
                h.iter()
                    .rposition(|item| item == &handler)
                    .map(|i| h.remove(i))
            });
            drop(old);
        }
    }
    pub fn subscribe(handler: MarkupDelegate) {
        HANDLERS.with_borrow_mut(|h| h.push(handler));
    }
    pub fn on_ping(&self) {
        CALLS.set(CALLS.get() + 1);
    }
    pub fn on_changed(&self, sender: Option<BoxedValue>, args: EventArgs) {
        assert!(sender.is_none());
        assert_eq!(args, EventArgs::EMPTY);
        CALLS.set(CALLS.get() + 1);
    }
    pub fn on_fail(&self, _: Option<BoxedValue>, _: EventArgs) -> Result<(), String> {
        CALLS.set(CALLS.get() + 1);
        Err("handler-failed".into())
    }
    pub fn on_value(&self, value: i32) -> i32 {
        CALLS.set(CALLS.get() + 1);
        value + 1
    }
    pub fn subscribe_signal(handler: Option<MarkupDelegate>) {
        if let Some(handler) = handler {
            SIGNAL_HANDLERS.with_borrow_mut(|h| h.push(handler));
        }
    }
    pub fn unsubscribe_signal(handler: Option<MarkupDelegate>) {
        if let Some(handler) = handler {
            let old = SIGNAL_HANDLERS.with_borrow_mut(|h| {
                h.iter()
                    .rposition(|item| item == &handler)
                    .map(|i| h.remove(i))
            });
            drop(old);
        }
    }
    pub fn try_raise_signal() -> Result<(), ferroui_base::metadata::MarkupInvokeError> {
        for handler in SIGNAL_HANDLERS.with_borrow(Clone::clone) {
            handler.try_invoke(&[
                None,
                ferroui_base::metadata::into_markup_value(EventArgs::EMPTY),
            ])?;
        }
        Ok(())
    }
    pub fn raise_signal() {
        Self::try_raise_signal().expect("test handler");
    }
    pub fn reset() {
        HANDLERS.with_borrow_mut(Vec::clear);
        SIGNAL_HANDLERS.with_borrow_mut(Vec::clear);
        CALLS.set(0);
    }
    pub fn raise() {
        let handlers = HANDLERS.with_borrow(Clone::clone);
        for handler in handlers {
            handler.try_invoke(&[]).expect("test handler");
        }
    }
    pub fn count() -> usize {
        CALLS.get()
    }
}
pub fn register() {
    ferroui_base::TypeInfo::register_namespaces(&[(
        "ferroribbon_markup_consumer::event_consumer",
        "FerroRibbon.Consumer",
    )]);
    ferroui_base::TypeInfo::register_all(&[StaticEventConsumer::TYPE]);
    ferroui_base::metadata::MarkupType::register(StaticEventConsumer::TYPE.markup().unwrap());
}

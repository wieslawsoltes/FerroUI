//! One cached native EventArgs.Empty allocation for untyped library subscribers.
//! The framework transports typed EventArgs by value; native UI-thread affinity
//! differs from CLR process statics and remains documented separately.
use ferroui_base::{BoxedValue, utilities::EventArgs};
use std::rc::Rc;
ferroui_system::ferro_thread_local! {static EMPTY_ARGS:BoxedValue=Rc::new(EventArgs::EMPTY);}
pub(crate) fn empty_event_args() -> BoxedValue {
    EMPTY_ARGS.with(Clone::clone)
}

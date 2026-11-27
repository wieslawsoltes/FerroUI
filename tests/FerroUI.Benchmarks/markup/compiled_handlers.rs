//! Calling a handler that compiled markup attached (#998): the delegate of a method named in a
//! document, as generated code builds it with `rt::method_delegate`, called alone and through a
//! routed event raised repeatedly, as pointer moves over an element raise theirs.

use std::cell::Cell;
use std::hint::black_box;
use std::rc::Rc;

use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEvent, RoutedEventArgs, RoutingStrategies};
use ferroui_base::metadata::{MarkupDelegate, MarkupValue};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::Border;
use ferroui_markup_xaml::xaml_il::runtime::compiled as rt;
use ferroui_markup_xaml::xaml_il::runtime::{XamlIlContext, XamlIlRuntimeHelpers};

use crate::harness::Registry;

/// The events a repeated invocation raises.
const MOVES: u32 = 64;

static DOCUMENT: rt::DocumentInfo = rt::DocumentInfo { base_uri: None, namespaces: &[] };

ferroui_base::ferro_thread_local! {
    static CALLS: Cell<u64> = const { Cell::new(0) };
    /// The event of the benchmark, registered once.
    static MOVED: &'static RoutedEvent<RoutedEventArgs> =
        RoutedEvent::register::<Border, RoutedEventArgs>("CompiledHandlerMoved", RoutingStrategies::BUBBLE);
}

/// A method of the code-behind, with the parameters of an event handler in a document.
#[inline(never)]
fn on_moved(this: &Ref<Border>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>) {
    black_box((this, &sender, &args));
    CALLS.with(|calls| calls.set(calls.get() + 1));
}

/// The delegate as the emitter writes it for a method with two parameters.
fn delegate(context: &Rc<XamlIlContext>) -> MarkupDelegate {
    rt::method_delegate(context, 2, |this, arguments| {
        on_moved(this, arguments.next()?, arguments.next()?);
        Ok(None)
    })
}

pub struct CompiledHandlers {
    /// The owner of the handler: the delegate holds it weakly.
    _root: Ref<Border>,
    target: Ref<Interactive>,
    delegate: MarkupDelegate,
    arguments: [MarkupValue; 2],
    event_args: RoutedEventArgs,
}

impl CompiledHandlers {
    pub fn new() -> Self {
        let event = MOVED.with(|event| *event);
        let root = Border::new();
        let service_provider = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
        let context = rt::populate_context(Some(service_provider), &DOCUMENT, rt::to_value(root.clone()));
        rt::add_handler_with_context(&context, &root, event, delegate(&context));

        let target: Ref<Interactive> = root.clone().upcast();
        let event_args = RoutedEventArgs::with_event(event);
        // What the adapter of an untyped handler passes (`Interactive::add_handler_untyped`).
        let sender: BoxedValue = Rc::new(target.clone());
        let args: BoxedValue = Rc::new(event_args.share());
        Self { delegate: delegate(&context), _root: root, target, arguments: [Some(sender), Some(args)], event_args }
    }

    /// One call of the delegate.
    pub fn invoke(&self) -> MarkupValue {
        self.delegate.invoke(&self.arguments)
    }

    /// A routed event raised [`MOVES`] times on the element whose handler markup attached.
    pub fn raise_repeatedly(&self) {
        for _ in 0..MOVES {
            self.target.raise_event(&self.event_args);
        }
    }
}

impl Default for CompiledHandlers {
    fn default() -> Self {
        Self::new()
    }
}

pub fn register(registry: &mut Registry) {
    let mut class = registry.class("markup", "CompiledHandlers");
    class.benchmark("invoke", "", CompiledHandlers::new, |b| b.invoke());
    class
        .benchmark("raise_repeatedly", "", CompiledHandlers::new, |b| b.raise_repeatedly())
        .operations_per_invoke(MOVES);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiled_handlers() {
        crate::harness::smoke_class(super::register, "CompiledHandlers");
    }

    #[test]
    fn every_call_reaches_the_method() {
        let handlers = CompiledHandlers::new();
        let before = CALLS.with(Cell::get);
        handlers.invoke();
        assert_eq!(CALLS.with(Cell::get), before + 1);
        handlers.raise_repeatedly();
        assert_eq!(CALLS.with(Cell::get), before + 1 + u64::from(MOVES));
    }
}

//! Consumer-only event owner using the actual source factory and event arguments.
use ferrodock_model::core::events::DockableClosingEventArgs;
use ferrodock_model_reactiveui_ferro::reactive_metadata::FactoryReference;
use ferroui_base::BoxedValue;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
/// Plain application event owner for the compiler boundary.
pub struct FactoryEventConsumer {
    factory: RefCell<Option<FactoryReference>>,
    calls: Cell<usize>,
    sender: RefCell<Option<BoxedValue>>,
}
impl FactoryEventConsumer {
    /// Constructs the event owner without creating a second engine.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            factory: RefCell::new(None),
            calls: Cell::new(0),
            sender: RefCell::new(None),
        })
    }
    /// Gets the source factory assigned by compiled markup.
    pub fn factory(&self) -> Option<FactoryReference> {
        self.factory.borrow().clone()
    }
    /// Receives the source factory assigned by compiled markup.
    pub fn set_factory(&self, value: Option<FactoryReference>) {
        self.factory.replace(value);
    }
    /// Cancels the actual source close event, recording its enclosing sender.
    pub fn on_closing(&self, sender: Option<BoxedValue>, args: DockableClosingEventArgs) {
        self.calls.set(self.calls.get() + 1);
        self.sender.replace(sender);
        args.set_cancel(true);
    }
    /// Releases the test owner's retained factory/sender after assertions.
    #[cfg(test)]
    pub(super) fn release_references(&self) {
        self.factory.borrow_mut().take();
        self.sender.borrow_mut().take();
    }
    /// Gets the number of source events received.
    pub fn calls(&self) -> usize {
        self.calls.get()
    }
    /// Gets the original source sender received by the handler.
    pub fn sender(&self) -> Option<BoxedValue> {
        self.sender.borrow().clone()
    }
}
impl PartialEq for FactoryEventConsumer {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class FactoryEventConsumer {
 namespace:"FerroDock.Markup.Tests",this:Rc<FactoryEventConsumer>,handles:[Rc<FactoryEventConsumer>],constructors:[()=>FactoryEventConsumer::new],
 properties:[Factory:Option<FactoryReference>{get:|this:&Rc<FactoryEventConsumer>|this.factory(),set:|this:&Rc<FactoryEventConsumer>,value|this.set_factory(value)}],
 methods:[fn OnClosing(Option<BoxedValue>,DockableClosingEventArgs)=>(|this:&Rc<FactoryEventConsumer>,sender,args|this.on_closing(sender,args))],
});

ferroui_base::ferro_rust_paths! {classes:[],types:[crate::factory_event_consumer::FactoryEventConsumer],contracts:[],generics:[],generic_contracts:[]}
/// Logical assembly identity of the consumer application.
pub static ASSEMBLY: ferroui_base::metadata::MarkupAssembly =
    ferroui_base::metadata::MarkupAssembly {
        name: "ferrodock_markup_tests",
        crate_name: "ferrodock_markup_tests",
        xmlns_definitions: &[],
        xmlns_prefixes: &[],
        metadata: &[],
    };
/// Registers the consumer's own application metadata in its composition root.
pub fn register_types() {
    ferroui_base::metadata::MarkupAssembly::register(&ASSEMBLY);
    ferroui_base::metadata::MarkupType::register(
        <FactoryEventConsumer as ferroui_base::metadata::MarkupTyped>::MARKUP,
    );
    ferroui_base::data::core::ValueTypes::register_reference::<FactoryEventConsumer>();
    register_rust_paths();
}

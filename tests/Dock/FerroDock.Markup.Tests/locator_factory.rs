//! A consumer Factory subclass built with the original engine and normal source dispatch.
use ferrodock_model::controls::IRootDock;
use ferrodock_model::{
    core::{IDockable, IFactory, IHostWindow},
    FactoryBase, FactoryBaseImpl,
};
use ferrodock_model_ferro::{
    DockablePredicateReference, DocumentFactoryReference, FactoryBaseReference,
    RootPredicateReference,
};
use ferrodock_model_reactiveui::{impl_factory, Factory, FactoryImpl};
use ferrodock_model_reactiveui_ferro::reactive_metadata::FactoryReference;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupAssembly, MarkupType, MarkupTyped},
    BoxedValue,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
/// Consumer-only source subclass used by both compiler hosts.
pub struct LocatorFactory {
    component: Factory,
    context: BoxedValue,
    host: Rc<dyn IHostWindow>,
    document: Rc<dyn IDockable>,
    document_factory: RefCell<Option<DocumentFactoryReference>>,
    context_calls: Cell<i32>,
    host_calls: Cell<i32>,
    dockable_calls: Cell<i32>,
    search_predicate: RefCell<Option<DockablePredicateReference>>,
    root_predicate: RefCell<Option<RootPredicateReference>>,
    predicate_calls: Cell<i32>,
}
impl LocatorFactory {
    /// Constructs the actual enclosing factory, with one original engine component.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self {
            component: Factory::construct(runtime.clone()),
            context: Rc::new("Located context".to_owned()),
            host: ferrodock_ferro::controls::HostWindow::as_host_window(
                ferrodock_ferro::controls::HostWindow::new(),
            ),
            document: ferrodock_model_reactiveui::controls::Document::new(),
            document_factory: RefCell::new(None),
            context_calls: Cell::new(0),
            host_calls: Cell::new(0),
            dockable_calls: Cell::new(0),
            search_predicate: RefCell::new(None),
            root_predicate: RefCell::new(None),
            predicate_calls: Cell::new(0),
        })
    }
    /// Gets the native context instance returned by the source locator.
    pub fn context(&self) -> BoxedValue {
        self.context.clone()
    }
    /// Gets the same original host returned by the source locator.
    pub fn host(&self) -> Rc<dyn IHostWindow> {
        self.host.clone()
    }
    /// Gets the same source document returned by the dockable locator.
    pub fn document(&self) -> Rc<dyn IDockable> {
        self.document.clone()
    }
    /// Implements the application's context callback.
    pub fn create_context(&self) -> Option<BoxedValue> {
        self.context_calls.set(self.context_calls.get() + 1);
        Some(self.context())
    }
    /// Implements the application's host callback.
    pub fn create_host(&self) -> Option<Rc<dyn IHostWindow>> {
        self.host_calls.set(self.host_calls.get() + 1);
        Some(self.host())
    }
    /// Implements the application's dockable callback.
    pub fn create_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.dockable_calls.set(self.dockable_calls.get() + 1);
        Some(self.document())
    }
    /// Gets the application's original document factory delegate view.
    pub fn document_factory(&self) -> Option<DocumentFactoryReference> {
        self.document_factory.borrow().clone()
    }
    /// Assigns the application callback without invoking it.
    pub fn set_document_factory(&self, value: Option<DocumentFactoryReference>) {
        *self.document_factory.borrow_mut() = value;
    }
    /// Returns the same original document for the nonnullable document callback.
    pub fn create_document_value(&self) -> Rc<dyn IDockable> {
        self.dockable_calls.set(self.dockable_calls.get() + 1);
        self.document()
    }
    /// Gets calls made to each callback, for deferred-invocation assertions.
    pub fn calls(&self) -> (i32, i32, i32) {
        (
            self.context_calls.get(),
            self.host_calls.get(),
            self.dockable_calls.get(),
        )
    }
    /// Gets the predicate loaded from the application's original method declaration.
    pub fn search_predicate(&self) -> Option<DockablePredicateReference> {
        self.search_predicate.borrow().clone()
    }
    /// Assigns the application's search predicate.
    pub fn set_search_predicate(&self, value: Option<DockablePredicateReference>) {
        *self.search_predicate.borrow_mut() = value;
    }
    /// Gets the application's root predicate.
    pub fn root_predicate(&self) -> Option<RootPredicateReference> {
        self.root_predicate.borrow().clone()
    }
    /// Assigns the application's root predicate.
    pub fn set_root_predicate(&self, value: Option<RootPredicateReference>) {
        *self.root_predicate.borrow_mut() = value;
    }
    /// Matches the original document by its ID.
    pub fn accept_dockable(&self, value: Rc<dyn IDockable>) -> bool {
        self.predicate_calls.set(self.predicate_calls.get() + 1);
        value.id() == "item"
    }
    /// Matches the original root by its ID.
    pub fn accept_root(&self, value: Rc<dyn IRootDock>) -> bool {
        self.predicate_calls.set(self.predicate_calls.get() + 1);
        value.id() == "root"
    }
    /// Gets the number of actual predicate invocations.
    pub fn predicate_calls(&self) -> i32 {
        self.predicate_calls.get()
    }
    /// Clears explicitly owned callbacks when the test fixture is finished.
    pub fn release_callbacks(&self) {
        self.set_document_factory(None);
        self.set_search_predicate(None);
        self.set_root_predicate(None);
        self.set_default_context_locator(None);
        self.set_default_host_window_locator(None);
        self.set_context_locator(None);
        self.set_host_window_locator(None);
        self.set_dockable_locator(None);
    }
}
impl FactoryBaseImpl for LocatorFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
}
impl FactoryImpl for LocatorFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
}
impl_factory!(LocatorFactory);
ferroui_base::ferro_markup_type!(class LocatorFactory {
 namespace:"FerroDock.Markup.Tests",this:Rc<LocatorFactory>,handles:[Rc<LocatorFactory>],base:FactoryReference,constructors:[()=>LocatorFactory::new],
 properties:[DocumentFactory:Option<DocumentFactoryReference>{get:|this:&Rc<LocatorFactory>|this.document_factory(),set:|this:&Rc<LocatorFactory>,value|this.set_document_factory(value)},SearchPredicate:Option<DockablePredicateReference>{get:|this:&Rc<LocatorFactory>|this.search_predicate(),set:|this:&Rc<LocatorFactory>,value|this.set_search_predicate(value)},RootPredicate:Option<RootPredicateReference>{get:|this:&Rc<LocatorFactory>|this.root_predicate(),set:|this:&Rc<LocatorFactory>,value|this.set_root_predicate(value)}],
 methods:[fn CreateDocumentValue()->Rc<dyn IDockable> =>(|this:&Rc<LocatorFactory>|this.create_document_value()),fn AcceptDockable(Rc<dyn IDockable>)->bool =>(|this:&Rc<LocatorFactory>,value|this.accept_dockable(value)),fn AcceptRoot(Rc<dyn IRootDock>)->bool =>(|this:&Rc<LocatorFactory>,value|this.accept_root(value)),fn CreateContext()->Option<BoxedValue> =>(|this:&Rc<LocatorFactory>|this.create_context()),fn CreateHost()->Option<Rc<dyn IHostWindow>> =>(|this:&Rc<LocatorFactory>|this.create_host()),fn CreateDockable()->Option<Rc<dyn IDockable>> =>(|this:&Rc<LocatorFactory>|this.create_dockable())],
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::locator_factory::LocatorFactory],contracts:[],generics:[],generic_contracts:[]}
/// The consumer assembly retains its existing resource identity.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "ferrodock_markup_tests",
    crate_name: "ferrodock_markup_tests",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};
/// Registers the application's ordinary class metadata and actual original base views.
pub fn register_types() {
    MarkupAssembly::register(&ASSEMBLY);
    MarkupType::register(LocatorFactory::MARKUP);
    register_rust_paths();
    ValueTypes::register_reference::<LocatorFactory>();
    ValueTypes::register_cast::<Rc<LocatorFactory>, FactoryReference>(|value| {
        FactoryReference::new(value.clone())
    });
    ValueTypes::register_cast::<Rc<LocatorFactory>, FactoryBaseReference>(|value| {
        FactoryBaseReference::new(value.clone())
    });
    ValueTypes::register_cast::<Rc<LocatorFactory>, Rc<dyn IFactory>>(|value| value.clone());
}

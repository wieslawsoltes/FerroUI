//! Original nullable argument and virtual dispatch behavior through native consumers.
use crate::markup_extension_tests::run;
use ferrodock_model::{controls::*, core::*};
use ferrodock_model_ferro::DocumentFactoryReference;
use ferrodock_model_reactiveui::{controls::DocumentDock, core::DockableBaseImpl};
use ferrodock_model_reactiveui_ferro::reactive_metadata::DocumentDockReference;
use ferroui_base::{metadata::MarkupTyped, threading::Dispatcher, BoxedValue, Ref};
use ferroui_controls::{Button, StackPanel};
use ferroui_reactiveui::{reactive_object, ICommand};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
struct NullDocumentOverride {
    base: DocumentDock,
    calls: RefCell<Vec<&'static str>>,
}
reactive_object!(NullDocumentOverride, base);
impl DockableBaseImpl for NullDocumentOverride {}
ferrodock_model_reactiveui::impl_dock!(@base NullDocumentOverride, DocumentDock, fn as_document_dock(self: Rc<Self>) -> Option<Rc<dyn IDocumentDock>> { Some(self) }, fn as_document_dock_factory(self: Rc<Self>) -> Option<Rc<dyn IDocumentDockFactory>> { Some(self) }
);
impl ILocalTarget for NullDocumentOverride {}
impl IDocumentDock for NullDocumentOverride {
    fn can_create_document(&self) -> bool {
        self.base.can_create_document()
    }
    fn set_can_create_document(&self, value: bool) {
        self.base.set_can_create_document(value);
    }
    fn create_document(&self) -> Option<Rc<dyn ICommand>> {
        self.base.create_document()
    }
    fn set_create_document(&self, value: Option<Rc<dyn ICommand>>) {
        self.base.set_create_document(value);
    }
    fn enable_window_drag(&self) -> bool {
        self.base.enable_window_drag()
    }
    fn set_enable_window_drag(&self, value: bool) {
        self.base.set_enable_window_drag(value);
    }
    fn layout_mode(&self) -> DocumentLayoutMode {
        self.base.layout_mode()
    }
    fn set_layout_mode(&self, value: DocumentLayoutMode) {
        self.base.set_layout_mode(value);
    }
    fn tabs_layout(&self) -> DocumentTabLayout {
        self.base.tabs_layout()
    }
    fn set_tabs_layout(&self, value: DocumentTabLayout) {
        self.base.set_tabs_layout(value);
    }
    fn close_button_show_mode(&self) -> DocumentCloseButtonShowMode {
        self.base.close_button_show_mode()
    }
    fn set_close_button_show_mode(&self, value: DocumentCloseButtonShowMode) {
        self.base.set_close_button_show_mode(value);
    }
    fn empty_content(&self) -> Option<ferrodock_model::BoxedValue> {
        self.base.empty_content()
    }
    fn set_empty_content(&self, value: Option<ferrodock_model::BoxedValue>) {
        self.base.set_empty_content(value);
    }
    fn cascade_documents(&self) -> Option<Rc<dyn ICommand>> {
        self.base.cascade_documents()
    }
    fn set_cascade_documents(&self, value: Option<Rc<dyn ICommand>>) {
        self.base.set_cascade_documents(value);
    }
    fn tile_documents_horizontal(&self) -> Option<Rc<dyn ICommand>> {
        self.base.tile_documents_horizontal()
    }
    fn set_tile_documents_horizontal(&self, value: Option<Rc<dyn ICommand>>) {
        self.base.set_tile_documents_horizontal(value);
    }
    fn tile_documents_vertical(&self) -> Option<Rc<dyn ICommand>> {
        self.base.tile_documents_vertical()
    }
    fn set_tile_documents_vertical(&self, value: Option<Rc<dyn ICommand>>) {
        self.base.set_tile_documents_vertical(value);
    }
    fn restore_documents(&self) -> Option<Rc<dyn ICommand>> {
        self.base.restore_documents()
    }
    fn set_restore_documents(&self, value: Option<Rc<dyn ICommand>>) {
        self.base.set_restore_documents(value);
    }
    fn add_document(&self, document: Option<Rc<dyn IDockable>>) {
        self.calls.borrow_mut().push(if document.is_none() {
            "add_document_null"
        } else {
            "add_document"
        });
    }
    fn add_tool(&self, tool: Option<Rc<dyn IDockable>>) {
        self.calls.borrow_mut().push(if tool.is_none() {
            "add_tool_null"
        } else {
            "add_tool"
        });
    }
}
impl IDocumentDockFactory for NullDocumentOverride {
    fn document_factory(&self) -> Option<DocumentFactory> {
        self.base.document_factory()
    }
    fn set_document_factory(&self, value: Option<DocumentFactory>) {
        self.base.set_document_factory(value);
    }
}
impl NullDocumentOverride {
    fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(|weak: &Weak<Self>| Self {
            base: DocumentDock::construct(weak),
            calls: RefCell::new(Vec::new()),
        });
        model.base.initialize(&model);
        model
    }
}

fn check(panel: &Ref<StackPanel>) {
    let source = DocumentDockReference::create();
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    let callback = DocumentFactoryReference::new(Rc::new(move || {
        seen.set(seen.get() + 1);
        None
    }));
    let value: BoxedValue = Rc::new(source.clone());
    let p = DocumentDockReference::MARKUP
        .find_property("DocumentFactory")
        .unwrap();
    (p.set.unwrap())(&[Some(value), Some(Rc::new(callback))]).unwrap();
    panel.set_data_context(Some(Rc::new(source.model())));
    Dispatcher::ui_thread().run_jobs(None);
    for i in 0..3 {
        panel
            .children()
            .get(i)
            .cast::<Button>()
            .unwrap()
            .command()
            .unwrap()
            .execute(None);
    }
    assert_eq!(calls.get(), 1);
    assert!(source.model().visible_dockables().is_none());
    let derived = NullDocumentOverride::new();
    derived.set_document_factory(Some(Rc::new(|| None)));
    panel.set_data_context(Some(Rc::new(derived.clone() as Rc<dyn IDocumentDock>)));
    Dispatcher::ui_thread().run_jobs(None);
    for i in 0..3 {
        panel
            .children()
            .get(i)
            .cast::<Button>()
            .unwrap()
            .command()
            .unwrap()
            .execute(None);
    }
    assert_eq!(
        &*derived.calls.borrow(),
        &["add_document_null", "add_document_null", "add_tool_null"]
    );
    panel.set_data_context(None);
    let factory = NullFactory::new();
    source.model().set_factory(Some(factory.clone()));
    panel.set_data_context(Some(Rc::new(source.model())));
    Dispatcher::ui_thread().run_jobs(None);
    for index in 0..3 {
        panel
            .children()
            .get(index)
            .cast::<Button>()
            .unwrap()
            .command()
            .unwrap()
            .execute(None);
    }
    assert_eq!(
        &*factory.calls.borrow(),
        &["add", "active", "focus"].repeat(3)
    );
    assert_eq!(calls.get(), 2);
    assert!(source.model().visible_dockables().is_none());
    panel.set_data_context(None);
}
#[test]
fn compiled_null_commands_use_original_no_factory_body_and_same_overridden_virtual_slot() {
    run(|| {
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://ferrodock_markup_tests/CoreNullDocumentBindings.xaml",
        )
        .unwrap()
        .unwrap();
        check(value.downcast_ref::<Ref<StackPanel>>().unwrap());
    });
}
#[test]
fn runtime_null_commands_preserve_original_callback_and_nullable_virtual_dispatch() {
    run(|| {
        let mut config = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default();
        config.local_assembly = Some(&crate::locator_factory::ASSEMBLY);
        config.use_compiled_bindings_by_default = true;
        let source = include_str!("CoreNullDocumentBindings.xaml");
        let emitted = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &[("Null.xaml", source)],
            Some("ferres://ferrodock_markup_tests/"),
            &config,
            &[],
        );
        assert_eq!(emitted.len(), 1);
        assert!(emitted[0].source.is_ok(), "{:?}", emitted[0].source);
        let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
            ferroui_markup_xaml::RuntimeXamlLoaderDocument::new(source),
            Some(config),
        )
        .unwrap();
        check(value.downcast_ref::<Ref<StackPanel>>().unwrap());
    });
}

struct NullFactory {
    component: ferrodock_model_reactiveui::Factory,
    calls: RefCell<Vec<&'static str>>,
}
impl NullFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: ferrodock_model_reactiveui::Factory::construct(weak.clone()),
            calls: RefCell::new(Vec::new()),
        })
    }
    fn record(&self, name: &'static str, item: Option<Rc<dyn IDockable>>) {
        assert!(item.is_none());
        self.calls.borrow_mut().push(name);
    }
}
impl ferrodock_model::FactoryBaseImpl for NullFactory {
    fn factory_base(&self) -> &ferrodock_model::FactoryBase {
        self.component.base()
    }
}
impl ferrodock_model_reactiveui::FactoryImpl for NullFactory {
    fn factory(&self) -> &ferrodock_model_reactiveui::Factory {
        &self.component
    }
    fn add_dockable(this: &Self, _dock: Option<Rc<dyn IDock>>, item: Option<Rc<dyn IDockable>>) {
        this.record("add", item);
    }
    fn set_active_dockable(this: &Self, item: Option<Rc<dyn IDockable>>) {
        this.record("active", item);
    }
    fn set_focused_dockable(
        this: &Self,
        _dock: Option<Rc<dyn IDock>>,
        item: Option<Rc<dyn IDockable>>,
    ) {
        this.record("focus", item);
    }
    fn init_dockable(
        this: &Self,
        item: Option<Rc<dyn IDockable>>,
        _owner: Option<Rc<dyn IDockable>>,
    ) {
        this.record("init", item);
    }
}
ferrodock_model_reactiveui::impl_factory!(NullFactory);

#[test]
fn original_interface_and_abstract_factory_metadata_forward_null_to_same_virtual_slots() {
    run(|| {
        use ferrodock_model_ferro::{FactoryBaseReference, IFactoryMetadata};
        let factory = NullFactory::new();
        let dock = DocumentDock::new();
        let interface: Rc<dyn IFactory> = factory.clone();
        for (metadata, receiver) in [
            (
                <dyn IFactoryMetadata as MarkupTyped>::MARKUP,
                Rc::new(interface) as BoxedValue,
            ),
            (
                FactoryBaseReference::MARKUP,
                Rc::new(FactoryBaseReference::new(factory.clone())) as BoxedValue,
            ),
        ] {
            for (name, arguments) in [
                (
                    "InitDockable",
                    vec![
                        None,
                        Some(Rc::new(dock.clone() as Rc<dyn IDockable>) as BoxedValue),
                    ],
                ),
                (
                    "AddDockable",
                    vec![
                        Some(Rc::new(dock.clone() as Rc<dyn IDock>) as BoxedValue),
                        None,
                    ],
                ),
                ("SetActiveDockable", vec![None]),
            ] {
                let mut values = vec![Some(receiver.clone())];
                values.extend(arguments);
                (metadata.find_methods(name).next().unwrap().invoke)(&values).unwrap();
            }
        }
        assert_eq!(
            &*factory.calls.borrow(),
            &["init", "add", "active"].repeat(2)
        );
        assert!(dock.visible_dockables().is_none());
    });
}

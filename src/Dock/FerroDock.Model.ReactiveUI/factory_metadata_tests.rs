//! Full optional metadata contracts exercised at the factory boundary.
use crate::controls::*;
use crate::core::DockableBaseImpl;
use crate::factory_init_tests::factory;
use ferrodock_model::collections::{IEnumerable, IEnumerator, ListHandle};
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::{BoxedValue, FactoryBaseImpl};
use ferroui_reactiveui::{reactive_object, ICommand};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};

type SourceRemovedCallback = Rc<dyn Fn(Option<BoxedValue>)>;

#[derive(Default)]
struct Source {
    items: Rc<RefCell<Vec<Option<BoxedValue>>>>,
    remove_calls: RefCell<Vec<Option<BoxedValue>>>,
    on_remove: RefCell<Option<SourceRemovedCallback>>,
}
impl Source {
    fn remove(&self, value: Option<BoxedValue>) -> bool {
        self.remove_calls.borrow_mut().push(value.clone());
        let index = self
            .items
            .borrow()
            .iter()
            .position(|item| match (item, &value) {
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            });
        if let Some(index) = index {
            self.items.borrow_mut().remove(index);
            let callback = self.on_remove.borrow().clone();
            if let Some(callback) = callback {
                callback(value);
            }
            true
        } else {
            false
        }
    }
}
struct Cursor {
    items: Rc<RefCell<Vec<Option<BoxedValue>>>>,
    index: Cell<isize>,
    disposed: Cell<bool>,
}
impl IEnumerable for Source {
    fn get_enumerator(&self) -> Rc<dyn IEnumerator> {
        Rc::new(Cursor {
            items: self.items.clone(),
            index: Cell::new(-1),
            disposed: Cell::new(false),
        })
    }
}
impl IEnumerator for Cursor {
    fn current(&self) -> Option<BoxedValue> {
        assert!(self.index.get() >= 0, "source cursor has not started");
        self.items.borrow()[self.index.get() as usize].clone()
    }
    fn move_next(&self) -> bool {
        assert!(!self.disposed.get(), "source cursor disposed");
        let next = self.index.get() + 1;
        self.index.set(next);
        next < self.items.borrow().len() as isize
    }
    fn reset(&self) {
        self.index.set(-1);
    }
    fn as_disposable(&self) -> Option<&dyn IDisposable> {
        Some(self)
    }
}
impl IDisposable for Cursor {
    fn dispose(&self) {
        self.disposed.set(true);
    }
}

#[derive(Default)]
struct Template(RefCell<Option<BoxedValue>>);
impl IDocumentTemplate for Template {
    fn content(&self) -> Option<BoxedValue> {
        self.0.borrow().clone()
    }
    fn set_content(&self, value: Option<BoxedValue>) {
        *self.0.borrow_mut() = value;
    }
}
impl IToolTemplate for Template {
    fn content(&self) -> Option<BoxedValue> {
        self.0.borrow().clone()
    }
    fn set_content(&self, value: Option<BoxedValue>) {
        *self.0.borrow_mut() = value;
    }
}
#[derive(Default)]
struct Selector {
    indexes: RefCell<Vec<i32>>,
    skip: Cell<bool>,
}
impl IDocumentItemTemplateSelector for Selector {
    fn select_template(
        &self,
        dock: Rc<dyn IItemsSourceDock>,
        item: BoxedValue,
        index: i32,
    ) -> Option<BoxedValue> {
        assert!(dock.items_source().is_some());
        self.indexes.borrow_mut().push(index);
        if self.skip.get() {
            None
        } else {
            Some(item)
        }
    }
}
impl IToolItemTemplateSelector for Selector {
    fn select_template(
        &self,
        dock: Rc<dyn IToolItemsSourceDock>,
        item: BoxedValue,
        index: i32,
    ) -> Option<BoxedValue> {
        assert!(dock.items_source().is_some());
        self.indexes.borrow_mut().push(index);
        if self.skip.get() {
            None
        } else {
            Some(item)
        }
    }
}
#[derive(Default)]
struct Extras {
    source: Rc<Source>,
    generator: RefCell<Option<Rc<dyn IDockItemContainerGenerator>>>,
    theme: RefCell<Option<BoxedValue>>,
    unregister: Cell<Option<bool>>,
    generated: RefCell<Vec<Rc<dyn IDockable>>>,
}

pub(crate) struct DocumentMetadata {
    base: DocumentDock,
    extras: Extras,
    template: RefCell<Option<Rc<dyn IDocumentTemplate>>>,
    selector: RefCell<Option<Rc<dyn IDocumentItemTemplateSelector>>>,
}
reactive_object!(DocumentMetadata, base);
impl DockableBaseImpl for DocumentMetadata {}
crate::impl_dock!(@base DocumentMetadata, DocumentDock,
    fn as_document_dock(self: Rc<Self>) -> Option<Rc<dyn IDocumentDock>> { Some(self) },
    fn as_document_dock_content(self: Rc<Self>) -> Option<Rc<dyn IDocumentDockContent>> { Some(self) },
    fn as_items_source_dock(self: Rc<Self>) -> Option<Rc<dyn IItemsSourceDock>> { Some(self) },
    fn as_document_dock_factory(self: Rc<Self>) -> Option<Rc<dyn IDocumentDockFactory>> { Some(self) }
);
impl ILocalTarget for DocumentMetadata {}
impl DocumentMetadata {
    pub(crate) fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(|weak: &Weak<Self>| Self {
            base: DocumentDock::construct(weak),
            extras: Extras::default(),
            template: RefCell::new(None),
            selector: RefCell::new(None),
        });
        model.base.initialize(&model);
        model
    }
}
impl IDocumentDock for DocumentMetadata {
    fn can_create_document(&self) -> bool {
        self.base.can_create_document()
    }
    fn set_can_create_document(&self, value: bool) {
        self.base.set_can_create_document(value)
    }
    fn create_document(&self) -> Option<Rc<dyn ICommand>> {
        self.base.create_document()
    }
    fn set_create_document(&self, value: Option<Rc<dyn ICommand>>) {
        self.base.set_create_document(value)
    }
    fn enable_window_drag(&self) -> bool {
        self.base.enable_window_drag()
    }
    fn set_enable_window_drag(&self, value: bool) {
        self.base.set_enable_window_drag(value)
    }
    fn layout_mode(&self) -> DocumentLayoutMode {
        self.base.layout_mode()
    }
    fn set_layout_mode(&self, value: DocumentLayoutMode) {
        self.base.set_layout_mode(value)
    }
    fn tabs_layout(&self) -> DocumentTabLayout {
        self.base.tabs_layout()
    }
    fn set_tabs_layout(&self, value: DocumentTabLayout) {
        self.base.set_tabs_layout(value)
    }
    fn close_button_show_mode(&self) -> DocumentCloseButtonShowMode {
        self.base.close_button_show_mode()
    }
    fn set_close_button_show_mode(&self, value: DocumentCloseButtonShowMode) {
        self.base.set_close_button_show_mode(value)
    }
    fn empty_content(&self) -> Option<BoxedValue> {
        self.base.empty_content()
    }
    fn set_empty_content(&self, value: Option<BoxedValue>) {
        self.base.set_empty_content(value)
    }
    fn cascade_documents(&self) -> Option<Rc<dyn ICommand>> {
        self.base.cascade_documents()
    }
    fn set_cascade_documents(&self, value: Option<Rc<dyn ICommand>>) {
        self.base.set_cascade_documents(value)
    }
    fn tile_documents_horizontal(&self) -> Option<Rc<dyn ICommand>> {
        self.base.tile_documents_horizontal()
    }
    fn set_tile_documents_horizontal(&self, value: Option<Rc<dyn ICommand>>) {
        self.base.set_tile_documents_horizontal(value)
    }
    fn tile_documents_vertical(&self) -> Option<Rc<dyn ICommand>> {
        self.base.tile_documents_vertical()
    }
    fn set_tile_documents_vertical(&self, value: Option<Rc<dyn ICommand>>) {
        self.base.set_tile_documents_vertical(value)
    }
    fn restore_documents(&self) -> Option<Rc<dyn ICommand>> {
        self.base.restore_documents()
    }
    fn set_restore_documents(&self, value: Option<Rc<dyn ICommand>>) {
        self.base.set_restore_documents(value)
    }
    fn add_document(&self, document: Option<Rc<dyn IDockable>>) {
        self.base.add_document_nullable_core(document)
    }
    fn add_tool(&self, tool: Option<Rc<dyn IDockable>>) {
        self.base.add_tool_nullable_core(tool)
    }
}
impl IDocumentDockFactory for DocumentMetadata {
    fn document_factory(&self) -> Option<DocumentFactory> {
        self.base.document_factory()
    }
    fn set_document_factory(&self, value: Option<DocumentFactory>) {
        self.base.set_document_factory(value);
    }
}
impl IDocumentDockContent for DocumentMetadata {
    fn document_template(&self) -> Option<Rc<dyn IDocumentTemplate>> {
        self.template.borrow().clone()
    }
    fn set_document_template(&self, value: Option<Rc<dyn IDocumentTemplate>>) {
        *self.template.borrow_mut() = value;
    }
    fn create_document_from_template(&self) -> Option<BoxedValue> {
        self.document_template()
            .and_then(|template| template.content())
    }
}
impl IItemsSourceDock for DocumentMetadata {
    fn items_source(&self) -> Option<Rc<dyn IEnumerable>> {
        Some(self.extras.source.clone())
    }
    fn item_container_generator(&self) -> Option<Rc<dyn IDockItemContainerGenerator>> {
        self.extras.generator.borrow().clone()
    }
    fn document_item_container_theme(&self) -> Option<BoxedValue> {
        self.extras.theme.borrow().clone()
    }
    fn set_document_item_container_theme(&self, value: Option<BoxedValue>) {
        *self.extras.theme.borrow_mut() = value;
    }
    fn document_item_template_selector(&self) -> Option<Rc<dyn IDocumentItemTemplateSelector>> {
        self.selector.borrow().clone()
    }
    fn set_document_item_template_selector(
        &self,
        value: Option<Rc<dyn IDocumentItemTemplateSelector>>,
    ) {
        *self.selector.borrow_mut() = value;
    }
    fn can_update_items_source_on_unregister(&self) -> Option<bool> {
        self.extras.unregister.get()
    }
    fn set_can_update_items_source_on_unregister(&self, value: Option<bool>) {
        self.extras.unregister.set(value);
    }
    fn is_document_from_items_source(&self, item: Rc<dyn IDockable>) -> bool {
        self.extras.generated.borrow().contains(&item)
    }
    fn remove_item_from_source(&self, item: Option<BoxedValue>) -> bool {
        self.extras.source.remove(item)
    }
}

pub(crate) struct ToolMetadata {
    base: ToolDock,
    extras: Extras,
    template: RefCell<Option<Rc<dyn IToolTemplate>>>,
    selector: RefCell<Option<Rc<dyn IToolItemTemplateSelector>>>,
}
reactive_object!(ToolMetadata, base);
impl DockableBaseImpl for ToolMetadata {}
crate::impl_dock!(@base ToolMetadata, ToolDock,
    fn as_tool_dock(self: Rc<Self>) -> Option<Rc<dyn IToolDock>> { Some(self) },
    fn as_tool_dock_content(self: Rc<Self>) -> Option<Rc<dyn IToolDockContent>> { Some(self) },
    fn as_tool_items_source_dock(self: Rc<Self>) -> Option<Rc<dyn IToolItemsSourceDock>> { Some(self) }
);
impl ILocalTarget for ToolMetadata {}
impl ToolMetadata {
    pub(crate) fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(|weak: &Weak<Self>| Self {
            base: ToolDock::construct(weak),
            extras: Extras::default(),
            template: RefCell::new(None),
            selector: RefCell::new(None),
        });
        model.base.initialize(&model);
        model
    }
}
impl IToolDock for ToolMetadata {
    fn alignment(&self) -> Alignment {
        self.base.alignment()
    }
    fn set_alignment(&self, value: Alignment) {
        self.base.set_alignment(value)
    }
    fn is_expanded(&self) -> bool {
        self.base.is_expanded()
    }
    fn set_is_expanded(&self, value: bool) {
        self.base.set_is_expanded(value)
    }
    fn auto_hide(&self) -> bool {
        self.base.auto_hide()
    }
    fn set_auto_hide(&self, value: bool) {
        self.base.set_auto_hide(value)
    }
    fn grip_mode(&self) -> GripMode {
        self.base.grip_mode()
    }
    fn set_grip_mode(&self, value: GripMode) {
        self.base.set_grip_mode(value)
    }
    fn add_tool(&self, tool: Option<Rc<dyn IDockable>>) {
        self.base.add_tool_nullable_core(tool)
    }
}
impl IToolDockContent for ToolMetadata {
    fn tool_template(&self) -> Option<Rc<dyn IToolTemplate>> {
        self.template.borrow().clone()
    }
    fn set_tool_template(&self, value: Option<Rc<dyn IToolTemplate>>) {
        *self.template.borrow_mut() = value;
    }
    fn create_tool_from_template(&self) -> Option<BoxedValue> {
        self.tool_template().and_then(|template| template.content())
    }
}
impl IToolItemsSourceDock for ToolMetadata {
    fn items_source(&self) -> Option<Rc<dyn IEnumerable>> {
        Some(self.extras.source.clone())
    }
    fn item_container_generator(&self) -> Option<Rc<dyn IDockItemContainerGenerator>> {
        self.extras.generator.borrow().clone()
    }
    fn tool_item_container_theme(&self) -> Option<BoxedValue> {
        self.extras.theme.borrow().clone()
    }
    fn set_tool_item_container_theme(&self, value: Option<BoxedValue>) {
        *self.extras.theme.borrow_mut() = value;
    }
    fn tool_item_template_selector(&self) -> Option<Rc<dyn IToolItemTemplateSelector>> {
        self.selector.borrow().clone()
    }
    fn set_tool_item_template_selector(&self, value: Option<Rc<dyn IToolItemTemplateSelector>>) {
        *self.selector.borrow_mut() = value;
    }
    fn can_update_items_source_on_unregister(&self) -> Option<bool> {
        self.extras.unregister.get()
    }
    fn set_can_update_items_source_on_unregister(&self, value: Option<bool>) {
        self.extras.unregister.set(value);
    }
    fn is_tool_from_items_source(&self, item: Rc<dyn IDockable>) -> bool {
        self.extras.generated.borrow().contains(&item)
    }
    fn remove_item_from_source(&self, item: Option<BoxedValue>) -> bool {
        self.extras.source.remove(item)
    }
}

#[derive(Default)]
struct Generator {
    calls: RefCell<Vec<(&'static str, i32)>>,
    skip: Cell<bool>,
}
impl IDockItemContainerGenerator for Generator {
    fn create_document_container(
        &self,
        dock: Rc<dyn IItemsSourceDock>,
        _item: BoxedValue,
        index: i32,
    ) -> Option<Rc<dyn IDockable>> {
        assert!(dock.items_source().is_some());
        self.calls.borrow_mut().push(("create-document", index));
        if self.skip.get() {
            None
        } else {
            Some(Document::new())
        }
    }
    fn prepare_document_container(
        &self,
        dock: Rc<dyn IItemsSourceDock>,
        container: Rc<dyn IDockable>,
        item: BoxedValue,
        index: i32,
    ) {
        assert!(dock.items_source().is_some());
        container.set_context(Some(item));
        self.calls.borrow_mut().push(("prepare-document", index));
    }
    fn clear_document_container(
        &self,
        dock: Rc<dyn IItemsSourceDock>,
        container: Rc<dyn IDockable>,
        item: Option<BoxedValue>,
    ) {
        assert!(dock.items_source().is_some());
        if let Some(item) = item {
            assert!(Rc::ptr_eq(&container.context().unwrap(), &item));
        }
        container.set_context(None);
        self.calls.borrow_mut().push(("clear-document", -1));
    }
    fn create_tool_container(
        &self,
        dock: Rc<dyn IToolItemsSourceDock>,
        _item: BoxedValue,
        index: i32,
    ) -> Option<Rc<dyn IDockable>> {
        assert!(dock.items_source().is_some());
        self.calls.borrow_mut().push(("create-tool", index));
        if self.skip.get() {
            None
        } else {
            Some(Tool::new())
        }
    }
    fn prepare_tool_container(
        &self,
        dock: Rc<dyn IToolItemsSourceDock>,
        container: Rc<dyn IDockable>,
        item: BoxedValue,
        index: i32,
    ) {
        assert!(dock.items_source().is_some());
        container.set_context(Some(item));
        self.calls.borrow_mut().push(("prepare-tool", index));
    }
    fn clear_tool_container(
        &self,
        dock: Rc<dyn IToolItemsSourceDock>,
        container: Rc<dyn IDockable>,
        item: Option<BoxedValue>,
    ) {
        assert!(dock.items_source().is_some());
        if let Some(item) = item {
            assert!(Rc::ptr_eq(&container.context().unwrap(), &item));
        }
        container.set_context(None);
        self.calls.borrow_mut().push(("clear-tool", -1));
    }
}

#[test]
fn document_metadata_is_copied_by_window_float_all_and_both_new_pane_operations() {
    for operation in 0..4 {
        let factory = factory();
        let root = RootDock::new();
        let source = DocumentMetadata::new();
        source.set_is_collapsable(false);
        source.set_layout_mode(DocumentLayoutMode::Mdi);
        let item = Document::new();
        source.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            item.clone() as Rc<dyn IDockable>
        )])));
        root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            source.clone() as Rc<dyn IDockable>
        )])));
        factory.init_dockable(Some(root.clone()), None);
        let target = DocumentMetadata::new();
        let created = target.clone();
        *factory.document_dock_creator.borrow_mut() = Some(Rc::new(move || created.clone()));
        let template: Rc<dyn IDocumentTemplate> = Rc::new(Template::default());
        let theme: BoxedValue = Rc::new("theme".to_owned());
        let selector: Rc<dyn IDocumentItemTemplateSelector> = Rc::new(Selector::default());
        let document_factory: DocumentFactory = Rc::new(|| Some(Document::new()));
        source.set_document_template(Some(template.clone()));
        source.set_document_item_container_theme(Some(theme.clone()));
        source.set_document_item_template_selector(Some(selector.clone()));
        source.set_can_update_items_source_on_unregister(Some(false));
        source.set_document_factory(Some(document_factory.clone()));
        let generator: Rc<dyn IDockItemContainerGenerator> = Rc::new(Generator::default());
        *source.extras.generator.borrow_mut() = Some(generator);
        source
            .extras
            .source
            .items
            .borrow_mut()
            .push(Some(Rc::new(123_i32)));
        let window = match operation {
            0 => factory.create_window_from(Some(item.clone())),
            1 => {
                factory.float_all_dockables(Some(item.clone()));
                root.windows()
                    .map(|windows| windows.get(0).expect("nonnull window fixture"))
            }
            2 => {
                factory.new_horizontal_document_dock(item.clone());
                None
            }
            _ => {
                factory.new_vertical_document_dock(item.clone());
                None
            }
        };
        assert!(Rc::ptr_eq(&target.document_template().unwrap(), &template));
        assert!(Rc::ptr_eq(
            &target.document_item_container_theme().unwrap(),
            &theme
        ));
        assert!(Rc::ptr_eq(
            &target.document_item_template_selector().unwrap(),
            &selector
        ));
        assert!(Rc::ptr_eq(
            &target.document_factory().unwrap(),
            &document_factory
        ));
        assert_eq!(target.can_update_items_source_on_unregister(), Some(false));
        assert_eq!(
            target.layout_mode(),
            if operation >= 2 {
                DocumentLayoutMode::Mdi
            } else {
                DocumentLayoutMode::Tabbed
            }
        );
        // The source copies template metadata, never ItemsSource, its generator, or generated membership.
        assert!(!Rc::ptr_eq(
            &target.items_source().unwrap(),
            &source.items_source().unwrap()
        ));
        assert!(!target.items_source().unwrap().get_enumerator().move_next());
        assert!(target.item_container_generator().is_none());
        assert!(!target.is_document_from_items_source(item));
        drop(window); // Retain the new root throughout the assertions; backlinks are intentionally weak.
    }
}

#[test]
fn tool_metadata_is_copied_by_window_and_float_all_without_copying_item_source() {
    for all in [false, true] {
        let factory = factory();
        let root = RootDock::new();
        let source = ToolMetadata::new();
        source.set_is_collapsable(false);
        source.set_alignment(Alignment::Top);
        source.set_is_expanded(false);
        source.set_auto_hide(false);
        source.set_grip_mode(GripMode::Hidden);
        let item = Tool::new();
        source.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            item.clone() as Rc<dyn IDockable>
        )])));
        root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            source.clone() as Rc<dyn IDockable>
        )])));
        factory.init_dockable(Some(root.clone()), None);
        let target = ToolMetadata::new();
        let created = target.clone();
        *factory.tool_dock_creator.borrow_mut() = Some(Rc::new(move || created.clone()));
        let template: Rc<dyn IToolTemplate> = Rc::new(Template::default());
        let theme: BoxedValue = Rc::new("theme".to_owned());
        let selector: Rc<dyn IToolItemTemplateSelector> = Rc::new(Selector::default());
        source.set_tool_template(Some(template.clone()));
        source.set_tool_item_container_theme(Some(theme.clone()));
        source.set_tool_item_template_selector(Some(selector.clone()));
        source.set_can_update_items_source_on_unregister(Some(true));
        let window = if all {
            factory.float_all_dockables(Some(item.clone()));
            root.windows()
                .unwrap()
                .get(0)
                .expect("nonnull window fixture")
        } else {
            factory.create_window_from(Some(item.clone())).unwrap()
        };
        assert!(Rc::ptr_eq(&target.tool_template().unwrap(), &template));
        assert!(Rc::ptr_eq(
            &target.tool_item_container_theme().unwrap(),
            &theme
        ));
        assert!(Rc::ptr_eq(
            &target.tool_item_template_selector().unwrap(),
            &selector
        ));
        assert_eq!(target.can_update_items_source_on_unregister(), Some(true));
        assert_eq!(target.alignment(), Alignment::Top);
        assert!(!target.is_expanded() && !target.auto_hide());
        assert_eq!(target.grip_mode(), GripMode::Hidden);
        assert!(!Rc::ptr_eq(
            &target.items_source().unwrap(),
            &source.items_source().unwrap()
        ));
        assert!(target.item_container_generator().is_none());
        assert!(!target.is_tool_from_items_source(item));
        drop(window);
    }
}

#[test]
fn null_optional_metadata_overwrites_preconfigured_targets() {
    let factory = factory();
    let source = DocumentMetadata::new();
    let target = DocumentMetadata::new();
    target.set_document_template(Some(Rc::new(Template::default())));
    target.set_document_item_container_theme(Some(Rc::new(1_i32)));
    target.set_document_item_template_selector(Some(Rc::new(Selector::default())));
    target.set_can_update_items_source_on_unregister(Some(true));
    target.set_document_factory(Some(Rc::new(|| Some(Document::new()))));
    let created = target.clone();
    *factory.document_dock_creator.borrow_mut() = Some(Rc::new(move || created.clone()));
    let item = Document::new();
    item.set_owner(Some(source.clone()));
    let window = factory.create_window_from(Some(item)).unwrap();
    assert!(target.document_template().is_none());
    assert!(target.document_item_container_theme().is_none());
    assert!(target.document_item_template_selector().is_none());
    assert!(target.can_update_items_source_on_unregister().is_none());
    assert!(target.document_factory().is_none());
    drop(window);
    let source = ToolMetadata::new();
    let target = ToolMetadata::new();
    target.set_tool_template(Some(Rc::new(Template::default())));
    target.set_tool_item_container_theme(Some(Rc::new(1_i32)));
    target.set_tool_item_template_selector(Some(Rc::new(Selector::default())));
    target.set_can_update_items_source_on_unregister(Some(false));
    let created = target.clone();
    *factory.tool_dock_creator.borrow_mut() = Some(Rc::new(move || created.clone()));
    let item = Tool::new();
    item.set_owner(Some(source.clone()));
    let window = factory.create_window_from(Some(item)).unwrap();
    assert!(target.tool_template().is_none());
    assert!(target.tool_item_container_theme().is_none());
    assert!(target.tool_item_template_selector().is_none());
    assert!(target.can_update_items_source_on_unregister().is_none());
    drop(window);
}

#[test]
fn optional_contracts_dispatch_full_container_lifecycle_selectors_and_nullable_source_removal() {
    let document = DocumentMetadata::new();
    let tool = ToolMetadata::new();
    let item: BoxedValue = Rc::new(42_i32);
    let generator = Rc::new(Generator::default());
    *document.extras.generator.borrow_mut() = Some(generator.clone());
    *tool.extras.generator.borrow_mut() = Some(generator.clone());
    document
        .extras
        .source
        .items
        .borrow_mut()
        .extend([None, Some(item.clone())]);
    tool.extras
        .source
        .items
        .borrow_mut()
        .extend([None, Some(item.clone())]);
    let documents = (document.clone() as Rc<dyn IDockable>)
        .as_items_source_dock()
        .unwrap();
    let tools = (tool.clone() as Rc<dyn IDockable>)
        .as_tool_items_source_dock()
        .unwrap();
    let lifecycle = documents.item_container_generator().unwrap();
    assert!(Rc::ptr_eq(
        &lifecycle,
        &tools.item_container_generator().unwrap()
    ));
    let container = lifecycle
        .create_document_container(documents.clone(), item.clone(), -1)
        .unwrap();
    lifecycle.prepare_document_container(documents.clone(), container.clone(), item.clone(), 3);
    assert!(Rc::ptr_eq(&container.context().unwrap(), &item));
    document
        .extras
        .generated
        .borrow_mut()
        .push(container.clone());
    assert!(documents.is_document_from_items_source(container.clone()));
    assert!(!documents.is_document_from_items_source(Document::new()));
    lifecycle.clear_document_container(documents.clone(), container.clone(), Some(item.clone()));
    assert!(container.context().is_none());
    let container = lifecycle
        .create_tool_container(tools.clone(), item.clone(), 4)
        .unwrap();
    lifecycle.prepare_tool_container(tools.clone(), container.clone(), item.clone(), 4);
    tool.extras.generated.borrow_mut().push(container.clone());
    assert!(tools.is_tool_from_items_source(container.clone()));
    lifecycle.clear_tool_container(tools.clone(), container.clone(), None);
    assert!(container.context().is_none());
    generator.skip.set(true);
    assert!(lifecycle
        .create_document_container(documents.clone(), item.clone(), 8)
        .is_none());
    assert!(lifecycle
        .create_tool_container(tools.clone(), item.clone(), 9)
        .is_none());
    assert_eq!(
        *generator.calls.borrow(),
        [
            ("create-document", -1),
            ("prepare-document", 3),
            ("clear-document", -1),
            ("create-tool", 4),
            ("prepare-tool", 4),
            ("clear-tool", -1),
            ("create-document", 8),
            ("create-tool", 9)
        ]
    );
    let selector = Rc::new(Selector::default());
    let doc_selector: Rc<dyn IDocumentItemTemplateSelector> = selector.clone();
    let tool_selector: Rc<dyn IToolItemTemplateSelector> = selector.clone();
    assert!(Rc::ptr_eq(
        &doc_selector
            .select_template(documents.clone(), item.clone(), -1)
            .unwrap(),
        &item
    ));
    assert!(Rc::ptr_eq(
        &tool_selector
            .select_template(tools.clone(), item.clone(), 5)
            .unwrap(),
        &item
    ));
    selector.skip.set(true);
    assert!(doc_selector
        .select_template(documents.clone(), item.clone(), 6)
        .is_none());
    assert!(tool_selector
        .select_template(tools.clone(), item.clone(), 7)
        .is_none());
    assert_eq!(*selector.indexes.borrow(), [-1, 5, 6, 7]);
    assert!(documents.remove_item_from_source(None));
    assert!(!documents.remove_item_from_source(None));
    assert!(documents.remove_item_from_source(Some(item.clone())));
    assert!(!documents.remove_item_from_source(Some(item.clone())));
    assert!(tools.remove_item_from_source(None));
    assert!(tools.remove_item_from_source(Some(item)));
    for policy in [None, Some(false), Some(true)] {
        documents.set_can_update_items_source_on_unregister(policy);
        tools.set_can_update_items_source_on_unregister(policy);
        assert_eq!(documents.can_update_items_source_on_unregister(), policy);
        assert_eq!(tools.can_update_items_source_on_unregister(), policy);
    }
}

#[test]
fn enumerable_bridge_preserves_source_cursor_identity_nulls_reset_and_explicit_disposal() {
    let source = Rc::new(Source::default());
    let value: BoxedValue = Rc::new(42_i32);
    source
        .items
        .borrow_mut()
        .extend([Some(value.clone()), None]);
    let enumerable: Rc<dyn IEnumerable> = source;
    let first = enumerable.get_enumerator();
    let second = enumerable.get_enumerator();
    assert!(!Rc::ptr_eq(&first, &second));
    assert!(catch_unwind(AssertUnwindSafe(|| first.current())).is_err());
    assert!(first.move_next());
    assert!(Rc::ptr_eq(&first.current().unwrap(), &value));
    assert!(first.move_next());
    assert!(first.current().is_none());
    assert!(!first.move_next());
    assert!(catch_unwind(AssertUnwindSafe(|| first.current())).is_err());
    first.reset();
    assert!(first.move_next());
    assert!(second.move_next());
    assert!(Rc::ptr_eq(
        &first.current().unwrap(),
        &second.current().unwrap()
    ));
    first.as_disposable().unwrap().dispose();
    assert!(catch_unwind(AssertUnwindSafe(|| first.move_next())).is_err());
    assert!(second.move_next());
}

struct GeneratedLayout {
    factory: Rc<crate::factory_parameter_tests::RecordingFactory>,
    root: Rc<RootDock>,
    dock: Rc<dyn IDock>,
    item: Rc<dyn IDockable>,
    value: BoxedValue,
    source: Rc<Source>,
}
impl GeneratedLayout {
    fn new(tool: bool) -> Self {
        let factory = factory();
        factory.set_hide_tools_on_close(true);
        factory.set_hide_documents_on_close(true);
        let root = RootDock::new();
        let item: Rc<dyn IDockable> = if tool { Tool::new() } else { Document::new() };
        let value: BoxedValue = Rc::new("generated".to_owned());
        item.set_context(Some(value.clone()));
        let (dock, source): (Rc<dyn IDock>, Rc<Source>) = if tool {
            let dock = ToolMetadata::new();
            dock.extras.generated.borrow_mut().push(item.clone());
            let source = dock.extras.source.clone();
            (dock, source)
        } else {
            let dock = DocumentMetadata::new();
            dock.extras.generated.borrow_mut().push(item.clone());
            let source = dock.extras.source.clone();
            (dock, source)
        };
        source.items.borrow_mut().push(Some(value.clone()));
        dock.set_is_collapsable(false);
        dock.set_visible_dockables(Some(ListHandle::from_items(vec![Some(item.clone())])));
        root.set_visible_dockables(Some(ListHandle::from_items(vec![Some(
            dock.clone() as Rc<dyn IDockable>
        )])));
        factory.init_dockable(Some(root.clone()), None);
        Self {
            factory,
            root,
            dock,
            item,
            value,
            source,
        }
    }
    fn track(&self) {
        FactoryBaseImpl::track_items_source_dockable_with_item(
            self.factory.as_ref(),
            self.item.clone(),
            self.dock.clone(),
            Some(self.value.clone()),
        );
    }
    fn move_to_manual_dock(&self) -> Rc<DocumentDock> {
        let target = DocumentDock::new();
        target.set_is_collapsable(false);
        self.factory
            .add_dockable(Some(self.root.clone()), Some(target.clone()));
        self.factory.move_dockable_between_docks(
            Some(self.dock.clone()),
            Some(target.clone()),
            Some(self.item.clone()),
            None,
        );
        target
    }
}

#[test]
fn generated_document_and_tool_close_remove_source_before_layout_and_ignore_hide_flags() {
    for tool in [false, true] {
        let layout = GeneratedLayout::new(tool);
        let events = Rc::new(RefCell::new(Vec::new()));
        let log = events.clone();
        layout
            .factory
            .dockable_closing(Rc::new(move |_| log.borrow_mut().push("closing")));
        let log = events.clone();
        *layout.source.on_remove.borrow_mut() =
            Some(Rc::new(move |_| log.borrow_mut().push("source")));
        let log = events.clone();
        layout
            .factory
            .dockable_removed(Rc::new(move |_| log.borrow_mut().push("removed")));
        let log = events.clone();
        layout
            .factory
            .dockable_closed(Rc::new(move |_| log.borrow_mut().push("closed")));
        layout.factory.close_dockable(Some(layout.item));
        assert_eq!(*events.borrow(), ["closing", "source", "removed", "closed"]);
        assert!(layout.source.items.borrow().is_empty());
        assert_eq!(layout.dock.visible_dockables().unwrap().count(), 0);
        assert!(layout.root.hidden_dockables().is_none());
    }
}

#[test]
fn closing_floated_generated_items_uses_tracked_owner_even_after_source_dock_collapse() {
    for tool in [false, true] {
        let layout = GeneratedLayout::new(tool);
        layout.track();
        layout.dock.set_is_collapsable(true);
        layout.factory.float_dockable(Some(layout.item.clone()));
        assert!(!layout
            .root
            .visible_dockables()
            .unwrap()
            .contains(&Some((layout.dock.clone() as Rc<dyn IDockable>).clone())));
        let window = layout
            .root
            .windows()
            .unwrap()
            .get(0)
            .expect("nonnull window fixture");
        assert!(window.layout().is_some());
        layout.factory.close_dockable(Some(layout.item.clone()));
        assert!(layout.source.items.borrow().is_empty());
        assert_eq!(layout.source.remove_calls.borrow().len(), 1);
        assert!(layout.root.hidden_dockables().is_none());
        // Untracking belongs to the source control's collection observer, not FactoryBase.Close.
        assert!(layout
            .factory
            .get_container_from_item(Some(layout.value))
            .is_some());
    }
}

#[test]
fn closing_uses_current_context_while_tracking_preserves_the_original_explicit_item() {
    let layout = GeneratedLayout::new(false);
    layout.track();
    let next: BoxedValue = Rc::new("changed".to_owned());
    layout.source.items.borrow_mut().push(Some(next.clone()));
    layout.item.set_context(Some(next.clone()));
    let target = layout.move_to_manual_dock();
    layout.factory.close_dockable(Some(layout.item.clone()));
    assert_eq!(target.visible_dockables().unwrap().count(), 0);
    assert!(Rc::ptr_eq(
        &layout.source.remove_calls.borrow()[0].clone().unwrap(),
        &next
    ));
    assert_eq!(layout.source.items.borrow().len(), 1);
    assert!(Rc::ptr_eq(
        &layout.source.items.borrow()[0].clone().unwrap(),
        &layout.value
    ));
    assert!(Rc::ptr_eq(
        &layout
            .factory
            .get_container_from_item(Some(layout.value))
            .unwrap(),
        &layout.item
    ));
    assert!(layout.factory.get_container_from_item(Some(next)).is_none());
}

#[test]
fn recognized_current_document_owner_takes_precedence_over_tracked_owner() {
    let layout = GeneratedLayout::new(false);
    layout
        .factory
        .on_tracked_owner
        .replace(Some(Rc::new(|_| panic!("current source owner must win"))));
    layout.factory.close_dockable(Some(layout.item));
    assert!(layout.source.items.borrow().is_empty());
}

#[test]
fn rejected_tracked_owner_falls_back_to_visible_root_source_for_both_container_types() {
    for tool in [false, true] {
        let layout = GeneratedLayout::new(tool);
        let wrong: Rc<dyn IDockable> = if tool {
            ToolMetadata::new()
        } else {
            DocumentMetadata::new()
        };
        FactoryBaseImpl::track_items_source_dockable_with_item(
            layout.factory.as_ref(),
            layout.item.clone(),
            wrong,
            None,
        );
        let target = layout.move_to_manual_dock();
        layout.factory.close_dockable(Some(layout.item));
        assert!(layout.source.items.borrow().is_empty());
        assert_eq!(target.visible_dockables().unwrap().count(), 0);
        assert!(layout.root.hidden_dockables().is_none());
    }
}

#[test]
fn root_source_search_remains_visible_only_and_does_not_resolve_hidden_source_docks() {
    for tool in [false, true] {
        let layout = GeneratedLayout::new(tool);
        let target = layout.move_to_manual_dock();
        layout.factory.hide_dockable(Some(layout.dock.clone()));
        layout.factory.close_dockable(Some(layout.item.clone()));
        assert_eq!(layout.source.items.borrow().len(), 1);
        assert!(layout.source.remove_calls.borrow().is_empty());
        assert!(layout
            .root
            .hidden_dockables()
            .unwrap()
            .contains(&Some((layout.item).clone())));
        assert_eq!(target.visible_dockables().unwrap().count(), 0);
    }
}

#[test]
fn recognized_source_suppresses_hide_when_context_is_null_or_source_removal_returns_false() {
    for tool in [false, true] {
        for null_context in [false, true] {
            let layout = GeneratedLayout::new(tool);
            layout.item.set_context(if null_context {
                None
            } else {
                Some(Rc::new("missing".to_owned()))
            });
            layout.factory.close_dockable(Some(layout.item));
            assert_eq!(layout.source.items.borrow().len(), 1);
            assert_eq!(
                layout.source.remove_calls.borrow().len(),
                usize::from(!null_context)
            );
            assert!(layout.root.hidden_dockables().is_none());
            assert_eq!(layout.dock.visible_dockables().unwrap().count(), 0);
        }
    }
}

#[test]
fn source_observer_can_untrack_and_remove_reentrantly_without_duplicate_removed_events() {
    for tool in [false, true] {
        let layout = GeneratedLayout::new(tool);
        layout.track();
        let weak = Rc::downgrade(&layout.factory);
        let item = Rc::downgrade(&layout.item);
        *layout.source.on_remove.borrow_mut() = Some(Rc::new(move |_| {
            let factory = weak.upgrade().unwrap();
            let item = item.upgrade().unwrap();
            FactoryBaseImpl::untrack_items_source_dockable(factory.as_ref(), item.clone());
            factory.remove_dockable(Some(item), true);
        }));
        let removed = Rc::new(Cell::new(0));
        let log = removed.clone();
        layout
            .factory
            .dockable_removed(Rc::new(move |_| log.set(log.get() + 1)));
        let closed = Rc::new(Cell::new(0));
        let log = closed.clone();
        layout
            .factory
            .dockable_closed(Rc::new(move |_| log.set(log.get() + 1)));
        layout.factory.close_dockable(Some(layout.item));
        assert_eq!(removed.get(), 1);
        assert_eq!(closed.get(), 1);
        assert!(layout
            .factory
            .get_container_from_item(Some(layout.value))
            .is_none());
        assert!(layout.source.items.borrow().is_empty());
    }
}

#[test]
fn source_observer_failure_after_source_mutation_preserves_layout_and_skips_closed_event() {
    let layout = GeneratedLayout::new(false);
    *layout.source.on_remove.borrow_mut() = Some(Rc::new(|_| panic!("source observer failure")));
    layout
        .factory
        .dockable_closed(Rc::new(|_| panic!("closed must not run")));
    assert!(catch_unwind(AssertUnwindSafe(|| layout
        .factory
        .close_dockable(Some(layout.item.clone()))))
    .is_err());
    assert!(layout.source.items.borrow().is_empty());
    assert!(layout
        .dock
        .visible_dockables()
        .unwrap()
        .contains(&Some((layout.item).clone())));
}

#[test]
fn tool_document_membership_resolves_document_source_before_its_tool_source() {
    let layout = GeneratedLayout::new(true);
    let documents = DocumentMetadata::new();
    documents
        .extras
        .generated
        .borrow_mut()
        .push(layout.item.clone());
    documents
        .extras
        .source
        .items
        .borrow_mut()
        .push(Some(layout.value.clone()));
    layout
        .factory
        .add_dockable(Some(layout.root.clone()), Some(documents.clone()));
    layout.factory.close_dockable(Some(layout.item));
    assert!(documents.extras.source.items.borrow().is_empty());
    assert_eq!(layout.source.items.borrow().len(), 1);
    assert!(layout.source.remove_calls.borrow().is_empty());
}

#[test]
fn protected_tracked_owner_override_is_used_when_current_layout_owner_is_not_source_backed() {
    let layout = GeneratedLayout::new(false);
    let target = layout.move_to_manual_dock();
    let original = layout.dock.clone();
    let requests = Rc::new(Cell::new(0));
    let log = requests.clone();
    *layout.factory.on_tracked_owner.borrow_mut() = Some(Rc::new(move |_| {
        log.set(log.get() + 1);
        Some(original.clone())
    }));
    layout.factory.close_dockable(Some(layout.item));
    assert_eq!(requests.get(), 1);
    assert!(layout.source.items.borrow().is_empty());
    assert_eq!(target.visible_dockables().unwrap().count(), 0);
}

#[test]
fn typed_locators_resolve_optional_document_and_tool_content_contracts() {
    let factory = crate::Factory::new();
    let document = DocumentMetadata::new();
    let tool = ToolMetadata::new();
    let located = document.clone();
    let registry: DockableLocatorMap = Rc::new(RefCell::new(
        ferroui_system::collections::generic::Dictionary::new(),
    ));
    registry.borrow_mut().insert(
        "document".into(),
        Some(Rc::new(move || Some(located.clone()))),
    );
    let located = tool.clone();
    registry
        .borrow_mut()
        .insert("tool".into(), Some(Rc::new(move || Some(located.clone()))));
    factory.set_dockable_locator(Some(registry));
    assert!(Rc::ptr_eq(
        &factory
            .get_dockable::<dyn IDocumentDockContent>("document".into())
            .unwrap(),
        &(document as Rc<dyn IDocumentDockContent>)
    ));
    assert!(Rc::ptr_eq(
        &factory
            .get_dockable::<dyn IToolDockContent>("tool".into())
            .unwrap(),
        &(tool as Rc<dyn IToolDockContent>)
    ));
}

#[test]
fn service_tool_split_copies_target_settings_before_source_settings_and_keeps_requested_alignment()
{
    use crate::engine_test_support::PolicyFactory;
    use ferrodock_model::DockService;
    let factory = PolicyFactory::new();
    let root = factory.create_root_dock();
    let source = ToolMetadata::new();
    source.set_is_collapsable(false);
    source.set_is_expanded(true);
    source.set_auto_hide(true);
    let target = ToolMetadata::new();
    target.set_alignment(Alignment::Bottom);
    target.set_is_expanded(false);
    target.set_auto_hide(false);
    target.set_grip_mode(GripMode::Hidden);
    let created = ToolMetadata::new();
    let default_id = created.id();
    let template: Rc<dyn IToolTemplate> = Rc::new(Template::default());
    let theme: BoxedValue = Rc::new("target-theme".to_owned());
    let selector: Rc<dyn IToolItemTemplateSelector> = Rc::new(Selector::default());
    target.set_tool_template(Some(template.clone()));
    target.set_tool_item_container_theme(Some(theme.clone()));
    target.set_tool_item_template_selector(Some(selector.clone()));
    target.set_can_update_items_source_on_unregister(Some(false));
    let tool = factory.create_tool();
    factory.add_dockable(Some(source.clone()), Some(tool.clone()));
    factory.add_dockable(Some(root.clone()), Some(source.clone()));
    factory.add_dockable(Some(root.clone()), Some(target.clone()));
    let returned = created.clone();
    *factory.create_tool.borrow_mut() = Some(Rc::new(move || returned.clone()));
    assert!(DockService::new().split_dockable(tool, source, target, DockOperation::Right, true));
    assert_eq!(created.alignment(), Alignment::Right);
    assert!(!created.is_expanded() && !created.auto_hide());
    assert_eq!(created.grip_mode(), GripMode::Hidden);
    assert_eq!(created.id(), default_id);
    assert!(Rc::ptr_eq(&created.tool_template().unwrap(), &template));
    assert!(Rc::ptr_eq(
        &created.tool_item_container_theme().unwrap(),
        &theme
    ));
    assert!(Rc::ptr_eq(
        &created.tool_item_template_selector().unwrap(),
        &selector
    ));
    assert_eq!(created.can_update_items_source_on_unregister(), Some(false));
}

#[test]
fn service_document_split_copies_owner_metadata_and_delegate_but_preserves_default_layout_mode() {
    use crate::engine_test_support::PolicyFactory;
    use ferrodock_model::DockService;
    let factory = PolicyFactory::new();
    let root = factory.create_root_dock();
    let source = DocumentMetadata::new();
    source.set_is_collapsable(false);
    source.set_id("source".into());
    source.set_layout_mode(DocumentLayoutMode::Mdi);
    source.set_can_create_document(true);
    source.set_enable_window_drag(false);
    let target = factory.create_document_dock();
    let created = DocumentMetadata::new();
    let template: Rc<dyn IDocumentTemplate> = Rc::new(Template::default());
    let theme: BoxedValue = Rc::new("theme".to_owned());
    let selector: Rc<dyn IDocumentItemTemplateSelector> = Rc::new(Selector::default());
    let delegate: DocumentFactory = Rc::new(|| Some(Document::new()));
    source.set_document_template(Some(template.clone()));
    source.set_document_item_container_theme(Some(theme.clone()));
    source.set_document_item_template_selector(Some(selector.clone()));
    source.set_document_factory(Some(delegate.clone()));
    source.set_can_update_items_source_on_unregister(Some(true));
    let document = factory.create_document();
    factory.add_dockable(Some(source.clone()), Some(document.clone()));
    factory.add_dockable(Some(root.clone()), Some(source.clone()));
    factory.add_dockable(Some(root.clone()), Some(target.clone()));
    let returned = created.clone();
    *factory.create_document.borrow_mut() = Some(Rc::new(move || returned.clone()));
    assert!(DockService::new().split_dockable(
        document,
        source,
        target,
        DockOperation::Bottom,
        true
    ));
    assert_eq!(created.id(), "source");
    assert_eq!(created.layout_mode(), DocumentLayoutMode::Tabbed);
    assert!(created.can_create_document() && !created.enable_window_drag());
    assert!(Rc::ptr_eq(&created.document_template().unwrap(), &template));
    assert!(Rc::ptr_eq(
        &created.document_item_container_theme().unwrap(),
        &theme
    ));
    assert!(Rc::ptr_eq(
        &created.document_item_template_selector().unwrap(),
        &selector
    ));
    assert!(Rc::ptr_eq(&created.document_factory().unwrap(), &delegate));
    assert_eq!(created.can_update_items_source_on_unregister(), Some(true));
}

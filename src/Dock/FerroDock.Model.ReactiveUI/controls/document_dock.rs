// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::dockable_base::ContextValue;
use crate::core::{DockBase, DockableBaseImpl};
use crate::impl_dock;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::BoxedValue;
use ferroui_reactiveui::{reactive_object, ICommand, IReactiveObjectExt, ReactiveCommand};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Creates a document to add to a DocumentDock.
pub use ferrodock_model::controls::DocumentFactory;

/// Reactive document container with creation, activation and MDI arrangement commands.
pub struct DocumentDock {
    base: DockBase,
    empty_content: RefCell<ContextValue>,
    document_factory: RefCell<Option<DocumentFactory>>,
    can_create_document: RefCell<bool>,
    enable_window_drag: RefCell<bool>,
    tabs_layout: RefCell<DocumentTabLayout>,
    layout_mode: RefCell<DocumentLayoutMode>,
    close_button_show_mode: RefCell<DocumentCloseButtonShowMode>,
    create_document: RefCell<Option<Rc<dyn ICommand>>>,
    cascade_documents: RefCell<Option<Rc<dyn ICommand>>>,
    tile_documents_horizontal: RefCell<Option<Rc<dyn ICommand>>>,
    tile_documents_vertical: RefCell<Option<Rc<dyn ICommand>>>,
    restore_documents: RefCell<Option<Rc<dyn ICommand>>>,
}
reactive_object!(DocumentDock, base);
impl DockableBaseImpl for DocumentDock {}
impl ILocalTarget for DocumentDock {}
impl_dock!(
    DocumentDock,
    fn as_local_target(self: Rc<Self>) -> Option<Rc<dyn ILocalTarget>> {
        Some(self)
    },
    fn as_document_dock(self: Rc<Self>) -> Option<Rc<dyn IDocumentDock>> {
        Some(self)
    },
    fn as_document_dock_factory(self: Rc<Self>) -> Option<Rc<dyn IDocumentDockFactory>> {
        Some(self)
    }
);

impl DocumentDock {
    /// Creates a shared document container and initializes its base observations.
    pub fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(Self::construct);
        model.base.initialize(&model);
        model
    }
    /// Constructs the component for a model implementing the full document dock contracts.
    /// Initialize DockBase after the enclosing Rc::new_cyclic returns.
    pub fn construct<T: DockableBaseImpl + IDocumentDock + IDocumentDockFactory + 'static>(
        runtime: &Weak<T>,
    ) -> Self {
        let weak = runtime.clone();
        let create_document = Rc::new(ReactiveCommand::create(move || {
            if let Some(model) = weak.upgrade() {
                if let Some(factory) = model.document_factory() {
                    let document = factory();
                    model.add_document(document);
                }
            }
        }));
        let weak = runtime.clone();
        let cascade_documents = Rc::new(ReactiveCommand::create(move || {
            if let Some(model) = weak.upgrade() {
                if model.layout_mode() == DocumentLayoutMode::Mdi {
                    MdiLayoutHelper::cascade_documents(model.as_ref());
                }
            }
        }));
        let weak = runtime.clone();
        let tile_documents_horizontal = Rc::new(ReactiveCommand::create(move || {
            if let Some(model) = weak.upgrade() {
                if model.layout_mode() == DocumentLayoutMode::Mdi {
                    MdiLayoutHelper::tile_documents_horizontal(model.as_ref());
                }
            }
        }));
        let weak = runtime.clone();
        let tile_documents_vertical = Rc::new(ReactiveCommand::create(move || {
            if let Some(model) = weak.upgrade() {
                if model.layout_mode() == DocumentLayoutMode::Mdi {
                    MdiLayoutHelper::tile_documents_vertical(model.as_ref());
                }
            }
        }));
        let weak = runtime.clone();
        let restore_documents = Rc::new(ReactiveCommand::create(move || {
            if let Some(model) = weak.upgrade() {
                if model.layout_mode() == DocumentLayoutMode::Mdi {
                    MdiLayoutHelper::restore_documents(model.as_ref());
                }
            }
        }));
        Self {
            base: DockBase::construct(runtime.clone()),
            empty_content: RefCell::new(ContextValue(Some(Rc::new(
                "No documents open".to_owned(),
            )))),
            document_factory: RefCell::new(None),
            can_create_document: RefCell::new(false),
            enable_window_drag: RefCell::new(false),
            tabs_layout: RefCell::new(DocumentTabLayout::Top),
            layout_mode: RefCell::new(DocumentLayoutMode::Tabbed),
            close_button_show_mode: RefCell::new(DocumentCloseButtonShowMode::Always),
            create_document: RefCell::new(Some(create_document)),
            cascade_documents: RefCell::new(Some(cascade_documents)),
            tile_documents_horizontal: RefCell::new(Some(tile_documents_horizontal)),
            tile_documents_vertical: RefCell::new(Some(tile_documents_vertical)),
            restore_documents: RefCell::new(Some(restore_documents)),
        }
    }
    /// Gets `CanCreateDocument`.
    pub fn can_create_document(&self) -> bool {
        *self.can_create_document.borrow()
    }
    /// Sets `CanCreateDocument`, notifying only on a value change.
    pub fn set_can_create_document(&self, value: bool) {
        self.base.runtime().raise_and_set_if_changed(
            &self.can_create_document,
            value,
            "CanCreateDocument",
        );
    }
    /// Gets `EnableWindowDrag`.
    pub fn enable_window_drag(&self) -> bool {
        *self.enable_window_drag.borrow()
    }
    /// Sets `EnableWindowDrag`, notifying only on a value change.
    pub fn set_enable_window_drag(&self, value: bool) {
        self.base.runtime().raise_and_set_if_changed(
            &self.enable_window_drag,
            value,
            "EnableWindowDrag",
        );
    }
    /// Gets `TabsLayout`.
    pub fn tabs_layout(&self) -> DocumentTabLayout {
        *self.tabs_layout.borrow()
    }
    /// Sets `TabsLayout`, notifying only on a value change.
    pub fn set_tabs_layout(&self, value: DocumentTabLayout) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.tabs_layout, value, "TabsLayout");
    }
    /// Gets `LayoutMode`.
    pub fn layout_mode(&self) -> DocumentLayoutMode {
        *self.layout_mode.borrow()
    }
    /// Sets `LayoutMode`, notifying only on a value change.
    pub fn set_layout_mode(&self, value: DocumentLayoutMode) {
        self.base
            .runtime()
            .raise_and_set_if_changed(&self.layout_mode, value, "LayoutMode");
    }
    /// Gets `CloseButtonShowMode`.
    pub fn close_button_show_mode(&self) -> DocumentCloseButtonShowMode {
        *self.close_button_show_mode.borrow()
    }
    /// Sets `CloseButtonShowMode`, notifying only on a value change.
    pub fn set_close_button_show_mode(&self, value: DocumentCloseButtonShowMode) {
        self.base.runtime().raise_and_set_if_changed(
            &self.close_button_show_mode,
            value,
            "CloseButtonShowMode",
        );
    }
    /// Gets `CreateDocument`.
    pub fn create_document(&self) -> Option<Rc<dyn ICommand>> {
        self.create_document.borrow().clone()
    }
    /// Replaces `CreateDocument` without reactive notifications, matching the source auto-property.
    pub fn set_create_document(&self, value: Option<Rc<dyn ICommand>>) {
        *self.create_document.borrow_mut() = value;
    }
    /// Gets `CascadeDocuments`.
    pub fn cascade_documents(&self) -> Option<Rc<dyn ICommand>> {
        self.cascade_documents.borrow().clone()
    }
    /// Replaces `CascadeDocuments` without reactive notifications, matching the source auto-property.
    pub fn set_cascade_documents(&self, value: Option<Rc<dyn ICommand>>) {
        *self.cascade_documents.borrow_mut() = value;
    }
    /// Gets `TileDocumentsHorizontal`.
    pub fn tile_documents_horizontal(&self) -> Option<Rc<dyn ICommand>> {
        self.tile_documents_horizontal.borrow().clone()
    }
    /// Replaces `TileDocumentsHorizontal` without reactive notifications, matching the source auto-property.
    pub fn set_tile_documents_horizontal(&self, value: Option<Rc<dyn ICommand>>) {
        *self.tile_documents_horizontal.borrow_mut() = value;
    }
    /// Gets `TileDocumentsVertical`.
    pub fn tile_documents_vertical(&self) -> Option<Rc<dyn ICommand>> {
        self.tile_documents_vertical.borrow().clone()
    }
    /// Replaces `TileDocumentsVertical` without reactive notifications, matching the source auto-property.
    pub fn set_tile_documents_vertical(&self, value: Option<Rc<dyn ICommand>>) {
        *self.tile_documents_vertical.borrow_mut() = value;
    }
    /// Gets `RestoreDocuments`.
    pub fn restore_documents(&self) -> Option<Rc<dyn ICommand>> {
        self.restore_documents.borrow().clone()
    }
    /// Replaces `RestoreDocuments` without reactive notifications, matching the source auto-property.
    pub fn set_restore_documents(&self, value: Option<Rc<dyn ICommand>>) {
        *self.restore_documents.borrow_mut() = value;
    }
    /// Gets the placeholder shown when there are no documents.
    pub fn empty_content(&self) -> Option<BoxedValue> {
        self.empty_content.borrow().0.clone()
    }
    /// Sets the placeholder using the same object equality rules as DockableBase.Context.
    pub fn set_empty_content(&self, value: Option<BoxedValue>) {
        self.base.runtime().raise_and_set_if_changed(
            &self.empty_content,
            ContextValue(value),
            "EmptyContent",
        );
    }
    /// Gets the source document factory delegate.
    pub fn document_factory(&self) -> Option<DocumentFactory> {
        self.document_factory.borrow().clone()
    }
    /// Replaces the factory delegate without reactive notifications.
    pub fn set_document_factory(&self, value: Option<DocumentFactory>) {
        *self.document_factory.borrow_mut() = value;
    }
    /// Invokes the enclosing model's virtual AddDocument implementation.
    pub fn add_document(&self, document: Rc<dyn IDockable>) {
        self.base
            .runtime()
            .as_document_dock()
            .expect("document dock membership")
            .add_document(Some(document));
    }
    /// Invokes the enclosing model's virtual AddTool implementation.
    pub fn add_tool(&self, tool: Rc<dyn IDockable>) {
        self.base
            .runtime()
            .as_document_dock()
            .expect("document dock membership")
            .add_tool(Some(tool));
    }
    /// Invokes the original AddDocument virtual slot with a possibly null reference.
    pub fn add_document_nullable(&self, document: Option<Rc<dyn IDockable>>) {
        self.base
            .runtime()
            .as_document_dock()
            .expect("document dock membership")
            .add_document(document);
    }
    /// Invokes the original AddTool virtual slot with a possibly null reference.
    pub fn add_tool_nullable(&self, tool: Option<Rc<dyn IDockable>>) {
        self.base
            .runtime()
            .as_document_dock()
            .expect("document dock membership")
            .add_tool(tool);
    }
    /// Implements the base AddDocument behavior for nonnull source calls.
    pub fn add_document_core(&self, document: Rc<dyn IDockable>) {
        self.add_dockable_core(Some(document));
    }
    /// Implements the base AddTool behavior for nonnull source calls.
    pub fn add_tool_core(&self, tool: Rc<dyn IDockable>) {
        self.add_dockable_core(Some(tool));
    }
    /// Implements the original base AddDocument body, preserving a null argument.
    pub fn add_document_nullable_core(&self, document: Option<Rc<dyn IDockable>>) {
        self.add_dockable_core(document);
    }
    /// Implements the original base AddTool body, preserving a null argument.
    pub fn add_tool_nullable_core(&self, tool: Option<Rc<dyn IDockable>>) {
        self.add_dockable_core(tool);
    }
    fn add_dockable_core(&self, dockable: Option<Rc<dyn IDockable>>) {
        let runtime = self.base.runtime();
        if let Some(factory) = runtime.factory() {
            factory.add_dockable(
                Some(runtime.clone().as_dock().expect("dock membership")),
                dockable.clone(),
            );
        }
        if let Some(factory) = runtime.factory() {
            factory.set_active_dockable(dockable.clone());
        }
        if let Some(factory) = runtime.factory() {
            factory
                .set_focused_dockable(Some(runtime.as_dock().expect("dock membership")), dockable);
        }
    }
    /// Boxes this container as the canonical erased dockable handle.
    pub fn boxed(self: &Rc<Self>) -> BoxedValue {
        Rc::new(self.clone() as Rc<dyn IDockable>)
    }
}
impl IDocumentDockFactory for DocumentDock {
    fn document_factory(&self) -> Option<DocumentFactory> {
        self.document_factory()
    }
    fn set_document_factory(&self, value: Option<DocumentFactory>) {
        self.set_document_factory(value);
    }
}
impl IDocumentDock for DocumentDock {
    fn can_create_document(&self) -> bool {
        self.can_create_document()
    }
    fn set_can_create_document(&self, value: bool) {
        self.set_can_create_document(value);
    }
    fn enable_window_drag(&self) -> bool {
        self.enable_window_drag()
    }
    fn set_enable_window_drag(&self, value: bool) {
        self.set_enable_window_drag(value);
    }
    fn tabs_layout(&self) -> DocumentTabLayout {
        self.tabs_layout()
    }
    fn set_tabs_layout(&self, value: DocumentTabLayout) {
        self.set_tabs_layout(value);
    }
    fn layout_mode(&self) -> DocumentLayoutMode {
        self.layout_mode()
    }
    fn set_layout_mode(&self, value: DocumentLayoutMode) {
        self.set_layout_mode(value);
    }
    fn close_button_show_mode(&self) -> DocumentCloseButtonShowMode {
        self.close_button_show_mode()
    }
    fn set_close_button_show_mode(&self, value: DocumentCloseButtonShowMode) {
        self.set_close_button_show_mode(value);
    }
    fn create_document(&self) -> Option<Rc<dyn ICommand>> {
        self.create_document()
    }
    fn set_create_document(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_create_document(value);
    }
    fn cascade_documents(&self) -> Option<Rc<dyn ICommand>> {
        self.cascade_documents()
    }
    fn set_cascade_documents(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_cascade_documents(value);
    }
    fn tile_documents_horizontal(&self) -> Option<Rc<dyn ICommand>> {
        self.tile_documents_horizontal()
    }
    fn set_tile_documents_horizontal(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_tile_documents_horizontal(value);
    }
    fn tile_documents_vertical(&self) -> Option<Rc<dyn ICommand>> {
        self.tile_documents_vertical()
    }
    fn set_tile_documents_vertical(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_tile_documents_vertical(value);
    }
    fn restore_documents(&self) -> Option<Rc<dyn ICommand>> {
        self.restore_documents()
    }
    fn set_restore_documents(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_restore_documents(value);
    }
    fn empty_content(&self) -> Option<BoxedValue> {
        self.empty_content()
    }
    fn set_empty_content(&self, value: Option<BoxedValue>) {
        self.set_empty_content(value);
    }
    fn add_document(&self, document: Option<Rc<dyn IDockable>>) {
        self.add_document_nullable_core(document);
    }
    fn add_tool(&self, tool: Option<Rc<dyn IDockable>>) {
        self.add_tool_nullable_core(tool);
    }
}

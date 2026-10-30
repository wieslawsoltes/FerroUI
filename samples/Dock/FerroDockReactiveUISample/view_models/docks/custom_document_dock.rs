// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::super::documents::DocumentViewModel;
use ferrodock_model::{controls::*, core::*, BoxedValue};
use ferrodock_model_reactiveui::{
    controls::{DocumentDock, DocumentFactory},
    core::DockableBaseImpl,
    impl_dock,
};
use ferroui_reactiveui::{reactive_object, ICommand, ReactiveCommand};
use std::rc::Rc;
/// Original document dock with the sample's document creation command.
pub struct CustomDocumentDock {
    base: DocumentDock,
}
reactive_object!(CustomDocumentDock, base);
impl DockableBaseImpl for CustomDocumentDock {}
impl ILocalTarget for CustomDocumentDock {}
impl_dock!(@base CustomDocumentDock,DocumentDock,fn as_document_dock(self:Rc<Self>)->Option<Rc<dyn IDocumentDock>>{Some(self)},fn as_document_dock_factory(self:Rc<Self>)->Option<Rc<dyn IDocumentDockFactory>>{Some(self)});
impl IDocumentDock for CustomDocumentDock {
    fn can_create_document(&self) -> bool {
        self.base.can_create_document()
    }
    fn set_can_create_document(&self, value: bool) {
        self.base.set_can_create_document(value);
    }
    fn enable_window_drag(&self) -> bool {
        self.base.enable_window_drag()
    }
    fn set_enable_window_drag(&self, value: bool) {
        self.base.set_enable_window_drag(value);
    }
    fn tabs_layout(&self) -> DocumentTabLayout {
        self.base.tabs_layout()
    }
    fn set_tabs_layout(&self, value: DocumentTabLayout) {
        self.base.set_tabs_layout(value);
    }
    fn layout_mode(&self) -> DocumentLayoutMode {
        self.base.layout_mode()
    }
    fn set_layout_mode(&self, value: DocumentLayoutMode) {
        self.base.set_layout_mode(value);
    }
    fn close_button_show_mode(&self) -> DocumentCloseButtonShowMode {
        self.base.close_button_show_mode()
    }
    fn set_close_button_show_mode(&self, value: DocumentCloseButtonShowMode) {
        self.base.set_close_button_show_mode(value);
    }
    fn create_document(&self) -> Option<Rc<dyn ICommand>> {
        self.base.create_document()
    }
    fn set_create_document(&self, value: Option<Rc<dyn ICommand>>) {
        self.base.set_create_document(value);
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
    fn empty_content(&self) -> Option<BoxedValue> {
        self.base.empty_content()
    }
    fn set_empty_content(&self, value: Option<BoxedValue>) {
        self.base.set_empty_content(value);
    }
    fn add_document(&self, document: Option<Rc<dyn IDockable>>) {
        self.base.add_document_nullable_core(document);
    }
    fn add_tool(&self, tool: Option<Rc<dyn IDockable>>) {
        self.base.add_tool_nullable_core(tool);
    }
}
impl IDocumentDockFactory for CustomDocumentDock {
    fn document_factory(&self) -> Option<DocumentFactory> {
        self.base.document_factory()
    }
    fn set_document_factory(&self, value: Option<DocumentFactory>) {
        self.base.set_document_factory(value);
    }
}
impl CustomDocumentDock {
    /// Creates the original base and replaces its create-document command.
    pub fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(|runtime| Self {
            base: DocumentDock::construct(runtime),
        });
        model.base.initialize(&model);
        let owner = model.clone();
        model.set_create_document(Some(Rc::new(ReactiveCommand::create(move || {
            owner.create_new_document()
        }))));
        model
    }
    fn create_new_document(self: &Rc<Self>) {
        if !self.can_create_document() {
            return;
        }
        let index = self
            .visible_dockables()
            .map(|items| (items.count() + 1).to_string())
            .unwrap_or_default();
        let document = DocumentViewModel::new();
        document.set_id(format!("Document{index}"));
        document.set_title(format!("Document{index}"));
        if let Some(factory) = self.factory() {
            factory.add_dockable(Some(self.clone()), Some(document.clone()));
        }
        if let Some(factory) = self.factory() {
            factory.set_active_dockable(Some(document.clone()));
        }
        if let Some(factory) = self.factory() {
            factory.set_focused_dockable(Some(self.clone()), Some(document));
        }
    }
}

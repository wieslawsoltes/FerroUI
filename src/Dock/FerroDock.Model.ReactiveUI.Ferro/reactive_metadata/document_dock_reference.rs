use ferrodock_model::{
    controls::{IDocumentDock, IDocumentDockFactory},
    core::{DocumentCloseButtonShowMode, DocumentLayoutMode, DocumentTabLayout, IDockable},
};
// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockBaseReference;
use ferrodock_model_ferro::DocumentFactoryReference;
use ferroui_base::BoxedValue;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_reactiveui::{ICommand, IReactiveObject};
use ferroui_system::{
    component_model::{INotifyPropertyChanged, INotifyPropertyChanging},
    events::Event,
};
use std::rc::Rc;
/// Native reference to the original DocumentDock class, retaining the enclosing object.
#[derive(Clone)]
pub struct DocumentDockReference {
    model: Rc<dyn IDocumentDock>,
    factory_contract: Rc<dyn IDocumentDockFactory>,
    base: DockBaseReference,
}
impl DocumentDockReference {
    /// Constructs the source class and exposes the same original engine instance.
    pub fn create() -> Self {
        Self::new(ferrodock_model_reactiveui::controls::DocumentDock::new())
    }

    /// Creates the source base-class view without copying or replacing its model.
    pub fn new<T: IDocumentDock + IDocumentDockFactory + IReactiveObject>(model: Rc<T>) -> Self {
        Self {
            base: super::DockBaseReference::new(model.clone()),
            factory_contract: model.clone(),
            model,
        }
    }
    /// Gets the actual model behind this class reference.
    pub fn model(&self) -> Rc<dyn IDocumentDock> {
        self.model.clone()
    }
    /// Gets the original document-factory interface of the same enclosing class.
    pub fn factory_contract(&self) -> Rc<dyn IDocumentDockFactory> {
        self.factory_contract.clone()
    }
    /// Gets the source base-class reference to the same enclosing model.
    pub fn base(&self) -> DockBaseReference {
        self.base.clone()
    }
}
impl PartialEq for DocumentDockReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
impl INotifyPropertyChanged for DocumentDockReference {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
    fn as_notify_property_changing(&self) -> Option<&dyn INotifyPropertyChanging> {
        Some(self)
    }
}
impl INotifyPropertyChanging for DocumentDockReference {
    fn property_changing(&self) -> &Event<str> {
        self.base.property_changing()
    }
}
ferroui_base::ferro_markup_type!(class DocumentDockReference as "DocumentDock"{namespace:"FerroDock.Model.ReactiveUI.Controls",this:DocumentDockReference,handles:[DocumentDockReference,Option<DocumentDockReference>],base:DockBaseReference,interfaces:[Rc<dyn IDocumentDock>,Rc<dyn IDocumentDockFactory>],constructors:[()=>DocumentDockReference::create],notify_property_changed:DocumentDockReference,properties:[
        CanCreateDocument:bool{get:|this:&DocumentDockReference|this.model.can_create_document(),try_set:|this:&DocumentDockReference,value:bool|{this.model.set_can_create_document(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        CreateDocument:Option<Rc<dyn ICommand>>{get:|this:&DocumentDockReference|this.model.create_document(),try_set:|this:&DocumentDockReference,value:Option<Rc<dyn ICommand>>|{this.model.set_create_document(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}} [IgnoreDataMember],
        CascadeDocuments:Option<Rc<dyn ICommand>>{get:|this:&DocumentDockReference|this.model.cascade_documents(),try_set:|this:&DocumentDockReference,value:Option<Rc<dyn ICommand>>|{this.model.set_cascade_documents(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}} [IgnoreDataMember],
        TileDocumentsHorizontal:Option<Rc<dyn ICommand>>{get:|this:&DocumentDockReference|this.model.tile_documents_horizontal(),try_set:|this:&DocumentDockReference,value:Option<Rc<dyn ICommand>>|{this.model.set_tile_documents_horizontal(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}} [IgnoreDataMember],
        TileDocumentsVertical:Option<Rc<dyn ICommand>>{get:|this:&DocumentDockReference|this.model.tile_documents_vertical(),try_set:|this:&DocumentDockReference,value:Option<Rc<dyn ICommand>>|{this.model.set_tile_documents_vertical(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}} [IgnoreDataMember],
        RestoreDocuments:Option<Rc<dyn ICommand>>{get:|this:&DocumentDockReference|this.model.restore_documents(),try_set:|this:&DocumentDockReference,value:Option<Rc<dyn ICommand>>|{this.model.set_restore_documents(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}} [IgnoreDataMember],
        DocumentFactory:Option<DocumentFactoryReference>{get:|this:&DocumentDockReference|this.factory_contract.document_factory().map(DocumentFactoryReference::new),set:|this:&DocumentDockReference,value:Option<DocumentFactoryReference>|this.factory_contract.set_document_factory(value.map(|v|v.model()))} [IgnoreDataMember],
        EnableWindowDrag:bool{get:|this:&DocumentDockReference|this.model.enable_window_drag(),try_set:|this:&DocumentDockReference,value:bool|{this.model.set_enable_window_drag(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        TabsLayout:DocumentTabLayout{get:|this:&DocumentDockReference|this.model.tabs_layout(),try_set:|this:&DocumentDockReference,value:DocumentTabLayout|{this.model.set_tabs_layout(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        LayoutMode:DocumentLayoutMode{get:|this:&DocumentDockReference|this.model.layout_mode(),try_set:|this:&DocumentDockReference,value:DocumentLayoutMode|{this.model.set_layout_mode(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        CloseButtonShowMode:DocumentCloseButtonShowMode{get:|this:&DocumentDockReference|this.model.close_button_show_mode(),try_set:|this:&DocumentDockReference,value:DocumentCloseButtonShowMode|{this.model.set_close_button_show_mode(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        EmptyContent:Option<BoxedValue>{get:|this:&DocumentDockReference|this.model.empty_content().as_ref().map(ferrodock_controls_recycling::from_recycling_value),try_set:|this:&DocumentDockReference,value:Option<BoxedValue>|{this.model.set_empty_content(value.and_then(ferroui_base::data::core::ValueTypes::normalize).map(ferrodock_controls_recycling::to_recycling_value));Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}} [IgnoreDataMember],
    ],methods:[
        fn AddDocument(Option<Rc<dyn IDockable>>) => (|this:&DocumentDockReference,value|this.model.add_document(value)),
        fn AddTool(Option<Rc<dyn IDockable>>) => (|this:&DocumentDockReference,value|this.model.add_tool(value)),
    ],});
pub(crate) fn register() {
    ValueTypes::register_cast::<DocumentDockReference, Rc<dyn IDocumentDockFactory>>(|value| {
        value.factory_contract()
    });
    ValueTypes::register_cast::<DocumentDockReference, super::DockableBaseReference>(|value| {
        value.base().base()
    });
    ValueTypes::register_cast::<DocumentDockReference, super::ReactiveBaseReference>(|value| {
        value.base().base().base()
    });
    ValueTypes::register_cast::<DocumentDockReference, super::ReactiveObjectReference>(|value| {
        value.base().base().base().base()
    });
    ValueTypes::register_cast::<DocumentDockReference, Rc<dyn IDockable>>(|value| value.model());

    MarkupType::register(<DocumentDockReference as MarkupTyped>::MARKUP);
    ValueTypes::register_nullable::<DocumentDockReference>();
    ValueTypes::register_cast::<DocumentDockReference, DockBaseReference>(|value| value.base());
    ValueTypes::register_cast::<DocumentDockReference, Rc<dyn IDocumentDock>>(|value| {
        value.model()
    });
}

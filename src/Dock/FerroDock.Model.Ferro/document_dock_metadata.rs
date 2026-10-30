// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::{
    controls::IDocumentDock,
    core::{DocumentCloseButtonShowMode, DocumentLayoutMode, DocumentTabLayout, IDock, IDockable},
};
use ferroui_base::{
    metadata::{MarkupType, MarkupTyped},
    AnyValue, BoxedValue,
};
use ferroui_reactiveui::ICommand;
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Descriptor carrier; values retain the original IDocumentDock object.
#[doc(hidden)]
pub trait IDocumentDockMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IDocumentDockMetadata as "IDocumentDock" {
    namespace:"FerroDock.Model.Controls",this:Rc<dyn IDocumentDock>,handles:[Rc<dyn IDocumentDock>,Option<Rc<dyn IDocumentDock>>],base:Rc<dyn IDock>,interfaces:[Rc<dyn IDock>,Rc<dyn ferrodock_model::core::ILocalTarget>],
    attributes:[RequiresDataTemplate],
    properties:[
        CanCreateDocument:bool{get:|this:&Rc<dyn IDocumentDock>|this.can_create_document(),try_set:|this:&Rc<dyn IDocumentDock>,value:bool|{this.set_can_create_document(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        CreateDocument:Option<Rc<dyn ICommand>>{get:|this:&Rc<dyn IDocumentDock>|this.create_document(),try_set:|this:&Rc<dyn IDocumentDock>,value:Option<Rc<dyn ICommand>>|{this.set_create_document(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        EnableWindowDrag:bool{get:|this:&Rc<dyn IDocumentDock>|this.enable_window_drag(),try_set:|this:&Rc<dyn IDocumentDock>,value:bool|{this.set_enable_window_drag(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        LayoutMode:DocumentLayoutMode{get:|this:&Rc<dyn IDocumentDock>|this.layout_mode(),try_set:|this:&Rc<dyn IDocumentDock>,value:DocumentLayoutMode|{this.set_layout_mode(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        TabsLayout:DocumentTabLayout{get:|this:&Rc<dyn IDocumentDock>|this.tabs_layout(),try_set:|this:&Rc<dyn IDocumentDock>,value:DocumentTabLayout|{this.set_tabs_layout(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        CloseButtonShowMode:DocumentCloseButtonShowMode{get:|this:&Rc<dyn IDocumentDock>|this.close_button_show_mode(),try_set:|this:&Rc<dyn IDocumentDock>,value:DocumentCloseButtonShowMode|{this.set_close_button_show_mode(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        EmptyContent:Option<BoxedValue>{get:|this:&Rc<dyn IDocumentDock>|this.empty_content().as_ref().map(ferrodock_runtime::from_source_value),try_set:|this:&Rc<dyn IDocumentDock>,value:Option<BoxedValue>|{this.set_empty_content(value.and_then(ferroui_base::data::core::ValueTypes::normalize).map(ferrodock_runtime::to_source_value));Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        CascadeDocuments:Option<Rc<dyn ICommand>>{get:|this:&Rc<dyn IDocumentDock>|this.cascade_documents(),try_set:|this:&Rc<dyn IDocumentDock>,value:Option<Rc<dyn ICommand>>|{this.set_cascade_documents(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        TileDocumentsHorizontal:Option<Rc<dyn ICommand>>{get:|this:&Rc<dyn IDocumentDock>|this.tile_documents_horizontal(),try_set:|this:&Rc<dyn IDocumentDock>,value:Option<Rc<dyn ICommand>>|{this.set_tile_documents_horizontal(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        TileDocumentsVertical:Option<Rc<dyn ICommand>>{get:|this:&Rc<dyn IDocumentDock>|this.tile_documents_vertical(),try_set:|this:&Rc<dyn IDocumentDock>,value:Option<Rc<dyn ICommand>>|{this.set_tile_documents_vertical(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
        RestoreDocuments:Option<Rc<dyn ICommand>>{get:|this:&Rc<dyn IDocumentDock>|this.restore_documents(),try_set:|this:&Rc<dyn IDocumentDock>,value:Option<Rc<dyn ICommand>>|{this.set_restore_documents(value);Ok::<(),ferroui_base::metadata::MarkupInvokeError>(())}},
    ],
    methods:[
        fn AddDocument(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IDocumentDock>,value|this.add_document(value)),
        fn AddTool(Option<Rc<dyn IDockable>>) => (|this:&Rc<dyn IDocumentDock>,value|this.add_tool(value)),
    ],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IDocumentDockMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IDocumentDock>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IDocumentDock>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    MarkupType::register(&METADATA);
    // The notifier descriptor is the registered identity; emitted members still
    // belong to the original public declaration carrier.
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA, "::ferrodock_model_ferro::IDocumentDockMetadata", true)]);
    }
}

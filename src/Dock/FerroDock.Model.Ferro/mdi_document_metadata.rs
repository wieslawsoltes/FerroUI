// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Core-owned metadata for the original MDI document interface and its optional notifier.
use ferrodock_model::{
    controls::IMdiDocument,
    core::{DockRect, IDockable, MdiWindowState},
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
    AnyValue,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
/// Descriptor carrier; values retain the original model interface.
#[doc(hidden)]
pub trait IMdiDocumentMetadata {}
ferroui_base::ferro_markup_type!(interface dyn IMdiDocumentMetadata as "IMdiDocument" {
    namespace:"FerroDock.Model.Controls",this:Rc<dyn IMdiDocument>,handles:[Rc<dyn IMdiDocument>,Option<Rc<dyn IMdiDocument>>],
    base:Rc<dyn ferrodock_model::controls::IDocument>,interfaces:[Rc<dyn ferrodock_model::controls::IDocument>],
    properties:[
        MdiBounds:DockRect{get:|this:&Rc<dyn IMdiDocument>|this.mdi_bounds(),set:|this:&Rc<dyn IMdiDocument>,value|this.set_mdi_bounds(value)},
        MdiState:MdiWindowState{get:|this:&Rc<dyn IMdiDocument>|this.mdi_state(),set:|this:&Rc<dyn IMdiDocument>,value|this.set_mdi_state(value)},
        MdiZIndex:i32{get:|this:&Rc<dyn IMdiDocument>|this.mdi_z_index(),set:|this:&Rc<dyn IMdiDocument>,value|this.set_mdi_z_index(value)},
    ],
});
static METADATA: MarkupType = MarkupType {
    notify_property_changed: Some(notifier),
    ..*<dyn IMdiDocumentMetadata as MarkupTyped>::MARKUP
};
fn notifier(value: &dyn AnyValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(model) = value.downcast_ref::<Rc<dyn IMdiDocument>>() {
        return model.as_notify_property_changed();
    }
    value
        .downcast_ref::<Option<Rc<dyn IMdiDocument>>>()?
        .as_ref()?
        .as_notify_property_changed()
}
pub(crate) fn register() {
    MarkupType::register(&METADATA);
    ValueTypes::register_nullable::<Rc<dyn IMdiDocument>>();
    ValueTypes::register_cast::<Rc<dyn IMdiDocument>, Rc<dyn ferrodock_model::controls::IDocument>>(
        |model| model.clone(),
    );
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[(&METADATA,"::ferrodock_model_ferro::IMdiDocumentMetadata",true)]);
    }
    ValueTypes::register_cast::<Rc<dyn IMdiDocument>, Rc<dyn IDockable>>(|model| model.clone());
}
